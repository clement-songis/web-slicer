// Conversion d'unités à l'import (Orca : Model::looks_like_saved_in_meters /
// looks_like_imperial_units, Model.cpp:830-872 ; Plater.cpp:6693) et entrées
// « Convert from inches/meters » / « Restore to inches/meters » du menu objet
// (GUI_Factories.cpp:1225). Côté web, la conversion est un facteur d'échelle
// uniforme (×25,4 ou ×1000) : visible et réversible dans le panneau d'échelle.
import type { SceneMesh } from '$lib/scene/mesh';
import type { ObjectMenuEntry, ObjectMenuItem } from '$lib/scene/menus';
import { get } from 'svelte/store';
import { locale, registerTranslations } from '$lib/i18n';

export type UnitSource = 'inch' | 'meter';

/** Seuils d'Orca (mm³) : 0,2³ m → mètres probables, 2³ in → pouces probables. */
export const VOLUME_THRESHOLD_METERS = 0.008;
export const VOLUME_THRESHOLD_INCHES = 8;
export const UNIT_FACTORS: Record<UnitSource, number> = { inch: 25.4, meter: 1000 };

/** Volume (mm³, valeur absolue) d'un maillage fermé, par le théorème de divergence. */
export function meshVolume(mesh: Pick<SceneMesh, 'positions' | 'indices'>): number {
	const p = mesh.positions;
	const idx = mesh.indices;
	let six = 0;
	for (let i = 0; i + 2 < idx.length; i += 3) {
		const a = idx[i] * 3;
		const b = idx[i + 1] * 3;
		const c = idx[i + 2] * 3;
		six +=
			p[a] * (p[b + 1] * p[c + 2] - p[b + 2] * p[c + 1]) -
			p[a + 1] * (p[b] * p[c + 2] - p[b + 2] * p[c]) +
			p[a + 2] * (p[b] * p[c + 1] - p[b + 1] * p[c]);
	}
	return Math.abs(six) / 6;
}

/** Volume après échelle (produit des facteurs). */
export function scaledVolume(volume: number, scale?: readonly number[]): number {
	const [x, y, z] = scale ?? [1, 1, 1];
	return volume * Math.abs(x * y * z);
}

/**
 * Unité probable d'origine selon Orca : mètres testés d'abord (< 0,008 mm³),
 * puis pouces (< 8 mm³). Volume nul (maillage ouvert ou plat) : pas de verdict.
 */
export function detectUnits(volume: number): UnitSource | null {
	if (!(volume > 0)) return null;
	if (volume < VOLUME_THRESHOLD_METERS) return 'meter';
	if (volume < VOLUME_THRESHOLD_INCHES) return 'inch';
	return null;
}

/** Objet déjà converti ? (échelle uniforme ≈ 25,4 ou ≈ 1000, au signe près). */
export function convertedFrom(scale?: readonly number[]): UnitSource | null {
	if (!scale) return null;
	const [x, y, z] = scale.map(Math.abs);
	const uniform = Math.abs(x - y) < 1e-6 * x && Math.abs(x - z) < 1e-6 * x;
	if (!uniform) return null;
	for (const unit of ['inch', 'meter'] as const)
		if (Math.abs(x - UNIT_FACTORS[unit]) < 1e-6 * UNIT_FACTORS[unit]) return unit;
	return null;
}

/** Échelle après conversion (ou retour) : facteur appliqué en gardant les signes (miroirs). */
export function convertScale(
	scale: readonly number[] | undefined,
	unit: UnitSource,
	restore = false
): [number, number, number] {
	const f = restore ? 1 / UNIT_FACTORS[unit] : UNIT_FACTORS[unit];
	const [x, y, z] = scale ?? [1, 1, 1];
	return [x * f, y * f, z * f];
}

export const UNIT_ACTIONS = {
	fromInch: 'object.convertFromInches',
	fromMeter: 'object.convertFromMeters',
	toInch: 'object.restoreToInches',
	toMeter: 'object.restoreToMeters'
} as const;

registerTranslations('fr', {
	'Convert from inches': 'Convertir depuis les pouces (×25,4)',
	'Convert from meters': 'Convertir depuis les mètres (×1000)',
	'Restore to inches': 'Revenir aux pouces',
	'Restore to meters': 'Revenir aux mètres'
});

/** Entrées d'unités du menu objet (Orca : selon l'état de conversion des volumes). */
export function unitMenuItems(scale?: readonly number[]): ObjectMenuEntry[] {
	const from = convertedFrom(scale);
	if (from === 'inch') return [{ label: 'Restore to inches', action: UNIT_ACTIONS.toInch }];
	if (from === 'meter') return [{ label: 'Restore to meters', action: UNIT_ACTIONS.toMeter }];
	return [
		{ label: 'Convert from inches', action: UNIT_ACTIONS.fromInch },
		{ label: 'Convert from meters', action: UNIT_ACTIONS.fromMeter }
	];
}

/** Menu objet complété des entrées d'unités, après « Scale to build volume ». */
export function withUnitItems(
	items: readonly ObjectMenuEntry[],
	scale?: readonly number[]
): ObjectMenuEntry[] {
	const isScale = (e: ObjectMenuEntry) => e !== 'separator' && e.action === 'object.scaleToBed';
	// Menu rangé en sections (phase 25) : insertion dans la section qui porte
	// « Scale to build volume » (Placement).
	const section = items.findIndex(
		(e) => e !== 'separator' && !!e.items && e.items.some((sub) => isScale(sub))
	);
	if (section >= 0) {
		const host = items[section] as ObjectMenuItem;
		const subs = [...(host.items ?? [])];
		subs.splice(subs.findIndex(isScale) + 1, 0, ...(unitMenuItems(scale) as ObjectMenuItem[]));
		const out = [...items];
		out[section] = { ...host, items: subs };
		return out;
	}
	const out = [...items];
	const at = out.findIndex(isScale);
	out.splice(at >= 0 ? at + 1 : out.length, 0, ...unitMenuItems(scale));
	return out;
}

/** Action d'unités → (unité, retour ?), ou null. */
export function unitActionOf(action: string): { unit: UnitSource; restore: boolean } | null {
	switch (action) {
		case UNIT_ACTIONS.fromInch:
			return { unit: 'inch', restore: false };
		case UNIT_ACTIONS.fromMeter:
			return { unit: 'meter', restore: false };
		case UNIT_ACTIONS.toInch:
			return { unit: 'inch', restore: true };
		case UNIT_ACTIONS.toMeter:
			return { unit: 'meter', restore: true };
		default:
			return null;
	}
}

/** Dimensions lisibles (« 1 × 1 × 1 mm »). */
export function formatSize(size: readonly number[]): string {
	const f = (v: number) =>
		v >= 100 ? v.toFixed(0) : v >= 1 ? v.toFixed(1).replace(/\.0$/, '') : v.toPrecision(2);
	const sep = get(locale) === 'fr' ? ',' : '.';
	return `${size.map((v) => f(v).replace('.', sep)).join(' × ')} mm`;
}
