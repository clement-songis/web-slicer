import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, test } from 'vitest';
import { decodeFacet, encodeFacet, remapChannelStates, remapFacetStates } from './facets';

/** Fixtures du moteur (référence Orca isolée, lot 38). */
function fixture(name: string): Record<string, string> {
	const url = new URL(`../../../../../../engine/tests/fixtures/paint/${name}`, import.meta.url);
	const out: Record<string, string> = {};
	for (const line of readFileSync(fileURLToPath(url), 'utf8').split('\n')) {
		const [tri, hex] = line.trim().split(/\s+/);
		if (tri && hex) out[tri] = hex;
	}
	return out;
}

describe('renumérotation des états peints (chaînes Orca)', () => {
	test('identique octet par octet à remap_triangle_state d’Orca (remap.txt, table 1→3, 2→1)', () => {
		const input = fixture('remap_input.txt');
		const expected = fixture('remap.txt');
		const map = (s: number) => (s === 1 ? 3 : s === 2 ? 1 : s);
		expect(remapChannelStates(input, map)).toEqual(expected);
	});

	test('feuille entière : état ≥ 3 sur deux quartets, effacement → retrait', () => {
		expect(remapFacetStates(encodeFacet(1), () => 5)).toBe(encodeFacet(5));
		expect(decodeFacet(remapFacetStates(encodeFacet(7), (s) => s - 1))).toBe(6);
		expect(remapFacetStates(encodeFacet(2), () => 0)).toBe('');
		expect(
			remapChannelStates({ '3': encodeFacet(2), '4': encodeFacet(1) }, (s) => (s === 2 ? 0 : s))
		).toEqual({
			'4': encodeFacet(1)
		});
	});

	test('triangle subdivisé : structure conservée, feuilles renumérotées', () => {
		const input = fixture('remap_input.txt');
		const [tri, hex] = Object.entries(input)[0];
		const there = remapFacetStates(hex, (s) => s + 2);
		const back = remapFacetStates(there, (s) => s - 2);
		expect(back).toBe(hex);
		expect(tri).toBeTruthy();
	});
});
