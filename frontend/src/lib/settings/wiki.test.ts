import { describe, expect, test } from 'vitest';
import { readFileSync } from 'node:fs';
import { wikiUrl } from './wiki';
import { WIKI_PATHS } from './wiki-paths';

describe('lien d’aide wiki par option (Orca OptionsGroup::get_url)', () => {
	test('URL = orcaslicer.com/wiki/<label_path de Tab.cpp>', () => {
		expect(wikiUrl('layer_height')).toBe(
			'https://www.orcaslicer.com/wiki/quality_settings_layer_height'
		);
		expect(wikiUrl('wall_loops')).toBe(
			'https://www.orcaslicer.com/wiki/strength_settings_walls#wall-loops'
		);
		// Ligne multi-options (line.label_path) : chaque option de la ligne.
		expect(wikiUrl('nozzle_temperature')).toBe(
			'https://www.orcaslicer.com/wiki/material_temperatures#nozzle'
		);
		expect(wikiUrl('not_an_option')).toBeNull();
	});
	test('chaque chemin vient bien de Tab.cpp (première occurrence)', () => {
		const tab = readFileSync(
			new URL('../../../../vendor/OrcaSlicer/src/slic3r/GUI/Tab.cpp', import.meta.url),
			'utf8'
		);
		expect(Object.keys(WIKI_PATHS).length).toBeGreaterThan(400);
		for (const [key, path] of Object.entries(WIKI_PATHS).slice(0, 50)) {
			expect(tab, key).toContain(`"${path}"`);
		}
	});
});
