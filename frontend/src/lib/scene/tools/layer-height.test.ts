// Tests du profil de hauteur de couche variable et de la vue éclatée (T058, G2).
import { describe, expect, test } from 'bun:test';
import {
	uniformProfile,
	heightAt,
	setBand,
	smooth,
	serialize,
	deserialize,
	clampHeight,
	DEFAULT_MAX_HEIGHT
} from './layer-height';
import { sceneCenter, explode } from './assembly';

describe('profil de hauteur de couche', () => {
	test('profil uniforme : hauteur constante partout', () => {
		const p = uniformProfile(10, 0.2);
		expect(heightAt(p, 0)).toBeCloseTo(0.2);
		expect(heightAt(p, 5)).toBeCloseTo(0.2);
		expect(heightAt(p, 10)).toBeCloseTo(0.2);
		// Extrapolation constante au-delà des bornes.
		expect(heightAt(p, 100)).toBeCloseTo(0.2);
	});

	test('heightAt interpole linéairement', () => {
		const p = [
			{ z: 0, height: 0.1 },
			{ z: 10, height: 0.3 }
		];
		expect(heightAt(p, 5)).toBeCloseTo(0.2);
	});

	test('setBand impose une hauteur constante sur une fenêtre', () => {
		const p = setBand(uniformProfile(10, 0.2), 3, 6, 0.1);
		expect(heightAt(p, 4)).toBeCloseTo(0.1);
		expect(heightAt(p, 0)).toBeCloseTo(0.2);
	});

	test('setBand borne la hauteur à l’intervalle imprimable', () => {
		const p = setBand(uniformProfile(10, 0.2), 0, 10, 99);
		expect(heightAt(p, 5)).toBeCloseTo(DEFAULT_MAX_HEIGHT);
	});

	test('smooth atténue un pic', () => {
		const spike = [
			{ z: 0, height: 0.2 },
			{ z: 1, height: 0.6 },
			{ z: 2, height: 0.2 }
		];
		const out = smooth(spike, 1);
		expect(out[1].height).toBeCloseTo(0.2); // moyenne des voisins
		expect(out[0].height).toBeCloseTo(0.2); // bords inchangés
	});

	test('sérialisation aller-retour (vecteur plat Orca)', () => {
		const p = [
			{ z: 0, height: 0.2 },
			{ z: 5, height: 0.15 }
		];
		expect(serialize(p)).toEqual([0, 0.2, 5, 0.15]);
		expect(deserialize([0, 0.2, 5, 0.15])).toEqual(p);
	});

	test('clampHeight borne et rejette les valeurs non finies', () => {
		expect(clampHeight(0.3)).toBeCloseTo(0.3);
		expect(clampHeight(-1)).toBeCloseTo(0.05);
		expect(clampHeight(NaN)).toBeCloseTo(0.05);
	});
});

describe('vue d’assemblage éclatée', () => {
	test('centre de scène = moyenne des centres', () => {
		expect(
			sceneCenter([
				{ id: 'a', center: [0, 0, 0] },
				{ id: 'b', center: [10, 0, 0] }
			])
		).toEqual([5, 0, 0]);
	});

	test('facteur 0 laisse les pièces en place', () => {
		const parts = [
			{ id: 'a', center: [0, 0, 0] as [number, number, number] },
			{ id: 'b', center: [10, 0, 0] as [number, number, number] }
		];
		expect(explode(parts, 0).map((e) => e.position)).toEqual([
			[0, 0, 0],
			[10, 0, 0]
		]);
	});

	test('facteur positif écarte radialement depuis le centre', () => {
		const parts = [
			{ id: 'a', center: [0, 0, 0] as [number, number, number] },
			{ id: 'b', center: [10, 0, 0] as [number, number, number] }
		];
		// Centre = (5,0,0) ; facteur 1 double la distance au centre.
		const pos = explode(parts, 1).map((e) => e.position);
		expect(pos[0]).toEqual([-5, 0, 0]);
		expect(pos[1]).toEqual([15, 0, 0]);
	});
});
