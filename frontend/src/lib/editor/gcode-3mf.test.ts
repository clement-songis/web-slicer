import { describe, expect, it } from 'vitest';
import { strToU8, zipSync } from 'fflate';
import { ApiError } from '$lib/api/client';
import {
	archivePlates,
	gcode3mfRefusal,
	gcodeFilePlates,
	gcodeFileText,
	is3mfName,
	isZipBytes
} from './gcode-3mf';

function archive(entries: Record<string, string>): Blob {
	const files = Object.fromEntries(Object.entries(entries).map(([k, v]) => [k, strToU8(v)]));
	return new Blob([zipSync(files)]);
}

const SLICED = {
	'3D/3dmodel.model': '<model/>',
	'Metadata/plate_2.gcode': '; plate 2\nG1 X2\n',
	'Metadata/plate_2.gcode.md5': 'x',
	'Metadata/plate_1.gcode': '; plate 1\nG1 X1\n',
	'Metadata/plate_1.png': 'png'
};

describe('.gcode.3mf ouvert comme G-code', () => {
	it('reconnaît une archive ZIP et un nom 3MF', async () => {
		const zip = new Uint8Array(await archive(SLICED).arrayBuffer());
		expect(isZipBytes(zip)).toBe(true);
		expect(isZipBytes(strToU8('; G-code\nG1 X0\n'))).toBe(false);
		expect(is3mfName('cube.gcode.3mf')).toBe(true);
		expect(is3mfName('cube.GCODE.3MF')).toBe(true);
		expect(is3mfName('cube.gcode')).toBe(false);
	});

	it('liste les plateaux tranchés (Metadata/plate_<n>.gcode) dans l’ordre', async () => {
		expect(
			archivePlates([
				'Metadata/plate_10.gcode',
				'Metadata/plate_2.gcode',
				'Metadata/plate_2.gcode.md5'
			])
		).toEqual([2, 10]);
		expect(await gcodeFilePlates(archive(SLICED))).toEqual([1, 2]);
		expect(await gcodeFilePlates(new Blob(['; G-code\n']))).toEqual([]);
		expect(await gcodeFilePlates(archive({ '3D/3dmodel.model': '<model/>' }))).toEqual([]);
	});

	it('relit le G-code du plateau demandé, à défaut le premier', async () => {
		const file = archive(SLICED);
		expect(await gcodeFileText(file)).toBe('; plate 1\nG1 X1\n');
		expect(await gcodeFileText(file, 2)).toBe('; plate 2\nG1 X2\n');
		expect(await gcodeFileText(file, 7)).toBe('; plate 1\nG1 X1\n');
		expect(await gcodeFileText(new Blob(['G1 X0\n']))).toBe('G1 X0\n');
		expect(await gcodeFileText(archive({ 'a.txt': 'x' }))).toBe('');
	});

	it('reconnaît le refus 422 « gcode_3mf » de l’ouverture de projet', () => {
		const refused = new ApiError(422, 'validation', 'fichier tranché', {
			filename: 'a.gcode.3mf',
			kind: 'gcode_3mf',
			plates: [1, 2]
		});
		expect(gcode3mfRefusal(refused)).toEqual({ plates: [1, 2] });
		expect(
			gcode3mfRefusal(new ApiError(422, 'validation', 'x', { kind: 'project_3mf' }))
		).toBeNull();
		expect(gcode3mfRefusal(new ApiError(400, 'bad', 'x', { kind: 'gcode_3mf' }))).toBeNull();
		expect(gcode3mfRefusal(new Error('x'))).toBeNull();
		expect(gcode3mfRefusal(null)).toBeNull();
	});
});
