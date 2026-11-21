import { beforeAll, afterAll, describe, expect, it } from 'vitest';
import { setLocale } from '$lib/i18n';
import { buildZip, crc32 } from './zip';
import { safeFileName } from './stl';

describe('zip stored', () => {
	it('crc32 de référence', () => {
		expect(crc32(new TextEncoder().encode('123456789'))).toBe(0xcbf43926);
	});
	it('en-têtes et répertoire central', () => {
		const zip = buildZip([
			{ name: 'a.gcode', data: new TextEncoder().encode('G1 X1') },
			{ name: 'b.gcode', data: new TextEncoder().encode('G1 X2') }
		]);
		const v = new DataView(zip.buffer);
		expect(v.getUint32(0, true)).toBe(0x04034b50);
		const end = zip.length - 22;
		expect(v.getUint32(end, true)).toBe(0x06054b50);
		expect(v.getUint16(end + 10, true)).toBe(2);
	});
});

describe('noms de fichiers', () => {
	it('noms de fichiers sûrs', () => {
		expect(safeFileName('Cube/20.stl', 'stl')).toBe('Cube_20.stl');
		expect(safeFileName('', 'stl')).toBe('objet.stl');
	});
});

// Les libellés de ces modules sont traduits (tr) : valeurs attendues en français.
beforeAll(() => setLocale('fr'));
afterAll(() => setLocale('en'));
