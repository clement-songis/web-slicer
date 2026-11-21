// Menu du curseur de couches de l'Aperçu (Orca IMSlider::render_add_menu /
// render_edit_menu, TickCodeInfo::add_tick) : sur la couche de la poignée
// active, ajouter une pause, un G-code personnalisé, un modèle personnalisé
// (`template_custom_gcode` non vide), un changement de filament (plusieurs
// filaments) ; sur une couche qui porte déjà un code, le modifier (G-code
// perso, filament) ou le supprimer. Les codes vivent dans le plateau
// (`plates[i].customGcodes`), appliqués au prochain tranchage. Pur.
import {
	Z_EPS,
	codeIndexAtZ,
	colorChangeColor,
	type CustomGcodeType,
	type PlateCustomGcode
} from './color-changes';
import { tr } from '$lib/i18n';

/** Entrée du menu (libellés = traductions françaises d'Orca). */
export interface LayerCodeMenuItem {
	action:
		| 'add-pause'
		| 'add-custom'
		| 'add-template'
		| 'add-color'
		| 'tool-change'
		| 'edit-custom'
		| 'delete';
	label: string;
	/** Infobulle d'Orca (show_tooltip). */
	hint?: string;
	/** Changement de filament : filament visé (1-based) et sa couleur. */
	extruder?: number;
	color?: string;
	disabled?: boolean;
}

/** Contexte du menu : liste du plateau, couche visée, filaments. */
export interface LayerCodeMenuContext {
	codes: readonly PlateCustomGcode[];
	z: number;
	/** Couleurs des filaments (slot i ↔ filament i + 1). */
	filamentColors: readonly string[];
	/** `template_custom_gcode` de l'imprimante (entrée « modèle » si non vide). */
	templateGcode?: string;
	/** Impression objet par objet : Orca grise les ajouts (dmSequentialFffPrint). */
	sequential?: boolean;
}

/** Libellé humain d'un type de code (repères, messages) — clé i18n anglaise, traduire avec `tr`. */
export const CODE_TYPE_LABELS: Record<CustomGcodeType, string> = {
	color_change: 'Color Change',
	pause_print: 'Pause',
	custom: 'Custom G-code',
	template: 'Custom Template',
	tool_change: 'Filament change'
};

const DELETE_LABELS: Record<CustomGcodeType, string> = {
	color_change: 'Delete Color Change',
	pause_print: 'Delete Pause',
	custom: 'Delete Custom G-code',
	template: 'Delete Custom Template',
	tool_change: 'Delete Filament Change'
};

function filamentItems(ctx: LayerCodeMenuContext, current?: number): LayerCodeMenuItem[] {
	return ctx.filamentColors.map((color, i) => ({
		action: 'tool-change' as const,
		label: tr('Change filament → Filament {n}', { n: i + 1 }),
		hint: tr('Change filament at the beginning of this layer.'),
		extruder: i + 1,
		color,
		disabled: current === i + 1
	}));
}

/** Entrées du menu pour la couche `ctx.z` (ajout ou édition, comme Orca). */
export function layerCodeMenu(ctx: LayerCodeMenuContext): LayerCodeMenuItem[] {
	const at = codeIndexAtZ(ctx.codes, ctx.z);
	if (at >= 0) {
		const code = ctx.codes[at];
		const items: LayerCodeMenuItem[] = [];
		if (code.type === 'custom')
			items.push({ action: 'edit-custom', label: tr('Edit Custom G-code') });
		if (code.type === 'tool_change' && ctx.filamentColors.length > 1)
			items.push(...filamentItems(ctx, code.extruder));
		items.push({ action: 'delete', label: tr(DELETE_LABELS[code.type]) });
		return items;
	}
	const disabled = !!ctx.sequential;
	const items: LayerCodeMenuItem[] = [
		{
			action: 'add-pause',
			label: tr('Add Pause'),
			hint: tr('Insert a pause command at the beginning of this layer.'),
			disabled
		},
		{
			action: 'add-custom',
			label: tr('Add Custom G-code'),
			hint: tr('Insert custom G-code at the beginning of this layer.'),
			disabled
		}
	];
	if (ctx.templateGcode?.trim())
		items.push({
			action: 'add-template',
			label: tr('Add Custom Template'),
			hint: tr('Insert the printer’s template G-code at the beginning of this layer.'),
			disabled
		});
	// En plus d'Orca (icône « + » séparée sur la poignée) : le changement de
	// couleur figure dans le même menu, à côté des autres codes.
	items.push({
		action: 'add-color',
		label: tr('Add color change'),
		hint: tr('Pause and color change at the beginning of this layer.')
	});
	if (ctx.filamentColors.length > 1) items.push(...filamentItems(ctx));
	return items;
}

/** Résultat d'une édition de la liste. */
export interface LayerCodeEdit {
	list: PlateCustomGcode[];
	changed: boolean;
	message: string;
}

const sortByZ = (list: PlateCustomGcode[]) => list.sort((a, b) => a.printZ - b.printZ);

