// Tests des helpers d'assemblage de l'aperçu (T088) : fusion de la fenêtre de
// tranches en une géométrie, bornes de coloration, corps de requête de tranchage.
import { describe, expect, it } from 'bun:test';
import type { PreviewMeta } from '../api/types';
import type { PreviewSegments } from '../preview/decode';
import { buildWindowGeometry, rangesFromMeta, sliceRequestFor } from './preview';

/** Une tranche à `n` segments linéaires triviaux (rôle 0, couche 0). */
function segments(n: number): PreviewSegments {
	return {
		from: 0,
		to: 0,
		count: n,
		start: new Float32Array(n * 3),
		end: new Float32Array(n * 3).fill(1),
		feedrate: new Float32Array(n).fill(60),
		width: new Float32Array(n).fill(0.4),
		height: new Float32Array(n).fill(0.2),
		kind: new Uint8Array(n),
		extruder: new Uint8Array(n),
		layer: new Uint16Array(n)
	};
}

const meta: PreviewMeta = {
	layer_count: 3,
	layers: [],
	types_present: [],
	extruders_present: [0],
	feedrate_min: 10,
	feedrate_max: 120,
	width_min: 0.3,
	width_max: 0.6,
	height_min: 0.1,
	height_max: 0.3,
	segment_record_bytes: 40
};

const options = { coloration: 'type' as const, ranges: rangesFromMeta(meta) };

describe('preview assembly', () => {
	it('fusionne la géométrie de plusieurs tranches (2 verts + rgb par segment)', () => {
		const geo = buildWindowGeometry([segments(2), segments(3)], options);
		expect(geo.visibleCount).toBe(5);
		expect(geo.positions.length).toBe(5 * 2 * 3);
		expect(geo.colors.length).toBe(5 * 2 * 3);
	});

	it('fenêtre vide → géométrie vide', () => {
		const geo = buildWindowGeometry([], options);
		expect(geo.visibleCount).toBe(0);
		expect(geo.positions.length).toBe(0);
	});

	it('dérive les bornes de coloration de la méta', () => {
		expect(rangesFromMeta(meta).feedrate).toEqual([10, 120]);
		expect(rangesFromMeta(meta).width).toEqual([0.3, 0.6]);
	});

	it('construit le corps de requête de tranchage', () => {
		expect(sliceRequestFor('all', 2)).toEqual({ all: true });
		expect(sliceRequestFor('plate', 2)).toEqual({ plate_index: 2n });
		expect(sliceRequestFor('plate', -5)).toEqual({ plate_index: 0n });
	});
});
