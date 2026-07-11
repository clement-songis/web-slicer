// Tests du décodage des buffers `WSPv` (T068) : round-trip avec un encodeur
// miroir du backend (`PreviewModel::encode_range`), et rejet des buffers mal
// formés.
import { describe, expect, test } from 'bun:test';
import { decodePreview, PreviewFormatError, HEADER_BYTES, RECORD_BYTES } from './decode';

interface Seg {
	start: [number, number, number];
	end: [number, number, number];
	feedrate: number;
	width: number;
	height: number;
	kind: number;
	extruder: number;
	layer: number;
}

/** Encode un buffer `WSPv` à l'identique du backend (little-endian). */
export function encodePreview(from: number, to: number, segments: Seg[]): ArrayBuffer {
	const buf = new ArrayBuffer(HEADER_BYTES + segments.length * RECORD_BYTES);
	const view = new DataView(buf);
	view.setUint8(0, 'W'.charCodeAt(0));
	view.setUint8(1, 'S'.charCodeAt(0));
	view.setUint8(2, 'P'.charCodeAt(0));
	view.setUint8(3, 'v'.charCodeAt(0));
	view.setUint16(4, 1, true);
	view.setUint32(6, from, true);
	view.setUint32(10, to, true);
	view.setUint32(14, segments.length, true);

	let o = HEADER_BYTES;
	for (const s of segments) {
		view.setFloat32(o, s.start[0], true);
		view.setFloat32(o + 4, s.start[1], true);
		view.setFloat32(o + 8, s.start[2], true);
		view.setFloat32(o + 12, s.end[0], true);
		view.setFloat32(o + 16, s.end[1], true);
		view.setFloat32(o + 20, s.end[2], true);
		view.setFloat32(o + 24, s.feedrate, true);
		view.setFloat32(o + 28, s.width, true);
		view.setFloat32(o + 32, s.height, true);
		view.setUint8(o + 36, s.kind);
		view.setUint8(o + 37, s.extruder);
		view.setUint16(o + 38, s.layer, true);
		o += RECORD_BYTES;
	}
	return buf;
}

const SEG_A: Seg = {
	start: [0, 0, 0.2],
	end: [10, 0, 0.2],
	feedrate: 1800,
	width: 0.45,
	height: 0.2,
	kind: 2,
	extruder: 0,
	layer: 0
};
const SEG_B: Seg = {
	start: [10, 0, 0.4],
	end: [10, 10, 0.4],
	feedrate: 3000,
	width: 0.4,
	height: 0.2,
	kind: 4,
	extruder: 1,
	layer: 1
};

describe('decodePreview', () => {
	test('round-trips segments from the WSPv buffer', () => {
		const decoded = decodePreview(encodePreview(0, 1, [SEG_A, SEG_B]));
		expect(decoded.from).toBe(0);
		expect(decoded.to).toBe(1);
		expect(decoded.count).toBe(2);

		// z=0.2 est stocké en f32 : comparaison avec tolérance.
		expect(decoded.start[0]).toBeCloseTo(0);
		expect(decoded.start[1]).toBeCloseTo(0);
		expect(decoded.start[2]).toBeCloseTo(0.2);
		expect(decoded.end[0]).toBeCloseTo(10);
		expect(decoded.feedrate[0]).toBeCloseTo(1800);
		expect(decoded.width[0]).toBeCloseTo(0.45);
		expect(decoded.kind[0]).toBe(2);
		expect(decoded.extruder[1]).toBe(1);
		expect(decoded.layer[1]).toBe(1);
	});

	test('decodes an empty buffer (header only)', () => {
		const decoded = decodePreview(encodePreview(0, 0, []));
		expect(decoded.count).toBe(0);
		expect(decoded.start.length).toBe(0);
	});

	test('rejects a wrong magic', () => {
		const buf = encodePreview(0, 0, []);
		new DataView(buf).setUint8(0, 'X'.charCodeAt(0));
		expect(() => decodePreview(buf)).toThrow(PreviewFormatError);
	});

	test('rejects a truncated buffer', () => {
		expect(() => decodePreview(new ArrayBuffer(4))).toThrow(PreviewFormatError);
		// En-tête annonçant 2 segments mais corps absent.
		const short = encodePreview(0, 0, []).slice(0, HEADER_BYTES);
		new DataView(short).setUint32(14, 2, true);
		expect(() => decodePreview(short)).toThrow(PreviewFormatError);
	});
});
