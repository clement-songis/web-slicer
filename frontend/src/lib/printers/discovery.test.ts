import { describe, expect, it } from 'vitest';
import { discoveryHints, discoveryService, discoveryTimeoutMs } from './hosts';

describe('Parcourir le réseau (Orca PhysicalPrinterDialog : Browse)', () => {
	it('Creality Print et Flashforge ont leur propre recherche', () => {
		expect(discoveryService('crealityprint')).toBe('creality');
		expect(discoveryService('flashforge')).toBe('flashforge');
		expect(discoveryService('elegoolink')).toBeNull();
		expect(discoveryService('obico')).toBeNull();
	});
	it('durées d’écoute d’Orca', () => {
		expect(discoveryTimeoutMs('creality')).toBe(5000);
		expect(discoveryTimeoutMs('flashforge')).toBe(10_000);
		expect(discoveryTimeoutMs('octoprint')).toBe(4000);
	});
	it('messages propres au constructeur', () => {
		expect(discoveryHints('creality').none).toMatch(/Creality/);
		expect(discoveryHints('flashforge').waiting).toMatch(/Flashforge/);
		expect(discoveryHints('octoprint').none).toMatch(/mDNS/);
	});
});
