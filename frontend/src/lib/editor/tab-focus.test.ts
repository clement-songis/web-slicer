import { describe, expect, it } from 'vitest';
import {
	EDITOR_CHROME_SELECTOR,
	pointerFocusesView,
	tabTogglesView,
	VIEW_ZONE_SELECTOR
} from './tab-focus';

const el = (tagName: string, inZone = false, role = false, chrome = false) => ({
	tagName,
	closest: (s: string) =>
		s === VIEW_ZONE_SELECTOR
			? inZone
				? {}
				: null
			: s === EDITOR_CHROME_SELECTOR
				? chrome
					? {}
					: null
				: role
					? {}
					: null
});

describe('tabTogglesView', () => {
	it('page tout juste ouverte (body, aucun clic dans la scène) : navigation native (A1)', () => {
		expect(tabTogglesView(null)).toBe(false);
		expect(tabTogglesView(el('BODY'))).toBe(false);
	});
	it('bascule quand la zone 3D a le focus (élément focalisé ou dernier clic)', () => {
		expect(tabTogglesView(null, true)).toBe(true);
		expect(tabTogglesView(el('BODY'), true)).toBe(true);
		expect(tabTogglesView(el('CANVAS', true))).toBe(true);
		expect(tabTogglesView(el('DIV', true))).toBe(true);
	});
	it('navigation native dès qu’un élément de l’interface a le focus', () => {
		expect(tabTogglesView(el('BUTTON'))).toBe(false);
		expect(tabTogglesView(el('DIV'))).toBe(false);
		expect(tabTogglesView(el('SELECT'))).toBe(false);
		expect(tabTogglesView(el('DIV'), true)).toBe(false);
	});
	it('contrôles posés sur la vue 3D : navigation native (pas de piège clavier)', () => {
		expect(tabTogglesView(el('BUTTON', true))).toBe(false);
		expect(tabTogglesView(el('INPUT', true))).toBe(false);
		expect(tabTogglesView(el('A', true))).toBe(false);
	});
});

describe('pointerFocusesView', () => {
	it('clic dans le canevas : la scène prend le focus', () => {
		expect(pointerFocusesView(el('CANVAS', true))).toBe(true);
	});
	it('fond de la barre supérieure (chrome, comme la barre d’Orca) : la scène garde le focus', () => {
		expect(pointerFocusesView(el('HEADER', false, false, true))).toBe(true);
		expect(pointerFocusesView(el('BUTTON', false, false, true))).toBe(false);
	});
	it('clic dans un panneau, sur un contrôle ou un rôle interactif : non', () => {
		expect(pointerFocusesView(null)).toBe(false);
		expect(pointerFocusesView(el('DIV'))).toBe(false);
		expect(pointerFocusesView(el('BUTTON', true))).toBe(false);
		expect(pointerFocusesView(el('DIV', true, true))).toBe(false);
	});
});
