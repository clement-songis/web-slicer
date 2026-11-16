// Presse-papiers des réglages d'objet (Orca : copier/coller l'élément
// « Réglages » d'un objet vers un autre, ObjectList::copy/paste_settings).
// État de session partagé par tous les panneaux de réglages.
import { allowedKeys } from './object-settings';

let clip = $state<{ source: string; settings: Record<string, unknown> } | null>(null);

export const settingsClipboard = {
	get value() {
		return clip;
	},
	copy(source: string, settings: Record<string, unknown>) {
		clip = { source, settings: structuredClone($state.snapshot(settings)) };
	},
	clear() {
		clip = null;
	}
};

/**
 * Colle les réglages copiés sur une cible : seules les clés autorisées pour la
 * cible (volume → réglages de région) sont reprises ; les valeurs collées
 * remplacent celles de la cible, les autres surcharges de la cible restent.
 */
export function pasteSettings(
	target: Record<string, unknown>,
	copied: Record<string, unknown>,
	isPart: boolean
): { settings: Record<string, unknown>; pasted: number; skipped: string[] } {
	const allowed = allowedKeys(isPart);
	const next = { ...target };
	const skipped: string[] = [];
	let pasted = 0;
	for (const [key, value] of Object.entries(copied)) {
		if (!allowed.includes(key)) {
			skipped.push(key);
			continue;
		}
		next[key] = structuredClone(value);
		pasted++;
	}
	return { settings: next, pasted, skipped };
}
