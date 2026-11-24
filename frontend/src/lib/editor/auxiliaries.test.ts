import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { setLocale } from '#lib/i18n';
import {
	AUX_FOLDERS,
	formatSize,
	groupAuxiliaries,
	isPreviewableImage,
	safeAuxName,
	uniqueAuxName
} from './auxiliaries';

describe('fichiers joints au projet (Auxiliaries/ d Orca)', () => {
	beforeAll(() => setLocale('fr'));
	afterAll(() => setLocale('en'));
	it('4 dossiers d Orca, noms exacts du 3MF', () => {
		expect(AUX_FOLDERS.map((f) => f.id)).toEqual([
			'Model Pictures',
			'Bill of Materials',
			'Assembly Guide',
			'Others'
		]);
	});

	it('regroupe la liste du serveur par dossier, noms triés, entrées invalides ignorées', () => {
		const g = groupAuxiliaries([
			{ path: 'Others/z.txt', size: 3 },
			{ path: 'Model Pictures/b.png', size: 10 },
			{ path: 'Model Pictures/a.jpg', size: 20 },
			{ path: 'Inconnu/x', size: 1 },
			{ path: 'sansdossier', size: 1 },
			{ path: 'Others/', size: 1 }
		]);
		expect(g['Model Pictures'].map((f) => f.name)).toEqual(['a.jpg', 'b.png']);
		expect(g['Others']).toEqual([{ folder: 'Others', name: 'z.txt', size: 3 }]);
		expect(g['Bill of Materials']).toEqual([]);
		expect(g['Assembly Guide']).toEqual([]);
	});

	it('noms sûrs pour le serveur et sans collision', () => {
		expect(safeAuxName('C:\\photos\\vue.png')).toBe('vue.png');
		expect(safeAuxName('a/b/../c.pdf')).toBe('c.pdf');
		expect(safeAuxName('.cache')).toBe('cache');
		expect(safeAuxName('..')).toBe('fichier');
		expect(uniqueAuxName('vue.png', ['VUE.png'])).toBe('vue (2).png');
		expect(uniqueAuxName('vue.png', ['vue.png', 'vue (2).png'])).toBe('vue (3).png');
		expect(uniqueAuxName('notice', [])).toBe('notice');
	});

	it('vignettes pour les images, tailles lisibles', () => {
		expect(isPreviewableImage('a.JPG')).toBe(true);
		expect(isPreviewableImage('a.pdf')).toBe(false);
		expect(formatSize(512)).toBe('512 o');
		expect(formatSize(2048)).toBe('2 Ko');
		expect(formatSize(3 * 1024 * 1024)).toBe('3 Mo');
	});
});
