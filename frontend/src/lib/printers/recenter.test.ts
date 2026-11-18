import { beforeEach, describe, expect, it } from 'vitest';
import { RECENTER_KEY, muteRecenter, recenterMuted, unhomedAxes } from './device';
import { discoveryService } from './hosts';

describe('origine avant déplacement (Orca RecenterDialog)', () => {
	beforeEach(() => {
		const map = new Map<string, string>();
		(globalThis as { localStorage?: unknown }).localStorage = {
			getItem: (k: string) => map.get(k) ?? null,
			setItem: (k: string, v: string) => map.set(k, v),
			removeItem: (k: string) => map.delete(k)
		};
	});
	it('axes sans origine (homed_axes Klipper)', () => {
		expect(unhomedAxes('')).toEqual(['X', 'Y', 'Z']);
		expect(unhomedAxes('xy')).toEqual(['Z']);
		expect(unhomedAxes('xyz')).toEqual([]);
		expect(unhomedAxes('xy', ['X'])).toEqual([]);
		expect(unhomedAxes(undefined)).toEqual([]);
		expect(unhomedAxes(null)).toEqual([]);
	});
	it('« Ne plus avertir » mémorisé', () => {
		expect(recenterMuted()).toBe(false);
		muteRecenter();
		expect(recenterMuted()).toBe(true);
		expect(localStorage.getItem(RECENTER_KEY)).toBe('muted');
	});
	it('service DNS-SD par type d’hôte (Parcourir le réseau)', () => {
		expect(discoveryService('octoprint')).toBe('octoprint');
		expect(discoveryService('moonraker')).toBe('moonraker');
		expect(discoveryService('repetier')).toBeNull();
	});
});
