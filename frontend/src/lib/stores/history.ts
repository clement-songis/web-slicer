// Historique undo/redo générique (T052) : pile d'instantanés immuables
// (past / present / future) façon « time-travel ». Un nouvel état après un undo
// tronque la branche redo (comportement standard des éditeurs). Profondeur
// bornée pour ne pas croître indéfiniment. Classe pure → testable avec bun.

/** Historique d'états de type `T` avec undo/redo. */
export class History<T> {
	private past: T[] = [];
	private present: T;
	private future: T[] = [];
	/** Profondeur maximale de la pile undo (les états les plus anciens sont oubliés). */
	private readonly limit: number;

	constructor(initial: T, limit = 100) {
		this.present = initial;
		this.limit = Math.max(1, limit);
	}

	/** État courant. */
	get current(): T {
		return this.present;
	}

	get canUndo(): boolean {
		return this.past.length > 0;
	}

	get canRedo(): boolean {
		return this.future.length > 0;
	}

	/** Nombre d'annulations et de rétablissements disponibles. */
	get depth(): { undo: number; redo: number } {
		return { undo: this.past.length, redo: this.future.length };
	}

	/**
	 * Enregistre un nouvel état comme état courant. Empile l'ancien dans l'undo,
	 * vide la branche redo, et applique la limite de profondeur. Un `next`
	 * identique à l'état courant est ignoré (pas d'entrée vide dans l'historique).
	 */
	push(next: T): void {
		if (next === this.present) return;
		this.past.push(this.present);
		if (this.past.length > this.limit) this.past.shift();
		this.present = next;
		this.future = [];
	}

	/** Annule : restaure l'état précédent et le renvoie (ou `undefined`). */
	undo(): T | undefined {
		const prev = this.past.pop();
		if (prev === undefined) return undefined;
		this.future.unshift(this.present);
		this.present = prev;
		return this.present;
	}

	/** Rétablit : ré-applique l'état annulé le plus récent et le renvoie. */
	redo(): T | undefined {
		const next = this.future.shift();
		if (next === undefined) return undefined;
		this.past.push(this.present);
		this.present = next;
		return this.present;
	}

	/** Réinitialise l'historique autour d'un état (par défaut l'état courant). */
	reset(state: T = this.present): void {
		this.past = [];
		this.future = [];
		this.present = state;
	}
}
