// Profil de hauteur de couche variable (T058, toolbar `layersediting`).
// Miroir du `layer_height_profile` d'OrcaSlicer : un vecteur plat de paires
// (z, hauteur) trié par z. Modèle pur (édition par bande, lissage adaptatif,
// interpolation) → testable ; persisté dans le document scène. Le calcul
// adaptatif à partir du maillage est une opération moteur (FFI, phase P5).
import type { SceneMesh } from '../mesh';

/** Point de contrôle du profil : hauteur de couche à la cote `z`. */
export interface LayerBand {
	z: number;
	height: number;
}

export const DEFAULT_MIN_HEIGHT = 0.05;
export const DEFAULT_MAX_HEIGHT = 0.8;

/** Borne une hauteur de couche dans l'intervalle imprimable. */
export function clampHeight(h: number, min = DEFAULT_MIN_HEIGHT, max = DEFAULT_MAX_HEIGHT): number {
	if (!Number.isFinite(h)) return min;
	return Math.min(max, Math.max(min, h));
}

/** Profil uniforme : hauteur constante de 0 à `zMax`. */
export function uniformProfile(zMax: number, height: number): LayerBand[] {
	return [
		{ z: 0, height },
		{ z: zMax, height }
	];
}

function sorted(profile: LayerBand[]): LayerBand[] {
	return [...profile].sort((a, b) => a.z - b.z);
}

/** Hauteur interpolée du profil à la cote `z` (extrapolation constante aux bords). */
export function heightAt(profile: LayerBand[], z: number): number {
	if (profile.length === 0) return NaN;
	const p = sorted(profile);
	if (z <= p[0].z) return p[0].height;
	if (z >= p[p.length - 1].z) return p[p.length - 1].height;
	for (let i = 1; i < p.length; i++) {
		if (z <= p[i].z) {
			const a = p[i - 1];
			const b = p[i];
			const t = b.z === a.z ? 0 : (z - a.z) / (b.z - a.z);
			return a.height + (b.height - a.height) * t;
		}
	}
	return p[p.length - 1].height;
}

/**
 * Impose une hauteur constante sur la bande `[z0, z1]` : retire les points de
 * contrôle strictement à l'intérieur et pose des points aux bornes. Les
 * hauteurs sont bornées à l'intervalle imprimable.
 */
export function setBand(profile: LayerBand[], z0: number, z1: number, height: number): LayerBand[] {
	const lo = Math.min(z0, z1);
	const hi = Math.max(z0, z1);
	const h = clampHeight(height);
	const kept = profile.filter((b) => b.z < lo || b.z > hi);
	kept.push({ z: lo, height: h }, { z: hi, height: h });
	return sorted(kept);
}

/**
 * Lissage adaptatif : moyenne mobile pondérée sur les hauteurs des points
 * intérieurs. `strength` ∈ [0, 1] (0 = inchangé, 1 = moyenne des voisins).
 */
export function smooth(profile: LayerBand[], strength: number): LayerBand[] {
	const s = Math.min(1, Math.max(0, strength));
	const p = sorted(profile);
	if (p.length < 3 || s === 0) return p;
	const out = p.map((b) => ({ ...b }));
	for (let i = 1; i < p.length - 1; i++) {
		const neighbour = (p[i - 1].height + p[i + 1].height) / 2;
		out[i].height = clampHeight(p[i].height * (1 - s) + neighbour * s);
	}
	return out;
}

/** Sérialise en vecteur plat Orca `[z0, h0, z1, h1, …]`. */
export function serialize(profile: LayerBand[]): number[] {
	const flat: number[] = [];
	for (const b of sorted(profile)) flat.push(b.z, b.height);
	return flat;
}

/** Reconstruit un profil depuis un vecteur plat Orca. */
export function deserialize(flat: number[]): LayerBand[] {
	const profile: LayerBand[] = [];
	for (let i = 0; i + 1 < flat.length; i += 2) profile.push({ z: flat[i], height: flat[i + 1] });
	return sorted(profile);
}

/** Cote Z maximale d'un maillage (borne haute du profil). */
export function meshMaxZ(mesh: SceneMesh): number {
	let maxZ = 0;
	for (let i = 2; i < mesh.positions.length; i += 3) {
		if (mesh.positions[i] > maxZ) maxZ = mesh.positions[i];
	}
	return maxZ;
}
