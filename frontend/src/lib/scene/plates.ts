// Multi-plateaux (T059) : gestion de plusieurs plateaux, chacun portant ses
// objets, son type de plaque (réglages `PlateTemps`) et son nom. Ajout /
// suppression / répartition automatique des objets, et index de plateau prêt
// pour le tranchage par plateau (P5 : `POST …/slice` body `plate_index`).
// Modèle pur → testable ; persisté dans le document scène.
import { PLATE_TYPES } from '../settings/special/dialogs';

/** Type de plaque par défaut (première entrée de la table des températures). */
export const DEFAULT_PLATE_TYPE = PLATE_TYPES[0].label;

/** Un plateau : identité, type de plaque et objets posés. */
export interface Plate {
	id: string;
	name: string;
	plateType: string;
	objectIds: string[];
}

/** Document sérialisé du jeu de plateaux. */
export interface PlatesDocument {
	plates: Plate[];
	activeId: string | null;
}

/** Jeu de plateaux d'un projet. */
export class PlateSet {
	private plates: Plate[] = [];
	private counter = 0;
	activeId: string | null = null;

	constructor() {
		this.activeId = this.addPlate().id;
	}

	private nextId(): string {
		this.counter += 1;
		return `plate-${this.counter}`;
	}

	/** Ajoute un plateau (vide) et le renvoie. */
	addPlate(): Plate {
		const plate: Plate = {
			id: this.nextId(),
			name: `Plateau ${this.plates.length + 1}`,
			plateType: DEFAULT_PLATE_TYPE,
			objectIds: []
		};
		this.plates.push(plate);
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
		if (plate) plate.plateType = plateType;
	}

	setName(id: string, name: string): void {
		const plate = this.get(id);
		if (plate) plate.name = name;
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

	serialize(): PlatesDocument {
		return {
			plates: this.plates.map((p) => ({ ...p, objectIds: [...p.objectIds] })),
			activeId: this.activeId
		};
	}

	static deserialize(doc: PlatesDocument): PlateSet {
		const set = new PlateSet();
		set.plates = (doc.plates ?? []).map((p) => ({ ...p, objectIds: [...p.objectIds] }));
		set.counter = set.plates.length;
		set.activeId = doc.activeId ?? set.plates[0]?.id ?? null;
		if (set.plates.length === 0) set.activeId = set.addPlate().id;
		return set;
	}
}
