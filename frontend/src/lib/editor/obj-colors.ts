// Import OBJ coloré (Orca : ObjColorDialog). Le serveur
// (`POST /api/models/{id}/obj-colors`, lot 16) regroupe les couleurs de l'OBJ
// (sommets « v x y z r g b » ou matériaux MTL Kd, k-means libslic3r) en une
// palette — couleur n = filament n — et renvoie la peinture MMU par facette.
// Ce module (pur) prépare le dialogue : regroupement à N couleurs, affectation
// de chaque couleur à un filament existant ou nouveau, puis renumérotation de
// la peinture MMU vers les extrudeurs choisis.
import { decodeFacet, encodeFacet } from '$lib/scene/gizmos/painting/facets';

/** Réponse de `POST /api/models/{id}/obj-colors`. */
export interface ObjColorsResponse {
	colored: boolean;
	/** Couleurs regroupées « #RRGGBBAA », filament n = palette[n-1]. */
	palette: string[];
	/** Extrudeur de base du volume (facettes non peintes). */
	extruder: number;
	triangles: number;
	painting: { mmu?: Record<string, string> };
}

/** Nombre maximal de filaments (états MMU encodables, Orca : 16). */
export const MAX_FILAMENTS = 16;

const HEX6 = /^#?([0-9a-f]{6})([0-9a-f]{2})?$/i;

/** « #RRGGBBAA » / « #RRGGBB » → « #rrggbb » (null si illisible). */
export function paletteHex(c: string): string | null {
	const m = HEX6.exec(c.trim());
	return m ? `#${m[1].toLowerCase()}` : null;
}

function rgb(hex: string): [number, number, number] {
	const h = paletteHex(hex) ?? '#808080';
	return [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16)) as [number, number, number];
}

function toHex([r, g, b]: [number, number, number]): string {
	return `#${[r, g, b]
		.map((v) =>
			Math.round(Math.min(255, Math.max(0, v)))
				.toString(16)
				.padStart(2, '0')
		)
		.join('')}`;
}

/** sRGB → CIE Lab (D65), pour une distance perçue (ΔE76). */
function lab(hex: string): [number, number, number] {
	const lin = rgb(hex).map((v) => {
		const c = v / 255;
		return c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
	});
	const x = (lin[0] * 0.4124 + lin[1] * 0.3576 + lin[2] * 0.1805) / 0.95047;
	const y = lin[0] * 0.2126 + lin[1] * 0.7152 + lin[2] * 0.0722;
	const z = (lin[0] * 0.0193 + lin[1] * 0.1192 + lin[2] * 0.9505) / 1.08883;
	const f = (t: number) => (t > 216 / 24389 ? Math.cbrt(t) : ((24389 / 27) * t) / 116 + 16 / 116);
	return [116 * f(y) - 16, 500 * (f(x) - f(y)), 200 * (f(y) - f(z))];
}

/** Distance perçue entre deux couleurs (ΔE76). */
export function colorDistance(a: string, b: string): number {
	const p = lab(a);
	const q = lab(b);
	return Math.hypot(p[0] - q[0], p[1] - q[1], p[2] - q[2]);
}

/** Slot (0-based) dont la couleur est la plus proche. */
export function nearestSlot(color: string, slotColours: readonly string[]): number {
	let best = 0;
	let bestD = Infinity;
	slotColours.forEach((c, i) => {
		const d = colorDistance(color, c);
		if (d < bestD) {
			bestD = d;
			best = i;
		}
	});
	return best;
}

/** Groupe de couleurs détectées fusionnées (nombre de couleurs réduit). */
export interface ColorGroup {
	/** Indices (0-based) dans la palette du serveur. */
	members: number[];
	/** Couleur représentative (moyenne des membres), « #rrggbb ». */
	hex: string;
}

/**
 * Réduit la palette à `count` couleurs par fusion des deux groupes les plus
 * proches (ΔE76), l'ordre de la palette étant conservé (Orca : « nombre de
 * couleurs » du dialogue). `count` ≥ palette.length → une couleur par groupe.
 */
export function groupPalette(palette: readonly string[], count: number): ColorGroup[] {
	let groups: ColorGroup[] = palette.map((c, i) => ({
		members: [i],
		hex: paletteHex(c) ?? '#808080'
	}));
	const target = Math.max(1, Math.min(palette.length, Math.round(count)));
	while (groups.length > target) {
		let bi = 0;
		let bj = 1;
		let bd = Infinity;
		for (let i = 0; i < groups.length; i++)
			for (let j = i + 1; j < groups.length; j++) {
				const d = colorDistance(groups[i].hex, groups[j].hex);
				if (d < bd) {
					bd = d;
					bi = i;
					bj = j;
				}
			}
		const members = [...groups[bi].members, ...groups[bj].members].sort((a, b) => a - b);
		const sum = members.reduce<[number, number, number]>(
			(acc, m) => {
				const c = rgb(palette[m]);
				return [acc[0] + c[0], acc[1] + c[1], acc[2] + c[2]];
			},
			[0, 0, 0]
		);
		const merged: ColorGroup = {
			members,
			hex: toHex([sum[0] / members.length, sum[1] / members.length, sum[2] / members.length])
		};
		groups = groups.filter((_, k) => k !== bi && k !== bj);
		groups.push(merged);
		groups.sort((a, b) => a.members[0] - b.members[0]);
	}
	return groups;
}

