import { describe, expect, test } from 'vitest';
import { zipSync } from 'fflate';
import { modelsFromZip, presetsFromFiles } from './menu-files';

describe('menu file imports', () => {
	test('extrait seulement les modèles et ignore les répertoires', async () => {
		const archive = new File(
			[zipSync({ 'cube.stl': new Uint8Array([1, 2]), 'notes.txt': new Uint8Array([3]) })],
			'models.zip'
		);
		const files = await modelsFromZip(archive);
		expect(files.map((f) => f.name)).toEqual(['cube.stl']);
		expect(new Uint8Array(await files[0].arrayBuffer())).toEqual(new Uint8Array([1, 2]));
	});

	test('lit les presets JSON et convertit le type Orca print en process', async () => {
		const file = new File(
			[JSON.stringify({ type: 'print', name: 'Mon profil', layer_height: '0.16' })],
			'profile.json'
		);
		expect(await presetsFromFiles([file])).toEqual([
			{ kind: 'process', name: 'Mon profil', values: { layer_height: '0.16' } }
		]);
	});
});
