// Changements de couleur par couche (curseur de couches d'Orca, IMSlider /
// TickCodeInfo) : « + » ajoute un changement de couleur à la couche courante
// de l'Aperçu, « − » supprime le code de cette couche (GLCanvas3D on_char →
// EVT_GLCANVAS_EDIT_COLOR_CHANGE). Les codes vivent dans le plateau
// (`plates[i].customGcodes`, lus par le serveur : slicing.rs
// scene_custom_gcodes) et s'appliquent au prochain tranchage. Pur.

import { tr } from '$lib/i18n';

export type CustomGcodeType =
	'color_change' | 'pause_print' | 'custom' | 'template' | 'tool_change';

/** Un G-code personnalisé à une hauteur d'impression (format scène). */
export interface PlateCustomGcode {
	printZ: number;
	type: CustomGcodeType;
	extruder?: number;
	color?: string;
	extra?: string;
}

/** Tolérance de comparaison des cotes Z (mm). */
export const Z_EPS = 1e-4;

export function codeIndexAtZ(list: readonly PlateCustomGcode[], z: number): number {
	return list.findIndex((g) => Math.abs(g.printZ - z) < Z_EPS);
}

function hexToRgb(hex: string): [number, number, number] | null {
	const m = /^#?([0-9a-f]{6})/i.exec(hex.trim());
	if (!m) return null;
	const n = parseInt(m[1], 16);
	return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

function rgbToHue([r, g, b]: [number, number, number]): number {
	const [R, G, B] = [r / 255, g / 255, b / 255];
	const max = Math.max(R, G, B);
	const d = max - Math.min(R, G, B);
	if (d === 0) return 0;
	let h = max === R ? ((G - B) / d) % 6 : max === G ? (B - R) / d + 2 : (R - G) / d + 4;
	h *= 60;
	return h < 0 ? h + 360 : h;
}

function hsvToHex(h: number, s: number, v: number): string {
	const c = v * s;
	const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
	const m = v - c;
	const [r, g, b] =
		h < 60
			? [c, x, 0]
			: h < 120
				? [x, c, 0]
				: h < 180
					? [0, c, x]
					: h < 240
						? [0, x, c]
						: h < 300
							? [x, 0, c]
							: [c, 0, x];
	const hex = (u: number) =>
		Math.round((u + m) * 255)
			.toString(16)
			.padStart(2, '0');
	return `#${hex(r)}${hex(g)}${hex(b)}`.toUpperCase();
}

/**
 * Couleur proposée pour un nouveau changement de couleur (Orca
 * TickCodeInfo::get_color_for_tick, mode mono-extrudeur) : teinte « opposée »
 * (+65°) de la couleur en cours à cette hauteur — dernier changement de couleur
 * en dessous, sinon la couleur du filament. Orca tire saturation et valeur au
 * hasard (0,65–1) ; ici 0,85 fixe, pour un résultat reproductible.
 */
export function colorChangeColor(
	list: readonly PlateCustomGcode[],
	z: number,
	filamentColor: string
): string {
	const below = list
		.filter((g) => g.type === 'color_change' && g.printZ < z - Z_EPS && g.color)
		.sort((a, b) => b.printZ - a.printZ)[0];
	const base = hexToRgb(below?.color ?? filamentColor) ?? [0, 174, 66];
	const h = (rgbToHue(base) + 65) % 360;
	return hsvToHex(h, 0.85, 0.85);
}

/** Résultat d'une édition au clavier (« + » / « − »). */
export interface ColorChangeEdit {
	list: PlateCustomGcode[];
	changed: boolean;
	message: string;
}

/**
 * « + » : ajoute un changement de couleur (filament 1) à `z` s'il n'y a pas
 * déjà un code à cette couche. « − » : supprime le code de la couche (quel
 * que soit son type, comme la suppression de repère d'Orca). Liste triée par Z.
 */
export function editColorChangeAt(
	list: readonly PlateCustomGcode[],
	key: '+' | '-',
	z: number,
	layerLabel: string,
	filamentColor: string
): ColorChangeEdit {
	const at = codeIndexAtZ(list, z);
	if (key === '+') {
		if (at >= 0)
			return {
				list: [...list],
				changed: false,
				message: tr('Layer {layer} already has a custom code.', { layer: layerLabel })
			};
		const color = colorChangeColor(list, z, filamentColor);
		const next = [...list, { printZ: z, type: 'color_change' as const, extruder: 1, color }].sort(
			(a, b) => a.printZ - b.printZ
		);
		return {
			list: next,
			changed: true,
			message: tr('Color change added to layer {layer} — reslice (Ctrl+R) to apply it.', {
				layer: layerLabel
			})
		};
	}
	if (at < 0)
		return {
			list: [...list],
			changed: false,
			message: tr('No custom code at layer {layer}.', { layer: layerLabel })
		};
	const removed = list[at];
	return {
		list: list.filter((_, i) => i !== at),
		changed: true,
		message: tr('{what} removed from layer {layer} — reslice (Ctrl+R) to apply it.', {
			what: tr(removed.type === 'color_change' ? 'Color Change' : 'Custom code'),
			layer: layerLabel
		})
	};
}

/** Repère du curseur de couches (index de couche + type), comme `layerTicks`. */
export interface LayerTick {
	layer: number;
	type: string;
}

/**
 * Repères des codes du plateau pas encore présents dans le résultat tranché
 * (ajoutés depuis) : affichés en pointillé sur le curseur jusqu'au tranchage.
 */
export function pendingTicks(
	codes: readonly PlateCustomGcode[],
	layers: readonly { z: number }[],
	sliced: readonly LayerTick[]
): LayerTick[] {
	const out: LayerTick[] = [];
	for (const code of codes) {
		const layer = layers.findIndex((l) => Math.abs(l.z - code.printZ) < Z_EPS);
		if (layer < 0) continue;
		if (sliced.some((t) => t.layer === layer && t.type === code.type)) continue;
		out.push({ layer, type: code.type });
	}
	return out;
}

/**
 * Repères tranchés restylés par les codes posés sur le plateau : Orca 2.4.1
 * écrit `;CUSTOM_GCODE` pour un changement de couleur
 * (GCode.cpp emit_custom_gcode_per_print_z, émission couleur retirée en BBS :
 * `#if 0`), donc le marqueur du moteur revient « custom » ; le curseur
 * d'Orca, lui, lit `custom_gcode_per_print_z` du modèle — le type posé fait
 * foi. Un code du plateau dont le Z correspond à la couche du marqueur
 * remplace son type ; les marqueurs sans code posé (G-code ouvert, outil…)
 * restent tels quels.
 */
export function withAuthoredTypes(
	markers: readonly LayerTick[],
	codes: readonly PlateCustomGcode[],
	layers: readonly { z: number }[]
): LayerTick[] {
	return markers.map((marker) => {
		const z = layers[marker.layer]?.z;
		if (z === undefined) return marker;
		const code = codes.find((g) => Math.abs(g.printZ - z) < Z_EPS);
		return code ? { layer: marker.layer, type: code.type } : marker;
	});
}
