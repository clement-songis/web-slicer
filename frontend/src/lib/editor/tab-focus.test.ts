import { describe, expect, it } from 'vitest';
import { tabTogglesView, VIEW_ZONE_SELECTOR } from './tab-focus';

const el = (tagName: string, inZone = false) => ({
	tagName,
	closest: (s: string) => (s === VIEW_ZONE_SELECTOR && inZone ? {} : null)
});

describe('tabTogglesView', () => {
	it('bascule depuis le body (aucun focus) ou la zone 3D', () => {
		expect(tabTogglesView(null)).toBe(true);
		expect(tabTogglesView(el('BODY'))).toBe(true);
		expect(tabTogglesView(el('CANVAS', true))).toBe(true);
		expect(tabTogglesView(el('DIV', true))).toBe(true);
	});
	it('navigation native dès qu’un élément de l’interface a le focus', () => {
		expect(tabTogglesView(el('BUTTON'))).toBe(false);
		expect(tabTogglesView(el('DIV'))).toBe(false);
		expect(tabTogglesView(el('SELECT'))).toBe(false);
	});
	it('contrôles posés sur la vue 3D : navigation native (pas de piège clavier)', () => {
		expect(tabTogglesView(el('BUTTON', true))).toBe(false);
		expect(tabTogglesView(el('INPUT', true))).toBe(false);
		expect(tabTogglesView(el('A', true))).toBe(false);
	});
});
