// Phase 27 — onglets sans option au mode courant masqués (Orca : Page::get_show).
import { describe, expect, it } from 'vitest';
import { UI_LAYOUT } from '../../generated/ui-layout';
import { filterLayout } from './filter';
import { resolveDynamicOptions } from './dynamic-options';

const LAYOUT = resolveDynamicOptions(UI_LAYOUT);
const machine = LAYOUT.filter((p) => p.kind === 'machine');
const titles = (mode: 'simple' | 'advanced' | 'develop') =>
	filterLayout(machine, mode, '', false).map((p) => p.title);

describe('onglets machine selon le mode', () => {
	it('« Machine G-code » n’a aucune option en Simple : onglet masqué', () => {
		expect(titles('develop')).toContain('Machine G-code');
		expect(titles('simple')).not.toContain('Machine G-code');
	});

	it('il réapparaît à un mode supérieur', () => {
		const shown = (['advanced', 'develop'] as const).some((m) =>
			titles(m).includes('Machine G-code')
		);
		expect(shown).toBe(true);
	});

	it('Simple garde au moins un onglet machine (jamais de dialogue vide)', () => {
		expect(titles('simple').length).toBeGreaterThan(0);
	});
});
