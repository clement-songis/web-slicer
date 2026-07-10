// Fraîcheur du layout généré `ui-layout.ts` (T040) : reconstruit l'arbre
// pages/sections/options attendu depuis audit/ui_inventory.json et le compare
// au layout généré. Vérifie aussi que chaque clé d'option existe dans PARAMS.
import { describe, expect, it } from 'bun:test';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { PARAMS } from '../../generated/params';
import { UI_LAYOUT, UI_LAYOUT_COUNTS, type UiOption } from '../../generated/ui-layout';

const AUDIT = resolve(import.meta.dir, '../../../../audit/ui_inventory.json');
const audit = JSON.parse(readFileSync(AUDIT, 'utf-8')) as {
	summary: {
		settings_pages: number;
		settings_sections: number;
		settings_option_lines: number;
	};
	settings_tabs: Array<{
		title: string;
		icon: string;
		sections: Array<{ title: string; options: UiOption[] }>;
	}>;
};

function countSections(): number {
	return UI_LAYOUT.reduce((n, p) => n + p.sections.length, 0);
}

function countOptions(): number {
	return UI_LAYOUT.reduce((n, p) => n + p.sections.reduce((m, s) => m + s.options.length, 0), 0);
}

describe('layout UI généré', () => {
	it('respecte les compteurs de l’audit (21 / 100 / 525)', () => {
		expect(UI_LAYOUT.length).toBe(audit.summary.settings_pages);
		expect(UI_LAYOUT_COUNTS.pages).toBe(audit.summary.settings_pages);
		expect(countSections()).toBe(audit.summary.settings_sections);
		expect(UI_LAYOUT_COUNTS.sections).toBe(audit.summary.settings_sections);
		expect(countOptions()).toBe(audit.summary.settings_option_lines);
		expect(UI_LAYOUT_COUNTS.optionLines).toBe(audit.summary.settings_option_lines);
	});

	it('reproduit fidèlement l’arbre pages → sections → options', () => {
		const expected = audit.settings_tabs.map((tab) => ({
			title: tab.title,
			icon: tab.icon,
			sections: tab.sections.map((s) => ({ title: s.title, options: s.options }))
		}));
		expect(UI_LAYOUT).toEqual(expected);
	});

	it('ne référence que des clés de PARAMS (hors marqueurs dynamiques)', () => {
		for (const page of UI_LAYOUT) {
			for (const section of page.sections) {
				for (const option of section.options) {
					if (typeof option === 'string') {
						expect(PARAMS[option], `option « ${option} » absente du registre`).toBeDefined();
					} else {
						expect(option.dynamic).toBeDefined();
					}
				}
			}
		}
	});
});