/** Destination d'une couleur : nouveau filament, ou slot existant (0-based). */
export type ColorTarget = { kind: 'new' } | { kind: 'slot'; slot: number };

/**
 * Affectation par défaut : `append` = un nouveau filament par couleur (réglage
 * par défaut d'Orca, celui du CLI) dans la limite de MAX_FILAMENTS ; `match` =
 * filament existant le plus proche.
 */
export function defaultTargets(
	groups: readonly ColorGroup[],
	slotColours: readonly string[],
	mode: 'append' | 'match'
): ColorTarget[] {
	let free = MAX_FILAMENTS - slotColours.length;
	return groups.map((g) => {
		if (mode === 'append' && free > 0) {
			free--;
			return { kind: 'new' };
		}
		return { kind: 'slot', slot: nearestSlot(g.hex, slotColours) };
	});
}

/** Plan d'application du dialogue. */
export interface ObjColorPlan {
	/** Couleurs des filaments à ajouter, dans l'ordre (slots N+1…). */
	newColours: string[];
	/** Filament n du serveur (index n-1) → extrudeur final (1-based). */
	extruderOf: number[];
}

/** Calcule les nouveaux slots et la table de renumérotation. */
export function planObjColors(
	palette: readonly string[],
	groups: readonly ColorGroup[],
	targets: readonly ColorTarget[],
	slotCount: number
): ObjColorPlan {
	const newColours: string[] = [];
	const extruderOf = palette.map(() => 1);
	groups.forEach((g, gi) => {
		const t = targets[gi] ?? { kind: 'new' };
		let extruder: number;
		if (t.kind === 'new' && slotCount + newColours.length < MAX_FILAMENTS) {
			newColours.push(g.hex);
			extruder = slotCount + newColours.length;
		} else {
			const slot = t.kind === 'slot' ? t.slot : slotCount - 1;
			extruder = Math.min(Math.max(slot, 0), Math.max(slotCount - 1, 0)) + 1;
		}
		for (const m of g.members) extruderOf[m] = extruder;
	});
	return { newColours, extruderOf };
}

/** Extrudeur final d'un filament du serveur (hors palette : inchangé). */
export function mapExtruder(plan: ObjColorPlan, filament: number): number {
	return filament >= 1 && filament <= plan.extruderOf.length
		? plan.extruderOf[filament - 1]
		: filament;
}

/**
 * Peinture MMU renumérotée (chaînes hex Orca par facette). Les triangles
 * subdivisés (non décodables à la facette) sont conservés tels quels.
 */
export function remapMmu(
	facets: Record<string, string>,
	plan: ObjColorPlan
): Record<string, string> {
	const out: Record<string, string> = {};
	for (const [tri, hex] of Object.entries(facets)) {
		const state = decodeFacet(hex);
		if (state > 0) out[tri] = encodeFacet(mapExtruder(plan, state));
		else if (state < 0) out[tri] = hex;
	}
	return out;
}

/** Nom du `mtllib` déclaré par un OBJ (première occurrence). */
export function mtllibName(objText: string): string | null {
	const m = /^\s*mtllib\s+(.+?)\s*$/m.exec(objText);
	return m ? m[1] : null;
}

const base = (name: string) => name.replace(/\.[^.]+$/, '').toLowerCase();

/**
 * MTL à joindre à un OBJ parmi les fichiers déposés ensemble : celui que
 * déclare `mtllib`, sinon même nom de base, sinon le seul MTL déposé.
 */
export function pickMtl(
	objName: string,
	objText: string | null,
	mtlNames: readonly string[]
): string | null {
	if (!mtlNames.length) return null;
	const declared = objText ? mtllibName(objText) : null;
	if (declared) {
		const leaf = declared.split(/[\\/]/).pop()!.toLowerCase();
		const hit = mtlNames.find((n) => n.toLowerCase() === leaf);
		if (hit) return hit;
	}
	const same = mtlNames.find((n) => base(n) === base(objName));
	if (same) return same;
	return mtlNames.length === 1 ? mtlNames[0] : null;
}

/** Fichier MTL (accompagne un OBJ, pas un modèle importable). */
export function isMtl(name: string): boolean {
	return /\.mtl$/i.test(name);
}
