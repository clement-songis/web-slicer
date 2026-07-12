// État de réglages d'un projet (T043) : superpose les surcharges projet aux
// valeurs effectives d'un preset résolu. Ne conserve que les vraies différences
// (une surcharge égale au preset est effacée), expose les marqueurs modifié /
// verrou, le reset par option (US2-AS6) et valide chaque écriture (bornes/enums
// via params.ts). Classe pure (pas de runes) → testable avec vitest.

import { PARAMS } from '../../generated/params';
import { validateValue, type Validation } from './validate';

/** Valeurs par clé de paramètre (format JSON simple, cf. endpoint /resolved). */
export type SettingValues = Record<string, unknown>;

function deepEqual(a: unknown, b: unknown): boolean {
	return a === b || JSON.stringify(a) === JSON.stringify(b);
}

export class SettingsStore {
	/** Valeurs effectives héritées du preset résolu (base immuable). */
	private readonly effective: SettingValues;
	/** Surcharges projet : uniquement les clés réellement différentes. */
	private readonly overrides: SettingValues;
	/** Clés verrouillées (non éditables : contrôlées par une dépendance/système). */
	private readonly locked: Set<string>;

	constructor(effective: SettingValues, overrides: SettingValues = {}) {
		this.effective = { ...effective };
		this.locked = new Set();
		// Ne garde que les surcharges qui diffèrent réellement du preset.
		this.overrides = {};
		for (const [key, value] of Object.entries(overrides)) {
			if (!deepEqual(value, this.effective[key])) this.overrides[key] = value;
		}
	}

	/** Valeur affichée : surcharge si présente, sinon valeur du preset. */
	value(key: string): unknown {
		return key in this.overrides ? this.overrides[key] : this.effective[key];
	}

	/** Valeur du preset (avant surcharge). */
	presetValue(key: string): unknown {
		return this.effective[key];
	}

	/** Le paramètre est-il surchargé par le projet ? */
	isModified(key: string): boolean {
		return key in this.overrides;
	}

	/** Le paramètre est-il verrouillé (édition interdite) ? */
	isLocked(key: string): boolean {
		return this.locked.has(key);
	}

	/** (Dé)verrouille un paramètre. */
	setLocked(key: string, locked: boolean): void {
		if (locked) this.locked.add(key);
		else this.locked.delete(key);
	}

	/**
	 * Écrit une valeur. Refuse si verrouillé ou invalide (bornes/enum). Une
	 * valeur égale au preset efface la surcharge (aucune diff conservée).
	 */
	set(key: string, value: unknown): Validation {
		if (this.locked.has(key)) return { ok: false, message: 'paramètre verrouillé' };
		const def = PARAMS[key];
		if (!def) return { ok: false, message: 'paramètre inconnu' };

		const result = validateValue(def, value);
		if (!result.ok) return result;

		if (deepEqual(value, this.effective[key])) delete this.overrides[key];
		else this.overrides[key] = value;
		return result;
	}

	/** Réinitialise un paramètre à la valeur du preset (US2-AS6). */
	reset(key: string): void {
		delete this.overrides[key];
	}

	/** Réinitialise toutes les surcharges. */
	resetAll(): void {
		for (const key of Object.keys(this.overrides)) delete this.overrides[key];
	}

	/** Clés surchargées (triées, stable pour l'affichage/les tests). */
	modifiedKeys(): string[] {
		return Object.keys(this.overrides).sort();
	}

	/** Instantané des surcharges à persister avec le projet (copie). */
	overridesSnapshot(): SettingValues {
		return { ...this.overrides };
	}
}
