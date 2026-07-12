// Tests de l'orchestrateur du workspace éditeur (T087) : bascule de panneau,
// modes gizmo/réglages, pont de sélection scène↔liste, éligibilité au tranchage.
import { describe, expect, it } from 'vitest';
import {
	canSlice,
	initialWorkspace,
	isSelected,
	pick,
	setGizmoMode,
	setPanel,
	setSelection,
	setSettingsMode,
	togglePanel
} from './workspace';

describe('workspace', () => {
	it('démarre en préparation, gizmo déplacer, mode simple, sans sélection', () => {
		const ws = initialWorkspace();
		expect(ws.panel).toBe('prepare');
		expect(ws.gizmoMode).toBe('translate');
		expect(ws.settingsMode).toBe('simple');
		expect(ws.selection.size).toBe(0);
	});

	it('bascule de panneau (explicite et alternée), immuablement', () => {
		const ws = initialWorkspace();
		const preview = setPanel(ws, 'preview');
		expect(preview.panel).toBe('preview');
		expect(ws.panel).toBe('prepare'); // l'original n'est pas muté
		expect(togglePanel(ws).panel).toBe('preview');
		expect(togglePanel(togglePanel(ws)).panel).toBe('prepare');
	});

	it('change les modes gizmo et réglages sans toucher au reste', () => {
		const ws = initialWorkspace();
		const rotated = setGizmoMode(ws, 'rotate');
		expect(rotated.gizmoMode).toBe('rotate');
		expect(rotated.panel).toBe('prepare');
		expect(setSettingsMode(ws, 'expert').settingsMode).toBe('expert');
	});

	it('propage la sélection : clic simple, bascule additive, clic dans le vide', () => {
		let ws = initialWorkspace();
		ws = pick(ws, 'a', false);
		expect([...ws.selection]).toEqual(['a']);

		// Additif : ajoute puis retire.
		ws = pick(ws, 'b', true);
		expect(isSelected(ws, 'a')).toBe(true);
		expect(isSelected(ws, 'b')).toBe(true);
		ws = pick(ws, 'b', true);
		expect(isSelected(ws, 'b')).toBe(false);

		// Clic simple : remplace la sélection.
		ws = pick(ws, 'c', false);
		expect([...ws.selection]).toEqual(['c']);

		// Clic dans le vide sans modificateur : vide.
		ws = pick(ws, null, false);
		expect(ws.selection.size).toBe(0);
	});

	it('synchronise la sélection depuis un ensemble externe (scène liée)', () => {
		const ws = setSelection(initialWorkspace(), new Set(['x', 'y']));
		expect([...ws.selection].sort()).toEqual(['x', 'y']);
	});

	it('mappe l’état vers le déclencheur de tranchage (au moins un objet)', () => {
		expect(canSlice(0)).toBe(false);
		expect(canSlice(1)).toBe(true);
		expect(canSlice(20)).toBe(true);
	});
});
