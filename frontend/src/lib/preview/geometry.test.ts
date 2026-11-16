// Tests de la construction de géométrie (T068) : positions/couleurs par sommet,
// filtrage par visibilité de type, colorations et légendes.
import { describe, expect, test } from 'vitest';
import { decodePreview } from './decode';
import { encodePreview } from './decode.test';
import {
	buildLegend,
	buildPreviewGeometry,
	computeFlowRange,
	flowValue,
	type GeometryOptions
} from './geometry';
import { SEGMENT_ROLE_COLORS } from './colorations';

const seg = (kind: number, extruder: number, feedrate: number, layer: number) => ({
	start: [0, 0, 0.2] as [number, number, number],
	end: [1, 0, 0.2] as [number, number, number],
	feedrate,
	width: 0.45,
	height: 0.2,
	kind,
	extruder,
	layer
});

// Trois segments : outer wall (2), sparse infill (4), outer wall (2).
const SEGMENTS = decodePreview(
	encodePreview(0, 0, [seg(2, 0, 1800, 0), seg(4, 0, 3000, 0), seg(2, 1, 2400, 0)])
);

const RANGES = {
	feedrate: [1800, 3000] as [number, number],
	width: [0.4, 0.5] as [number, number],
	height: [0.2, 0.3] as [number, number]
};

describe('buildPreviewGeometry', () => {
	test('curseur horizontal : limite les moves de la couche haute', () => {
		const g = buildPreviewGeometry(SEGMENTS, {
			coloration: 'type',
			ranges: RANGES,
			moveCursor: { layer: 0, position: 2 }
		});
		expect(g.visibleCount).toBe(2);
		expect(g.positions.length).toBe(12);
	});

	test('emits two vertices per segment coloured by role in type mode', () => {
		const opts: GeometryOptions = { coloration: 'type', ranges: RANGES };
		const g = buildPreviewGeometry(SEGMENTS, opts);
		expect(g.visibleCount).toBe(3);
		expect(g.positions.length).toBe(3 * 6);
		expect(g.colors.length).toBe(3 * 6);
		// Premier segment (outer wall) coloré selon la palette de rôles.
		const [r, gg, b] = SEGMENT_ROLE_COLORS[2];
		expect(g.colors[0]).toBeCloseTo(r);
		expect(g.colors[1]).toBeCloseTo(gg);
		expect(g.colors[2]).toBeCloseTo(b);
		// Couleur dupliquée sur les deux sommets.
		expect(g.colors[3]).toBeCloseTo(r);
	});

	test('hides segments whose type is not visible', () => {
		const opts: GeometryOptions = {
			coloration: 'type',
			ranges: RANGES,
			visibleTypes: new Set([2]) // seulement outer wall
		};
		const g = buildPreviewGeometry(SEGMENTS, opts);
		expect(g.visibleCount).toBe(2);
		expect(g.positions.length).toBe(2 * 6);
	});

	test('speed mode maps fastest segment to the hot end of the gradient', () => {
		const g = buildPreviewGeometry(SEGMENTS, { coloration: 'speed', ranges: RANGES });
		// Segment le plus lent (1800 = min) → bleu (composante rouge faible),
		// le plus rapide (3000 = max) → rouge (composante rouge forte).
		const slowR = g.colors[0];
		const fastR = g.colors[6]; // 2e segment, feedrate 3000
		expect(fastR).toBeGreaterThan(slowR);
	});

	test('temperature mode falls back to the no-data colour', () => {
		const g = buildPreviewGeometry(SEGMENTS, { coloration: 'temperature', ranges: RANGES });
		// Gris neutre identique pour tous les sommets.
		expect(g.colors[0]).toBeCloseTo(g.colors[6]);
		expect(g.colors[0]).toBeCloseTo(120 / 255);
	});
});

