// Comparaison et persistance des brouillons de presets. Les valeurs affichées
// viennent de `/resolved`, mais seules les différences réellement saisies sont
// renvoyées : un preset système sert de parent sans jamais être modifié, et un
// preset utilisateur conserve ses surcharges brutes existantes.

import { PARAMS } from '../../generated/params';
import { UI_LAYOUT, type PresetKind } from '../../generated/ui-layout';

export type PresetValues = Record<string, unknown>;

/** Options pourcentage pures (coPercent/coPercents) : `/resolved` porte 15, l'UI « 15% ». */
export function isPercentKey(key: string): boolean {
	const type = PARAMS[key]?.type;
	return type === 'coPercent' || type === 'coPercents';
}

/** Copie JSON : les tableaux édités ne doivent jamais muter la base résolue. */
export function clonePresetValues(values: PresetValues): PresetValues {
	return JSON.parse(JSON.stringify(values)) as PresetValues;
}

function scalarEqual(a: unknown, b: unknown, percent = false): boolean {
	if (a === b) return true;
	if (typeof a === 'string' || typeof b === 'string') {
		let as = String(a).trim();
		let bs = String(b).trim();
		if (as === bs) return true;
		// Pourcentage pur : le suffixe est implicite (15 ≡ « 15% »).
		if (percent) {
			as = as.replace(/%$/, '');
			bs = bs.replace(/%$/, '');
		}
		// Les widgets texte peuvent produire « 20 » où l'API résolue porte 20.
		// Le suffixe % reste significatif et doit être identique des deux côtés.
		if (as.endsWith('%') !== bs.endsWith('%')) return false;
		const an = Number(as.replace(/%$/, ''));
		const bn = Number(bs.replace(/%$/, ''));
		return as !== '' && bs !== '' && Number.isFinite(an) && Number.isFinite(bn) && an === bn;
	}
	return false;
}

/** Égalité structurelle, tolérante au formatage des champs numériques. */
export function settingEqual(a: unknown, b: unknown, percent = false): boolean {
	if (Array.isArray(a) || Array.isArray(b)) {
		if (!Array.isArray(a) || !Array.isArray(b) || a.length !== b.length) return false;
		return a.every((value, index) => settingEqual(value, b[index], percent));
	}
	return scalarEqual(a, b, percent);
}

/** Diff d'un brouillon contre la configuration résolue au moment de son ouverture. */
export function changedPresetValues(resolved: PresetValues, current: PresetValues): PresetValues {
	const changed: PresetValues = {};
	const keys = new Set([...Object.keys(resolved), ...Object.keys(current)]);
	for (const key of keys) {
		if (resolved[key] === undefined && isWidgetFill(key, current[key])) continue;
		if (!settingEqual(resolved[key], current[key], isPercentKey(key))) changed[key] = current[key];
	}
	return changed;
}

/**
 * Valeur « neutre » qu'un widget pose de lui-même sur une clé absente de
 * `/resolved` (`bind:` adopte l'état du contrôle : case décochée, premier choix
 * d'un select, point 0×0, champ vide). La persister fausserait le preset (ex.
 * `bed_mesh_probe_distance = 0x0` au lieu du défaut Orca 50x50 → tranchage en
 * échec) : seule une saisie différente de cet état neutre compte.
 */
export function isWidgetFill(key: string, value: unknown): boolean {
	// Option « nullable » (surcharge filament) : la valeur n'existe que si
	// l'utilisateur a coché la surcharge — jamais un remplissage du widget, même
	// égale au premier choix (ex. `filament_retract_lift_enforce = All Surfaces`).
	if (PARAMS[key]?.nullable && value !== undefined && value !== null) return false;
	if (value === undefined || value === null || value === '' || value === false || value === 0)
		return true;
	// Défaut Orca connu (registre PrintConfig) : c'est la valeur effective d'une
	// clé absente du preset. L'égaler n'est pas un changement.
	const def = PARAMS[key]?.default;
	const hasDefault = def !== undefined && def !== null;
	if (hasDefault && settingEqual(def, value, isPercentKey(key))) return true;
	if (Array.isArray(value)) return value.every((v) => isWidgetFill(key, v));
	// Premier choix d'un select : remplissage du widget seulement si le défaut
	// Orca est inconnu. Sinon un choix différent du défaut est une vraie saisie,
	// même s'il s'agit du premier élément de la liste.
	if (hasDefault) return false;
	const first = PARAMS[key]?.enumValues?.[0];
	return typeof value === 'string' && first !== undefined && value === first;
}