/**
 * Applique une entrée du menu à la couche `z` (`layerLabel` = numéro affiché).
 * `extra` = texte du G-code personnalisé (add-custom / edit-custom).
 */
export function applyLayerCode(
	codes: readonly PlateCustomGcode[],
	item: Pick<LayerCodeMenuItem, 'action' | 'extruder'>,
	z: number,
	layerLabel: string,
	opts: { extra?: string; filamentColors?: readonly string[] } = {}
): LayerCodeEdit {
	const at = codeIndexAtZ(codes, z);
	const list = codes.map((g) => ({ ...g }));
	const suffix = ' ' + tr('— reslice (Ctrl+R) to apply it.');
	const unchanged = (message: string): LayerCodeEdit => ({ list, changed: false, message });
	const colors = opts.filamentColors ?? [];
	switch (item.action) {
		case 'delete': {
			if (at < 0) return unchanged(tr('No code at layer {layer}.', { layer: layerLabel }));
			const removed = list[at];
			list.splice(at, 1);
			return {
				list,
				changed: true,
				message:
					tr('{what} removed from layer {layer}', {
						what: tr(CODE_TYPE_LABELS[removed.type]),
						layer: layerLabel
					}) + suffix
			};
		}
		case 'edit-custom': {
			const text = opts.extra?.trim();
			if (at < 0 || list[at].type !== 'custom' || !text)
				return unchanged(tr('Empty custom G-code: nothing to save.'));
			if (list[at].extra === text) return unchanged(tr('Custom G-code unchanged.'));
			list[at] = { ...list[at], extra: text };
			return {
				list,
				changed: true,
				message: tr('Custom G-code of layer {layer} modified', { layer: layerLabel }) + suffix
			};
		}
		case 'tool-change': {
			const extruder = item.extruder ?? 1;
			const color = colors[extruder - 1];
			// Orca switch_code_for_tick : un changement de filament existant change
			// simplement de filament.
			if (at >= 0) {
				if (list[at].type !== 'tool_change')
					return unchanged(tr('Layer {layer} already has a custom code.', { layer: layerLabel }));
				if (list[at].extruder === extruder) return unchanged(tr('Filament already selected.'));
				list[at] = { ...list[at], extruder, ...(color ? { color } : {}) };
				return {
					list,
					changed: true,
					message:
						tr('Layer {layer}: switch to filament {n}', { layer: layerLabel, n: extruder }) + suffix
				};
			}
			list.push({ printZ: z, type: 'tool_change', extruder, ...(color ? { color } : {}) });
			return {
				list: sortByZ(list),
				changed: true,
				message:
					tr('Filament change (filament {n}) added to layer {layer}', {
						n: extruder,
						layer: layerLabel
					}) + suffix
			};
		}
		default: {
			if (at >= 0)
				return unchanged(tr('Layer {layer} already has a custom code.', { layer: layerLabel }));
			let code: PlateCustomGcode;
			if (item.action === 'add-pause') code = { printZ: z, type: 'pause_print', extruder: 1 };
			else if (item.action === 'add-template') code = { printZ: z, type: 'template', extruder: 1 };
			else if (item.action === 'add-color')
				code = {
					printZ: z,
					type: 'color_change',
					extruder: 1,
					color: colorChangeColor(codes, z, colors[0] ?? '#00AE42')
				};
			else {
				const text = opts.extra?.trim();
				if (!text) return unchanged(tr('Empty custom G-code: nothing to add.'));
				code = { printZ: z, type: 'custom', extruder: 1, extra: text };
			}
			list.push(code);
			return {
				list: sortByZ(list),
				changed: true,
				message:
					tr('{what} added to layer {layer}', {
						what: tr(CODE_TYPE_LABELS[code.type]),
						layer: layerLabel
					}) + suffix
			};
		}
	}
}

/** Code posé à la cote `z` (édition du G-code perso : texte actuel). */
export function codeAt(codes: readonly PlateCustomGcode[], z: number): PlateCustomGcode | null {
	return codes.find((g) => Math.abs(g.printZ - z) < Z_EPS) ?? null;
}

/**
 * Codes de changement de filament après le retrait du slot `removed` (0-based),
 * fusionné dans `target` (0-based, numérotation d'avant le retrait) —
 * Plater::on_filaments_delete d'Orca : les changements vers le filament retiré
 * visent le filament de remplacement, les suivants descendent d'un cran. Sans
 * `target` (simple suppression), les changements vers le filament retiré
 * disparaissent.
 */
export function remapCodesAfterFilamentRemoval(
	codes: readonly PlateCustomGcode[],
	removed: number,
	target: number | null
): PlateCustomGcode[] {
	const shifted = (extruder: number) => (extruder - 1 > removed ? extruder - 1 : extruder);
	return codes.flatMap((g) => {
		if (g.type !== 'tool_change' || !g.extruder) return [{ ...g }];
		if (g.extruder - 1 === removed) {
			if (target === null) return [];
			return [{ ...g, extruder: shifted(target + 1) }];
		}
		return [{ ...g, extruder: shifted(g.extruder) }];
	});
}
