// Multi-plateaux (T059) : gestion de plusieurs plateaux, chacun portant ses
// objets, son type de plaque (réglages `PlateTemps`) et son nom. Ajout /
// suppression / répartition automatique des objets, et index de plateau prêt
// pour le tranchage par plateau (P5 : `POST …/slice` body `plate_index`).
// Réactif : `plates` et `activeId` sont en `$state` pour que la couche Svelte
// se re-rende à chaque mutation. Runes → testé sous vitest, pas sous bun.
// Persisté dans le document scène.
import { PLATE_TYPES } from '../settings/special/dialogs';
import { tr } from '$lib/i18n';
import type { PlateCustomGcode } from '../preview/color-changes';

/** Type de plaque par défaut (première entrée de la table des températures). */
export const DEFAULT_PLATE_TYPE = PLATE_TYPES[0].label;

/** Un plateau : identité, type de plaque et objets posés. */
export interface Plate {
	id: string;
	name: string;
	plateType: string;
	objectIds: string[];
	/** Réglages par plateau (Orca « Plate settings ») : clés de PLATE_SETTING_KEYS
	 *  côté moteur (curr_bed_type, print_sequence, spiral_mode…), appliquées
	 *  par-dessus les presets au tranchage de ce plateau. Absent = suit le profil. */
	settings?: Record<string, unknown>;
	/** G-code par couche du curseur de l'Aperçu (changement de couleur, pause,
	 *  code perso) — lus par le serveur au tranchage (`scene_custom_gcodes`). */
	customGcodes?: PlateCustomGcode[];
	/**
	 * Plateau verrouillé (lot 34, Orca PartPlate verrouillé) : l'arrangement
	 *  « tout » ne le déplace pas (l'arrangeur serveur lit `plates[].locked`),
	 *  « ce plateau » est refusé (422) et ses objets ne se déplacent pas.
	 */
	locked?: boolean;
}

/** Copie profonde d'un plateau (objets, réglages, G-code par couche). */
function clonePlate(p: Plate): Plate {
	return {
		...p,
		objectIds: [...p.objectIds],
		...(p.settings ? { settings: { ...p.settings } } : {}),
		...(p.customGcodes ? { customGcodes: p.customGcodes.map((g) => ({ ...g })) } : {}),
		...(p.locked ? { locked: true } : {})
	};
}

/** Document sérialisé du jeu de plateaux. */
export interface PlatesDocument {
	plates: Plate[];
	activeId: string | null;
}

/** Jeu de plateaux d'un projet. */
export class PlateSet {
	private plates = $state<Plate[]>([]);
	private counter = 0;
	activeId = $state<string | null>(null);
	/** Type de plaque d'un nouveau plateau : celui de l'imprimante (phase 44b,
	 *  Orca `Preset::get_default_bed_type`), « Cool Plate » tant qu'il est inconnu. */
	private defaultType: string;
	/** Plateaux dont le type vient du défaut et n'a pas été choisi (Orca : plateau
	 *  « par défaut », qui suit le type du projet). Non persisté. */
	// eslint-disable-next-line svelte/prefer-svelte-reactivity -- ensemble interne, non affiché
	private autoTyped = new Set<string>();

	constructor(defaultPlateType: string = DEFAULT_PLATE_TYPE) {
		this.defaultType = defaultPlateType;
		this.activeId = this.addPlate().id;
	}

	/** Type de plaque donné aux nouveaux plateaux. */
	get defaultPlateType(): string {
		return this.defaultType;
	}

	/**
	 * Nouveau type par défaut (imprimante résolue ou changée) : les plateaux au
	 * type automatique le prennent ; `previous` (changement d'imprimante) le
	 * donne aussi aux plateaux encore au défaut de l'imprimante précédente
	 * (Orca : le type du projet est réinitialisé, les plateaux « par défaut »
	 * suivent ; un type choisi explicitement est conservé). Renvoie les ids
	 * des plateaux modifiés.
	 */
	followDefaultPlateType(next: string, previous: string | null = null): string[] {
		this.defaultType = next;
		const changed: string[] = [];
		for (const p of this.plates) {
			const follows = this.autoTyped.has(p.id) || (previous !== null && p.plateType === previous);
			if (!follows) continue;
			this.autoTyped.add(p.id);
			if (p.plateType !== next) {
				p.plateType = next;
				changed.push(p.id);
			}
		}
		return changed;
	}

