// Phase 27 — onglets sans option au mode courant masqués (Orca : Page::get_show).
import { describe, expect, it } from 'vitest';
import { UI_LAYOUT } from '../../generated/ui-layout';
import { filterLayout, isEditorOnlyPage } from './filter';
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

describe('seuls les onglets d’éditeurs de texte se masquent', () => {
	const editorOnly = (kind: string, title: string) =>
		isEditorOnlyPage(LAYOUT.find((p) => p.kind === kind && p.title === title)!);

	it('G-code machine, notes et G-code filament : éditeurs seuls', () => {
		expect(editorOnly('machine', 'Machine G-code')).toBe(true);
		expect(editorOnly('machine', 'Notes')).toBe(true);
		expect(editorOnly('filament', 'Advanced')).toBe(true);
		expect(editorOnly('filament', 'Notes')).toBe(true);
	});

	it('onglets de paramètres (Vitesse, Capacités de mouvement…) : stables, jamais masqués', () => {
		// Aucun réglage Simple dans ces onglets, mais leur onglet reste présent
		// (contrat UI des tests params : visible au niveau, masqué en dessous).
		expect(editorOnly('process', 'Speed')).toBe(false);
		expect(editorOnly('machine', 'Motion ability')).toBe(false);
		expect(editorOnly('filament', 'Multimaterial')).toBe(false);
		expect(titles('simple')).not.toContain('Motion ability');
	});
});