export function hasPresetChanges(resolved: PresetValues, current: PresetValues): boolean {
	return Object.keys(changedPresetValues(resolved, current)).length > 0;
}

/**
 * Corps `values` à enregistrer. Pour une dérivation, seules les différences sont
 * nécessaires puisque `inherits` fournit le reste. Pour une mise à jour, les
 * surcharges brutes existantes sont conservées puis les différences appliquées.
 */
export function presetValuesForSave(
	resolved: PresetValues,
	current: PresetValues,
	raw: PresetValues,
	derive: boolean
): PresetValues {
	const changed = changedPresetValues(resolved, syncThumbnailsFormat(resolved, current));
	return derive ? changed : { ...clonePresetValues(raw), ...changed };
}

const THUMBNAIL_FORMATS = ['PNG', 'JPG', 'QOI', 'BTT_TFT', 'COLPIC'];

/**
 * TabPrinter (Tab.cpp:4510-4533) : quand `thumbnails` change, `thumbnails_format`
 * prend le format de sa première entrée (« WxH/EXT », extension absente → PNG ;
 * extension inconnue → PNG), pour la rétrocompatibilité des firmwares qui lisent
 * encore `thumbnails_format`. Copie ; valeurs inchangées sinon.
 */
export function syncThumbnailsFormat(resolved: PresetValues, current: PresetValues): PresetValues {
	const value = current.thumbnails;
	if (
		typeof value !== 'string' ||
		value === resolved.thumbnails ||
		!('thumbnails_format' in current)
	)
		return current;
	const first = value.split(',')[0]?.trim() ?? '';
	if (!/^\d+(\.\d+)?x\d+(\.\d+)?(\/|$)/.test(first)) return current;
	const ext = (first.split('/')[1] ?? '').trim().toUpperCase();
	const format = THUMBNAIL_FORMATS.includes(ext) ? ext : 'PNG';
	return current.thumbnails_format === format ? current : { ...current, thumbnails_format: format };
}

// --- Valeurs par défaut d'Orca pour les clés absentes du profil -------------

/** Clés éditables d'un type de preset (onglets du layout). */
const KIND_KEYS = new Map<PresetKind, string[]>();
function keysOfKind(kind: PresetKind): string[] {
	let keys = KIND_KEYS.get(kind);
	if (!keys) {
		const set = new Set<string>();
		for (const page of UI_LAYOUT)
			if (page.kind === kind)
				for (const section of page.sections)
					for (const option of section.options) if (typeof option === 'string') set.add(option);
		keys = [...set];
		KIND_KEYS.set(kind, keys);
	}
	return keys;
}

/** Défaut du registre (PrintConfig.cpp) au format de `/resolved`, ou undefined. */
export function orcaDefault(key: string): unknown {
	const def = PARAMS[key];
	if (!def || def.nullable || def.default === null || def.default === undefined) return undefined;
	const vector = /s(Nullable)?$/.test(def.type) && def.type !== 'coPointsGroups';
	const one = (v: unknown): unknown =>
		v && typeof v === 'object' && !Array.isArray(v) && 'value' in v
			? (v as { percent?: boolean }).percent
				? `${(v as { value: number }).value}%`
				: (v as { value: number }).value
			: v;
	const d = def.default;
	if (Array.isArray(d)) return vector || def.type === 'coPoint' ? d.map(one) : one(d[0]);
	return vector ? [one(d)] : one(d);
}

/**
 * Complète les valeurs résolues d'un preset avec les défauts d'Orca pour les
 * clés que la chaîne d'héritage ne définit pas. Orca affiche ces options avec
 * leur valeur par défaut ; sans cela, un booléen absent s'affichait décoché
 * (precise_outer_wall, défaut vrai) et le premier clic écrivait la mauvaise
 * valeur. Les clés présentes ne sont jamais modifiées. Copie.
 */
export function withOrcaDefaults(kind: PresetKind, values: PresetValues): PresetValues {
	const out = { ...values };
	for (const key of keysOfKind(kind)) {
		if (key in out && out[key] !== null && out[key] !== undefined) continue;
		const d = orcaDefault(key);
		if (d !== undefined) out[key] = clonePresetValues({ d }).d;
	}
	return out;
}

/** Clés d'un type de preset qui reçoivent un défaut Orca quand elles manquent. */
export function defaultableKeys(kind: PresetKind): string[] {
	return keysOfKind(kind).filter((key) => orcaDefault(key) !== undefined);
}