describe('buildPreviewGeometry layer range', () => {
	// Couches 0..4, deux segments par couche (outer wall + sparse infill),
	// z distinct par couche pour identifier les sommets émis.
	const layered = (() => {
		const segs = [];
		for (let layer = 0; layer < 5; layer++) {
			for (const kind of [2, 4]) {
				const z = 0.2 * (layer + 1);
				segs.push({ ...seg(kind, 0, 1800, layer), start: [0, 0, z], end: [1, 0, z] });
			}
		}
		return decodePreview(encodePreview(0, 4, segs as ReturnType<typeof seg>[]));
	})();
	const zs = (g: { positions: Float32Array; visibleCount: number }) => {
		const out = new Set<number>();
		for (let i = 0; i < g.visibleCount; i++) out.add(Math.round(g.positions[i * 6 + 2] * 10));
		return [...out].sort((a, b) => a - b);
	};

	test('without layerRange every layer is kept (backward compatible)', () => {
		const g = buildPreviewGeometry(layered, { coloration: 'type', ranges: RANGES });
		expect(g.visibleCount).toBe(10);
	});

	test('excludes segments above high', () => {
		const g = buildPreviewGeometry(layered, {
			coloration: 'type',
			ranges: RANGES,
			layerRange: { low: 0, high: 2 }
		});
		expect(g.visibleCount).toBe(6);
		expect(g.positions.length).toBe(6 * 6);
		expect(zs(g)).toEqual([2, 4, 6]); // couches 0,1,2 ; 3 et 4 exclues
	});

	test('excludes segments below low', () => {
		const g = buildPreviewGeometry(layered, {
			coloration: 'type',
			ranges: RANGES,
			layerRange: { low: 3, high: 4 }
		});
		expect(g.visibleCount).toBe(4);
		expect(zs(g)).toEqual([8, 10]);
	});

	test('one-layer mode (low === high) keeps exactly that layer', () => {
		const g = buildPreviewGeometry(layered, {
			coloration: 'type',
			ranges: RANGES,
			layerRange: { low: 2, high: 2 }
		});
		expect(g.visibleCount).toBe(2);
		expect(g.colors.length).toBe(2 * 6);
		expect(zs(g)).toEqual([6]);
	});

	test('combines with type visibility and tolerates swapped bounds', () => {
		const g = buildPreviewGeometry(layered, {
			coloration: 'type',
			ranges: RANGES,
			visibleTypes: new Set([4]),
			layerRange: { low: 3, high: 1 }
		});
		expect(g.visibleCount).toBe(3); // infill des couches 1..3
		expect(zs(g)).toEqual([4, 6, 8]);
	});

	test('range outside the buffer yields an empty geometry', () => {
		const g = buildPreviewGeometry(layered, {
			coloration: 'type',
			ranges: RANGES,
			layerRange: { low: 10, high: 12 }
		});
		expect(g.visibleCount).toBe(0);
		expect(g.positions.length).toBe(0);
	});
});

describe('flow', () => {
	test('flowValue is section times speed (mm3/s)', () => {
		expect(flowValue(0.45, 0.2, 1800)).toBeCloseTo(0.45 * 0.2 * 30);
	});

	test('computeFlowRange spans the extruded segments', () => {
		const [min, max] = computeFlowRange(SEGMENTS);
		expect(min).toBeCloseTo(flowValue(0.45, 0.2, 1800));
		expect(max).toBeCloseTo(flowValue(0.45, 0.2, 3000));
	});
});

describe('buildLegend', () => {
	test('type legend lists present roles with their colours', () => {
		const legend = buildLegend('type', {
			ranges: RANGES,
			typesPresent: [
				{ id: 2, name: 'Outer wall' },
				{ id: 4, name: 'Sparse infill' }
			],
			extrudersPresent: [0]
		});
		expect(legend.kind).toBe('list');
		if (legend.kind === 'list') {
			expect(legend.entries.map((e) => e.label)).toEqual(['Outer wall', 'Sparse infill']);
			expect(legend.entries[0].color).toEqual(SEGMENT_ROLE_COLORS[2]);
		}
	});

	test('speed legend is a scale in mm/s with converted bounds', () => {
		const legend = buildLegend('speed', {
			ranges: RANGES,
			typesPresent: [],
			extrudersPresent: []
		});
		expect(legend.kind).toBe('scale');
		if (legend.kind === 'scale') {
			expect(legend.unit).toBe('mm/s');
			expect(legend.min).toBeCloseTo(30); // 1800 mm/min
			expect(legend.max).toBeCloseTo(50); // 3000 mm/min
			expect(legend.stops.length).toBeGreaterThan(1);
		}
	});

	test('filament legend lists present extruders', () => {
		const legend = buildLegend('filament', {
			ranges: RANGES,
			typesPresent: [],
			extrudersPresent: [0, 1]
		});
		expect(legend.kind).toBe('list');
		if (legend.kind === 'list') {
			expect(legend.entries.map((e) => e.label)).toEqual(['Filament 1', 'Filament 2']);
		}
	});
});