	// Ne réémet jamais un id déjà présent : après désérialisation ou suppression,
	// `plate-<n>` peut déjà exister (ids non contigus, ids UUID d'une copie).
	private nextId(): string {
		let id: string;
		do {
			this.counter += 1;
			id = `plate-${this.counter}`;
		} while (this.plates.some((p) => p.id === id));
		return id;
	}

	// Premier nom « Plateau <n> » libre, à partir du rang du nouveau plateau.
	private freeName(): string {
		let n = this.plates.length + 1;
		while (this.plates.some((p) => p.name === tr('Plate {n}', { n }))) n += 1;
		return tr('Plate {n}', { n });
	}

	/** Ajoute un plateau (vide) et le renvoie. */
	addPlate(): Plate {
		const plate: Plate = {
			id: this.nextId(),
			name: this.freeName(),
			plateType: this.defaultType,
			objectIds: []
		};
		this.plates.push(plate);
		this.autoTyped.add(plate.id);
		return plate;
	}

	/**
	 * Duplique un plateau : nouvel id unique, nom « <source> (copie) » libre,
	 * même type de plaque, inséré juste après la source puis activé. Chaque
	 * objet de la source est confié à `copyObject` (qui crée l'instance copiée et
	 * renvoie son id, ou `null` si l'objet ne peut pas être copié) ; les copies
	 * sont affectées au nouveau plateau, la source reste intacte. `null` si le
	 * plateau source n'existe pas.
	 */
	duplicatePlate(
		sourceId: string,
		copyObject: (objectId: string, plateId: string) => string | null
	): Plate | null {
		const source = this.get(sourceId);
		if (!source) return null;
		const sourceObjects = [...source.objectIds];
		let name = `${source.name} (copie)`;
		for (let n = 2; this.plates.some((p) => p.name === name); n += 1) {
			name = `${source.name} (copie ${n})`;
		}
		this.plates.splice(this.plateIndex(sourceId) + 1, 0, {
			id: this.nextId(),
			name,
			plateType: source.plateType,
			objectIds: []
		});
		// Relit le plateau via le proxy réactif (l'objet littéral n'est pas suivi).
		const plate = this.plates[this.plateIndex(sourceId) + 1];
		for (const objectId of sourceObjects) {
			const copyId = copyObject(objectId, plate.id);
			if (copyId) this.assign(copyId, plate.id);
		}
		this.activeId = plate.id;
		return plate;
	}

	list(): readonly Plate[] {
		return this.plates;
	}

	get(id: string): Plate | undefined {
		return this.plates.find((p) => p.id === id);
	}

	/** Index (0-based) d'un plateau — cible du tranchage par plateau (P5). */
	plateIndex(id: string): number {
		return this.plates.findIndex((p) => p.id === id);
	}

	/**
	 * Supprime un plateau : ses objets migrent vers le premier plateau restant.
	 * Refuse de supprimer le dernier plateau (il en faut toujours un).
	 */
	removePlate(id: string): boolean {
		if (this.plates.length <= 1) return false;
		const idx = this.plateIndex(id);
		if (idx < 0) return false;
		const [removed] = this.plates.splice(idx, 1);
		const target = this.plates[0];
		target.objectIds.push(...removed.objectIds);
		if (this.activeId === id) this.activeId = target.id;
		return true;
	}

	/**
	 * « Déplacer le plateau vers l'avant » (icône du plateau, Orca
	 * PartPlateList::move_plate_to_index(i, 0)) : le plateau devient le premier,
	 * les précédents reculent d'un rang ; il devient actif (select_plate(0)).
	 * Les objets suivent leur plateau (positions relatives au plateau).
	 */
	moveToFront(id: string): boolean {
		const idx = this.plateIndex(id);
		if (idx <= 0) return false;
		const [plate] = this.plates.splice(idx, 1);
		this.plates.unshift(plate);
		this.activeId = plate.id;
		return true;
	}

	/** Affecte un objet à un plateau (le retire de tout autre plateau). */
	assign(objectId: string, plateId: string): void {
		const plate = this.get(plateId);
		if (!plate) return;
		this.unassign(objectId);
		plate.objectIds.push(objectId);
	}

