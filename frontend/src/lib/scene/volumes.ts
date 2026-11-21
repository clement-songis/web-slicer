import type { Transform } from './transform';
import { tr } from '$lib/i18n';
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
	/** Phase 28 — texte / SVG en relief ré-éditable (parts[].emboss du serveur). */
	emboss?: Record<string, unknown>;
	/** Lot 37 — connecteur d'une coupe (Orca `ModelVolume::cut_info`), recopié
	 *  de `parts[].volumes[].cut_info` de /cut ; écrit dans cut_information.xml. */
	cutInfo?: VolumeCutInfo;
}

/** Informations de coupe d'un volume connecteur (forme scène du serveur). */
export interface VolumeCutInfo {
	connectorType: 'plug' | 'dowel' | 'snap';
	radiusTolerance: number;
	heightTolerance: number;
}

/** Action de menu → rôle (libellés de parité Orca). `name` : nom par défaut
 *  traduit à la lecture (accesseur), dans la langue courante. */
export const VOLUME_ACTIONS: Record<string, { type: VolumeType; label: string; name: string }> = {
	'object.addPart': {
		type: 'part',
		label: 'Add part',
		get name() {
			return tr('Part||volume');
		}
	},
	'object.addNegativeVolume': {
		type: 'negative',
		label: 'Add negative volume',
		get name() {
			return tr('Negative volume');
		}
	},
	'object.addModifier': {
		type: 'modifier',
		label: 'Add modifier',
		get name() {
			return tr('Modifier');
		}
	},
	'object.addSupportBlocker': {
		type: 'blocker',
		label: 'Add support blocker',
		get name() {
			return tr('Support blocker');
		}
	},
	'object.addSupportEnforcer': {
		type: 'enforcer',
		label: 'Add support enforcer',
		get name() {
			return tr('Support enforcer');
		}
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
					: {}),
				...(o.emboss && typeof o.emboss === 'object' && !Array.isArray(o.emboss)
					? { emboss: { ...o.emboss } }
					: {}),
				...(parseCutInfo(o.cutInfo) ? { cutInfo: parseCutInfo(o.cutInfo)! } : {})
			}
		];
	});
}

/** `cutInfo` d'un volume lu d'un document (forme invalide → absent). */
export function parseCutInfo(raw: unknown): VolumeCutInfo | null {
	if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return null;
	const c = raw as Partial<VolumeCutInfo>;
	if (c.connectorType !== 'plug' && c.connectorType !== 'dowel' && c.connectorType !== 'snap')
		return null;
	const num = (v: unknown) => (typeof v === 'number' && Number.isFinite(v) ? v : 0);
	return {
		connectorType: c.connectorType,
		radiusTolerance: num(c.radiusTolerance),
		heightTolerance: num(c.heightTolerance)
	};
}

/**
 * Choix de « Changer le type » (ObjectList::change_part_type d'Orca, dialogue
 * « Choisissez le type de pièce ») : pièce, volume négatif, modificateur, et —
 * sauf pour un texte / SVG en relief — bloqueur et forceur de supports. Mêmes
 * noms que la liste des pièces et le menu « Ajouter » de l'application.
 */
export const PART_TYPE_CHOICES: { type: VolumeType; label: string }[] = (
	['part', 'negative', 'modifier', 'blocker', 'enforcer'] as const
).map((type) => ({
	type,
	// Accesseur : libellé traduit à la lecture (jamais figé au chargement du module).
	get label() {
		return volumeTypeName(type);
	}
}));

/** Types proposés pour un volume (relief : pas de bloqueur / générateur). */
export function partTypeChoices(v: Pick<SceneVolume, 'emboss'>): typeof PART_TYPE_CHOICES {
	return v.emboss ? PART_TYPE_CHOICES.slice(0, 3) : PART_TYPE_CHOICES;
}

/**
 * Change le type d'un volume ajouté. Le maillage propre de l'objet (1er volume,
 * toujours « pièce ») garantit qu'il reste une pièce solide : Orca refuse
 * seulement de changer la dernière pièce solide, jamais un volume ajouté.
 * Comme `ModelVolume::set_type`, réglages et extrudeur propres sont conservés
 * (ignorés au tranchage hors pièces / modificateurs, retrouvés au retour).
 * `null` si le type est inchangé ou interdit (relief en bloqueur/générateur).
 */
export function changeVolumeType(v: SceneVolume, type: VolumeType): SceneVolume | null {
	if (v.type === type) return null;
	if (!partTypeChoices(v).some((c) => c.type === type)) return null;
	return { ...v, type };
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
