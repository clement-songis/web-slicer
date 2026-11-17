import type { Transform } from './transform';
// Volumes ajoutés à un objet (Orca : Add part / negative volume / modifier /
// support blocker / support enforcer). Primitive pavé, exprimée dans le repère
// du maillage recentré de l'objet (même repère que le backend, cf.
// `backend/src/slicing.rs::scene_object_volumes`). Pur → testable.

/** Rôle d'un volume (miroir de `ModelVolumeType` côté moteur). */
export type VolumeType = 'part' | 'negative' | 'modifier' | 'blocker' | 'enforcer';

/** Volume sérialisé dans le document scène. */
export interface SceneVolume {
	id: string;
	type: VolumeType;
	name: string;
	/** `box` : pavé paramétrique ; `mesh` : maillage propre du modèle `modelId`
	 *  (Orca `ModelVolume`, ex. « Split to parts »), recentré puis placé en `offset`. */
	shape: 'box' | 'mesh';
	/** Modèle serveur portant le maillage d'un volume `mesh`. */
	modelId?: string;
	/** Dimensions du pavé (mm) ; emprise du maillage pour un volume `mesh`. */
	size: [number, number, number];
	/** Centre du pavé dans le repère recentré de l'objet (mm). */
	offset: [number, number, number];
	/** Rotation Euler XYZ (degrés) autour du centre du pavé ; absente = 0. */
	rotation?: [number, number, number];
	/** Surcharges de réglages (modificateur), clés du registre. */
	settings: Record<string, unknown>;
	/** Extrudeur propre du volume (1-based ; absent/0 = celui de l'objet),
	 *  Orca `ModelVolume` « extruder » (pièces et modificateurs). */
	extruder?: number;
}

/** Action de menu → rôle (libellés de parité Orca). */
export const VOLUME_ACTIONS: Record<string, { type: VolumeType; label: string; name: string }> = {
	'object.addPart': { type: 'part', label: 'Add part', name: 'Pièce' },
	'object.addNegativeVolume': {
		type: 'negative',
		label: 'Add negative volume',
		name: 'Volume négatif'
	},
	'object.addModifier': { type: 'modifier', label: 'Add modifier', name: 'Modificateur' },
	'object.addSupportBlocker': {
		type: 'blocker',
		label: 'Add support blocker',
		name: 'Bloqueur de supports'
	},
	'object.addSupportEnforcer': {
		type: 'enforcer',
		label: 'Add support enforcer',
		name: 'Forceur de supports'
	}
};

/** Libellé affiché d'un rôle. */
export function volumeTypeName(type: VolumeType): string {
	return Object.values(VOLUME_ACTIONS).find((a) => a.type === type)?.name ?? type;
}

/**
 * Volume par défaut pour un objet de dimensions `extent` (mm) : pavé de la
 * moitié de l'objet (2 mm minimum par axe), centré sur l'objet — comme le cube
 * générique qu'Orca insère au centre de l'objet sélectionné.
 */
export function defaultVolume(
	type: VolumeType,
	extent: [number, number, number],
	id: string,
	name: string
): SceneVolume {
	const size = extent.map((e) => Math.max(2, Math.round((e / 2) * 100) / 100)) as [
		number,
		number,
		number
	];
	return { id, type, name, shape: 'box', size, offset: [0, 0, 0], settings: {} };
}

/** Normalise une liste de volumes lue d'un document (entrées invalides écartées). */
export function parseVolumes(raw: unknown): SceneVolume[] {
	if (!Array.isArray(raw)) return [];
	const types: VolumeType[] = ['part', 'negative', 'modifier', 'blocker', 'enforcer'];
	const vec = (v: unknown, fallback: number): [number, number, number] =>
		[0, 1, 2].map((i) => {
			const n = Array.isArray(v) ? Number(v[i]) : NaN;
			return Number.isFinite(n) ? n : fallback;
		}) as [number, number, number];
	return raw.flatMap((v, i) => {
		const o = (v ?? {}) as Partial<SceneVolume>;
		if (!o.type || !types.includes(o.type)) return [];
		const mesh = o.shape === 'mesh';
		if (mesh && typeof o.modelId !== 'string') return [];
		const size = vec(o.size, 10);
		if (size.some((s) => s <= 0)) return [];
		return [
			{
				id: typeof o.id === 'string' ? o.id : `volume-${i}`,
				type: o.type,
				name: typeof o.name === 'string' ? o.name : volumeTypeName(o.type),
				...(mesh ? { shape: 'mesh' as const, modelId: o.modelId } : { shape: 'box' as const }),
				size,
				offset: vec(o.offset, 0),
				...(Array.isArray(o.rotation) && vec(o.rotation, 0).some((r) => r !== 0)
					? { rotation: vec(o.rotation, 0) }
					: {}),
				settings: o.settings && typeof o.settings === 'object' ? { ...o.settings } : {},
				...(Number.isInteger(o.extruder) && (o.extruder as number) > 0
					? { extruder: o.extruder as number }
					: {})
			}
		];
	});
}

/**
 * Transformation affichée par le panneau de transformation pour un volume :
 * position = centre (repère de l'objet), rotation propre, échelle relative 1
 * (l'échelle saisie multiplie la taille du pavé, comme l'échelle Orca d'un volume).
 */
export function volumeTransform(v: SceneVolume): Transform {
	return { position: [...v.offset], rotation: [...(v.rotation ?? [0, 0, 0])], scale: [1, 1, 1] };
}

/** Applique une transformation du panneau à un volume (taille × échelle). */
export function applyVolumeTransform(v: SceneVolume, t: Transform): SceneVolume {
	// Un volume maillé n'a pas de taille paramétrique : l'échelle est ignorée.
	const size = (
		v.shape === 'mesh'
			? [...v.size]
			: v.size.map((s, i) => Math.max(0.01, Math.round(s * t.scale[i] * 1000) / 1000))
	) as [number, number, number];
	const { rotation: _old, ...rest } = v;
	void _old;
	const rotation = [...t.rotation] as [number, number, number];
	return {
		...rest,
		offset: [...t.position],
		size,
		...(rotation.some((r) => r !== 0) ? { rotation } : {})
	};
}