	/** Retire un objet de tous les plateaux. */
	unassign(objectId: string): void {
		for (const p of this.plates) {
			const i = p.objectIds.indexOf(objectId);
			if (i >= 0) p.objectIds.splice(i, 1);
		}
	}

	/** Plateau contenant un objet, ou `null`. */
	plateOf(objectId: string): string | null {
		return this.plates.find((p) => p.objectIds.includes(objectId))?.id ?? null;
	}

	setPlateType(id: string, plateType: string): void {
		const plate = this.get(id);
		if (!plate) return;
		plate.plateType = plateType;
		this.autoTyped.delete(id);
	}

	/** Définit (ou efface si `undefined`) un réglage du plateau. */
	setPlateSetting(id: string, key: string, value: unknown): void {
		const plate = this.get(id);
		if (!plate) return;
		const next = { ...(plate.settings ?? {}) };
		if (value === undefined) delete next[key];
		else next[key] = value;
		if (Object.keys(next).length) plate.settings = next;
		else delete plate.settings;
	}

	/** Remplace les G-code par couche du plateau (liste vide = clé retirée). */
	setCustomGcodes(id: string, list: PlateCustomGcode[]): void {
		const plate = this.get(id);
		if (!plate) return;
		if (list.length) plate.customGcodes = list.map((g) => ({ ...g }));
		else delete plate.customGcodes;
	}

	setName(id: string, name: string): void {
		const plate = this.get(id);
		if (plate) plate.name = name;
	}

	/** Verrouille (ou déverrouille) un plateau : non déplacé à l'arrangement. */
	setLocked(id: string, locked: boolean): void {
		const plate = this.get(id);
		if (!plate) return;
		if (locked) plate.locked = true;
		else delete plate.locked;
	}

	/**
	 * Répartit des objets par paquets de `perPlate` sur autant de plateaux que
	 * nécessaire (crée les plateaux manquants, vide d'abord les affectations).
	 */
	distribute(objectIds: string[], perPlate: number): void {
		const size = Math.max(1, Math.trunc(perPlate));
		const needed = Math.max(1, Math.ceil(objectIds.length / size));
		while (this.plates.length < needed) this.addPlate();
		for (const p of this.plates) p.objectIds = [];
		for (let i = 0; i < objectIds.length; i++) {
			this.plates[Math.floor(i / size)].objectIds.push(objectIds[i]);
		}
	}

	/** Renomme un plateau (nom vide ignoré). */
	rename(id: string, name: string): void {
		const plate = this.get(id);
		const trimmed = name.trim();
		if (plate && trimmed) plate.name = trimmed;
	}

	/** Restaure sur place un document issu de `serialize()` (historique). */
	restore(doc: PlatesDocument): void {
		this.plates = doc.plates.map(clonePlate);
		for (const p of doc.plates) {
			const m = /^plate-(\d+)$/.exec(p.id);
			if (m) this.counter = Math.max(this.counter, Number(m[1]));
		}
		this.activeId =
			doc.activeId && this.get(doc.activeId) ? doc.activeId : (this.plates[0]?.id ?? null);
	}

	serialize(): PlatesDocument {
		return {
			plates: this.plates.map(clonePlate),
			activeId: this.activeId
		};
	}

	static deserialize(doc: PlatesDocument, defaultPlateType?: string): PlateSet {
		const set = new PlateSet(defaultPlateType);
		set.plates = (doc.plates ?? []).map(clonePlate);
		// Types relus : choisis (Orca ne touche pas au type d'un projet chargé).
		set.autoTyped.clear();
		// Compteur = plus grand suffixe `plate-<n>` existant (et non le nombre de
		// plateaux, qui recréait un id déjà présent après une suppression).
		set.counter = set.plates.reduce((max, p) => {
			const m = /^plate-(\d+)$/.exec(p.id);
			return m ? Math.max(max, Number(m[1])) : max;
		}, 0);
		const active = doc.activeId && set.get(doc.activeId) ? doc.activeId : null;
		set.activeId = active ?? set.plates[0]?.id ?? null;
		if (set.plates.length === 0) set.activeId = set.addPlate().id;
		return set;
	}
}
