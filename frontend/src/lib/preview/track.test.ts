import { describe, expect, it } from 'vitest';
import { existsSync, readFileSync } from 'node:fs';
import { parsePreviewTrack, trackMatches } from './track';
import { buildOverlay, buildLegend, buildPreviewGeometry } from './geometry';
import { rangeColor, rangeValues, sampleRange, OPTION_COLORS } from './colorations';
import type { PreviewSegments } from './decode';

const GCODE = [
	'M83',
	'M104 S215',
	';LAYER_CHANGE',
	';Z:0.2',
	'M106 S0',
	'SET_VELOCITY_LIMIT ACCEL=1000 SQUARE_CORNER_VELOCITY=5',
	'G1 X0 Y0 Z0.2 F6000',
	';TYPE:Outer wall',
	'G1 X10 Y0 E1 F1200',
	'G1 X10 Y10 E1',
	'G1 E-0.8 F2100',
	'G1 X20 Y10 F6000',
	'G1 E0.8 F2100',
	';TYPE:Sparse infill',
	'G1 X30 Y10 E1 F3000',
	';LAYER_CHANGE',
	';Z:0.4',
	'M106 S255',
	'M104 S225',
	'G1 Z0.4 F600',
	';TYPE:Outer wall',
	'G1 X40 Y10 E1 F1200',
	';WIPE_START',
	'G1 X41 Y10 E-0.2',
	';WIPE_END',
	'T1',
	'M600',
	'G1 X50 Y10 E1'
].join('\n');

describe('parsePreviewTrack', () => {
	const tr = parsePreviewTrack(GCODE);

	it('mêmes couches et segments que parse_preview (réamorçage compté comme extrusion)', () => {
		expect(tr.layers.map((l) => l.segments)).toEqual([4, 2]);
		expect(trackMatches(tr, 2)).toBe(true);
		expect(trackMatches(tr, 3)).toBe(false);
	});

	it('valeurs par segment : ventilateur, température par extrudeur, accélération, jerk', () => {
		expect([...tr.layers[0].values.fan]).toEqual([0, 0, 0, 0]);
		expect([...tr.layers[1].values.fan]).toEqual([100, 100]);
		expect(tr.layers[0].values.temperature[0]).toBe(215);
		expect(tr.layers[1].values.temperature[0]).toBe(225);
		// Après T1 sans M104 T1 : pas de consigne connue pour l'extrudeur 1.
		expect(tr.layers[1].values.temperature[1]).toBe(0);
		expect(tr.layers[0].values.acceleration[0]).toBe(1000);
		expect(tr.layers[0].values.jerk[0]).toBe(5);
		expect(tr.present.pa).toBe(false);
	});

	it('marqueurs : rétraction, réamorçage, couture, essuyage, outil, couleur', () => {
		const n = (l: number, k: keyof (typeof tr.layers)[0]['markers']) =>
			tr.layers[l].markers[k].length / 3;
		expect(n(0, 'retract')).toBe(1);
		expect(n(0, 'unretract')).toBe(1);
		expect(n(0, 'seam')).toBe(1);
		// Reprise de paroi externe après l'essuyage = nouveau départ de boucle.
		expect(n(1, 'seam')).toBe(2);
		expect(n(1, 'retract')).toBe(0); // rétraction roulante = essuyage
		expect(tr.layers[1].wipes.length / 6).toBe(1);
		expect(n(1, 'toolchange')).toBe(1);
		expect(n(1, 'colorchange')).toBe(1);
		expect(tr.layers[0].travels.length / 6).toBeGreaterThanOrEqual(1);
	});

	it('temps de couche nominal (distance / vitesse)', () => {
		// Couche 0 : 10 mm + 10 mm à 1200 mm/min = 1 s, rétraction 0.8 mm à 2100…
		expect(tr.layers[0].time).toBeGreaterThan(1);
		expect(tr.ranges.layerTime[1]).toBeGreaterThanOrEqual(tr.ranges.layerTime[0]);
	});

	const golden = new URL(
		'../../../../tests/oracle/refs/golden/overhang.0-28mm-extra-draft-voron.gcode',
		import.meta.url
	);
	it.skipIf(!existsSync(golden))(
		'G-code Orca réel : coutures ≈ couches, rétractions ≈ réamorçages',
		() => {
			const t = parsePreviewTrack(readFileSync(golden, 'utf8'));
			const sum = (k: 'seam' | 'retract' | 'unretract') =>
				t.layers.reduce((a, l) => a + l.markers[k].length / 3, 0);
			expect(t.layers.length).toBe(72); // = layer_count du backend (vérifié sur la stack)
			expect(sum('seam')).toBe(72);
			expect(Math.abs(sum('retract') - sum('unretract'))).toBeLessThanOrEqual(1);
			expect(t.present.fan && t.present.acceleration).toBe(true);
		}
	);
});

describe('échelles Orca (ColorRange)', () => {
	it('logarithmique : milieu géométrique au centre', () => {
		expect(rangeColor(10, 1, 100, true)).toEqual(sampleRange(0.5));
		expect(rangeColor(50.5, 1, 100)).toEqual(sampleRange(0.5));
	});
	it('11 valeurs (une par couleur), 1 si plate', () => {
		expect(rangeValues(0, 10)).toHaveLength(11);
		expect(rangeValues(5, 5)).toEqual([5]);
		const log = rangeValues(1, 100, true);
		expect(log[5]).toBeCloseTo(10, 6);
	});
});

describe('géométrie avec piste', () => {
	const tr = parsePreviewTrack(GCODE);
	const segs: PreviewSegments = {
		from: 0,
		to: 1,
		count: 6,
		start: new Float32Array(18),
		end: new Float32Array(18).fill(1),
		feedrate: new Float32Array(6).fill(1200),
		width: new Float32Array(6).fill(0.4),
		height: new Float32Array(6).fill(0.2),
		kind: new Uint8Array(6).fill(2),
		extruder: new Uint8Array(6),
		layer: Uint16Array.from([0, 0, 0, 0, 1, 1])
	};
	const ranges = {
		feedrate: [1200, 1200] as [number, number],
		width: [0.4, 0.4] as [number, number],
		height: [0.2, 0.2] as [number, number]
	};

	it('ventilateur : couleurs extrêmes de la palette par couche', () => {
		const g = buildPreviewGeometry(segs, { coloration: 'fan', ranges, track: tr });
		expect([...g.colors.slice(0, 3)]).toEqual(sampleRange(0).map(Math.fround));
		expect([...g.colors.slice(4 * 6, 4 * 6 + 3)]).toEqual(sampleRange(1).map(Math.fround));
	});

	it('sans piste : gris « aucune donnée », légende explicite', () => {
		const g = buildPreviewGeometry(segs, { coloration: 'fan', ranges });
		expect(g.colors[0]).toBeCloseTo(120 / 255, 5);
		const legend = buildLegend('pa', { ranges, typesPresent: [], extrudersPresent: [], track: tr });
		expect(legend.kind).toBe('list');
	});

	it('superposition : déplacements et marqueurs aux couleurs Orca', () => {
		const o = buildOverlay(tr, { low: 0, high: 0 }, new Set(['travel', 'retract']));
		expect(o.lines.visibleCount).toBe(tr.layers[0].travels.length / 6);
		expect(o.points.visibleCount).toBe(1);
		expect([...o.points.colors.slice(0, 3)]).toEqual(OPTION_COLORS.retract.map(Math.fround));
	});
});
