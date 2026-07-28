// Visualisation de la peinture au triangle (T145) : produit une géométrie
// **non indexée** teintée par sommet à partir d'un document de peinture 3MF Orca,
// afin de colorer les facettes peintes dans la vue 3D (parité OrcaSlicer : on
// *voit* ce qu'on peint, pas seulement un compteur). Pur → testable.
import type { SceneMesh } from '../../mesh';
import { decodeFacet } from './facets';
import type { PaintDocument, PaintChannel } from './painting';

/** Couleur de base (facette non peinte) — repère gris-bleu neutre. */
const BASE: [number, number, number] = [0.6, 0.65, 0.7];

/** Ordre de priorité si une facette est peinte dans plusieurs canaux. */
const CHANNEL_ORDER: PaintChannel[] = ['supports', 'seam', 'fuzzy', 'mmu'];

/** Couleur d'un canal selon l'état peint (enforcer/blocker pour les supports). */
function channelColor(channel: PaintChannel, state: number): [number, number, number] {
	switch (channel) {
		case 'supports':
			// enforcer (1) rouge · blocker (2) bleu (valeurs Orca).
			return state === 2 ? [0.2, 0.45, 0.95] : [0.95, 0.25, 0.25];
		case 'seam':
			return [0.7, 0.3, 0.85];
		case 'fuzzy':
			return [0.2, 0.75, 0.55];
		case 'mmu':
			return [0.95, 0.8, 0.2];
	}
}

/**
 * Positions non indexées + couleurs par sommet visualisant la peinture. Chaque
 * triangle est développé en 3 sommets propres, coloré selon le premier canal
 * peint (par priorité) ; les facettes non peintes prennent la couleur de base.
 * Renvoie `null` si **aucune** facette n'est peinte (on garde alors la géométrie
 * indexée normale, moins lourde).
 */
export function paintFaceColorData(
	mesh: SceneMesh,
	painting: PaintDocument
): { positions: Float32Array; colors: Float32Array } | null {
	const colorByTriangle = new Map<number, [number, number, number]>();
	for (const channel of CHANNEL_ORDER) {
		const entries = painting[channel];
		if (!entries) continue;
		for (const [key, hex] of Object.entries(entries)) {
			const triangle = Number(key);
			if (colorByTriangle.has(triangle)) continue; // priorité déjà posée
			const state = decodeFacet(hex);
			if (state > 0) colorByTriangle.set(triangle, channelColor(channel, state));
		}
	}
	if (colorByTriangle.size === 0) return null;

	const triangleCount = mesh.indices.length / 3;
	const positions = new Float32Array(triangleCount * 9);
	const colors = new Float32Array(triangleCount * 9);
	for (let t = 0; t < triangleCount; t++) {
		const color = colorByTriangle.get(t) ?? BASE;
		for (let k = 0; k < 3; k++) {
			const i = mesh.indices[t * 3 + k];
			const o = t * 9 + k * 3;
			positions[o] = mesh.positions[i * 3];
			positions[o + 1] = mesh.positions[i * 3 + 1];
			positions[o + 2] = mesh.positions[i * 3 + 2];
			colors[o] = color[0];
			colors[o + 1] = color[1];
			colors[o + 2] = color[2];
		}
	}
	return { positions, colors };
}
