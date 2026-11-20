// Options d'arrangement GUI (lot 34) : fusion des valeurs mémorisées avec
// la réponse de GET /api/projects/{id}/arrange/options (GLCanvas3D::_render_arrange_menu
// d'Orca : bornes du slider d'espacement, Reset = ArrangeSettings + structure
// d'imprimante, case « zone de calibration » seulement sur BBL + scan_first_layer).
// Pur → testable ; le panneau (ArrangeOptionsPanel) et la page orchestrent.
import type { ArrangeOptionsInfo } from '$lib/api/types';
import type { ArrangePreferences } from '$lib/settings/preferences';

export type SavedArrangeOptions = ArrangePreferences;

/**
 * Options mémorisées ramenées aux bornes du serveur : espacement borné au
 * [min_spacing, 100] du slider en impression séquentielle (Orca force
 * l'affichage à max(spacing, min_spacing)), 0 = automatique sinon.
 */
export function effectiveOptions(
	saved: SavedArrangeOptions,
	info: ArrangeOptionsInfo
): SavedArrangeOptions {
	return {
		...saved,
		spacing: clampSpacing(saved.spacing, info)
	};
}

/** Espacement borné selon le mode d'impression (0 = auto hors séquentiel). */
export function clampSpacing(value: number, info: ArrangeOptionsInfo): number {
	if (!Number.isFinite(value)) return spacingFloor(info);
	const bounded = Math.min(100, Math.max(0, value));
	return Math.max(spacingFloor(info), bounded);
}

/** Borne basse du slider : min_spacing en séquentiel, 0 (auto) sinon. */
export function spacingFloor(info: ArrangeOptionsInfo): number {
	return info.is_seq_print ? Math.max(0, info.min_spacing) : 0;
}

/**
 * Bouton « Réinitialiser » d'Orca : ArrangeSettings par défaut (espacement 0,
 * rotation off, multi-matériaux on, calibration on), alignement Y = structure
 * i3 — déjà calculés par le serveur dans `info.defaults` — puis bornage.
 */
export function resetOptions(info: ArrangeOptionsInfo): SavedArrangeOptions {
	return {
		spacing: clampSpacing(info.defaults.spacing, info),
		enableRotation: info.defaults.enable_rotation,
		allowMultiMaterialsOnSamePlate: info.defaults.allow_multi_materials_on_same_plate,
		avoidExtrusionCaliRegion: info.defaults.avoid_extrusion_cali_region,
		alignToYAxis: info.defaults.align_to_y_axis
	};
}

/** Corps `options` du POST /arrange (AlignSettings envoyés à l'arrangeur). */
export function optionsBody(saved: SavedArrangeOptions) {
	return {
		spacing: saved.spacing,
		enable_rotation: saved.enableRotation,
		allow_multi_materials_on_same_plate: saved.allowMultiMaterialsOnSamePlate,
		avoid_extrusion_cali_region: saved.avoidExtrusionCaliRegion,
		// Le serveur ignore l'alignement Y quand la rotation est active ; on
		// l'envoie déjà éteint pour refléter la case grisée d'Orca.
		align_to_y_axis: saved.alignToYAxis && !saved.enableRotation
	};
}
