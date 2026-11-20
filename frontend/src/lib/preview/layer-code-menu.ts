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

/** Libellé humain d'un type de code (repères, messages). */
export const CODE_TYPE_LABELS: Record<CustomGcodeType, string> = {
	color_change: 'Changement de couleur',
	pause_print: 'Pause',
	custom: 'G-code personnalisé',
	template: 'Modèle personnalisé',
	tool_change: 'Changement de filament'
};

const DELETE_LABELS: Record<CustomGcodeType, string> = {
	color_change: 'Supprimer le changement de couleur',
	pause_print: 'Supprimer la pause',
	custom: 'Supprimer le G-code personnalisé',
	template: 'Supprimer le modèle personnalisé',
	tool_change: 'Supprimer le changement de filament'
};

function filamentItems(ctx: LayerCodeMenuContext, current?: number): LayerCodeMenuItem[] {
	return ctx.filamentColors.map((color, i) => ({
		action: 'tool-change' as const,
		label: `Changer de filament → Filament ${i + 1}`,
		hint: 'Changez le filament au début de cette couche.',
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
			items.push({ action: 'edit-custom', label: 'Modifier le G-code personnalisé' });
		if (code.type === 'tool_change' && ctx.filamentColors.length > 1)
			items.push(...filamentItems(ctx, code.extruder));
		items.push({ action: 'delete', label: DELETE_LABELS[code.type] });
		return items;
	}
	const disabled = !!ctx.sequential;
	const items: LayerCodeMenuItem[] = [
		{
			action: 'add-pause',
			label: 'Ajouter une pause',
			hint: 'Insérez une commande de pause au début de cette couche.',
			disabled
		},
		{
			action: 'add-custom',
			label: 'Ajouter un G-code personnalisé',
			hint: 'Insérez un G-code personnalisé au début de cette couche.',
			disabled
		}
	];
	if (ctx.templateGcode?.trim())
		items.push({
			action: 'add-template',
			label: 'Ajouter un modèle personnalisé',
			hint: 'Insérez le G-code modèle de l’imprimante au début de cette couche.',
			disabled
		});
	// En plus d'Orca (icône « + » séparée sur la poignée) : le changement de
	// couleur figure dans le même menu, à côté des autres codes.
	items.push({
		action: 'add-color',
		label: 'Ajouter un changement de couleur',
		hint: 'Pause et changement de couleur au début de cette couche.'
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
	const suffix = ' — retranchez (Ctrl+R) pour l’appliquer.';
	const unchanged = (message: string): LayerCodeEdit => ({ list, changed: false, message });
	const colors = opts.filamentColors ?? [];
	switch (item.action) {
		case 'delete': {
			if (at < 0) return unchanged(`Aucun code à la couche ${layerLabel}.`);
			const removed = list[at];
			list.splice(at, 1);
			return {
				list,
				changed: true,
				message: `${CODE_TYPE_LABELS[removed.type]} retiré de la couche ${layerLabel}${suffix}`
			};
		}
		case 'edit-custom': {
			const text = opts.extra?.trim();
			if (at < 0 || list[at].type !== 'custom' || !text)
				return unchanged('G-code personnalisé vide : rien à enregistrer.');
			if (list[at].extra === text) return unchanged('G-code personnalisé inchangé.');
			list[at] = { ...list[at], extra: text };
			return {
				list,
				changed: true,
				message: `G-code personnalisé de la couche ${layerLabel} modifié${suffix}`
			};
		}
		case 'tool-change': {
			const extruder = item.extruder ?? 1;
			const color = colors[extruder - 1];
			// Orca switch_code_for_tick : un changement de filament existant change
			// simplement de filament.
			if (at >= 0) {
				if (list[at].type !== 'tool_change')
					return unchanged(`La couche ${layerLabel} a déjà un code personnalisé.`);
				if (list[at].extruder === extruder) return unchanged('Filament déjà choisi.');
				list[at] = { ...list[at], extruder, ...(color ? { color } : {}) };
				return {
					list,
					changed: true,
					message: `Couche ${layerLabel} : passage au filament ${extruder}${suffix}`
				};
			}
			list.push({ printZ: z, type: 'tool_change', extruder, ...(color ? { color } : {}) });
			return {
				list: sortByZ(list),
				changed: true,
				message: `Changement de filament (filament ${extruder}) ajouté à la couche ${layerLabel}${suffix}`
			};
		}
		default: {
			if (at >= 0) return unchanged(`La couche ${layerLabel} a déjà un code personnalisé.`);
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
				if (!text) return unchanged('G-code personnalisé vide : rien à ajouter.');
				code = { printZ: z, type: 'custom', extruder: 1, extra: text };
			}
			list.push(code);
			return {
				list: sortByZ(list),
				changed: true,
				message: `${CODE_TYPE_LABELS[code.type]} ajouté à la couche ${layerLabel}${suffix}`
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
