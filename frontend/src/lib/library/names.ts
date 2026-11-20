// Noms de projet uniques (phase 32). Le serveur impose un nom unique par compte
// (contrainte projects.user_id + name) : « Nouveau projet » une seconde fois, ou
// le nommage automatique d'après un fichier déjà utilisé, échouaient avec
// l'erreur SQL brute. Orca n'a pas de bibliothèque à noms uniques ; comme un
// gestionnaire de fichiers, on suffixe « (2) », « (3) »…
import { ApiError } from '$lib/api/client';

/** Nom libre dérivé de `base` : `base`, puis `base (2)`, `base (3)`… La
 *  comparaison est exacte (la contrainte SQLite est sensible à la casse). */
export function uniqueProjectName(base: string, taken: Iterable<string>): string {
	const name = base.trim() || 'Sans titre';
	const used = new Set(taken);
	if (!used.has(name)) return name;
	// « cube (2) » renommé encore : repart de la racine « cube ».
	const root = name.replace(/ \((\d+)\)$/, '');
	for (let n = 2; n < 10_000; n++) {
		const candidate = `${root} (${n})`;
		if (!used.has(candidate)) return candidate;
	}
	return `${root} (${Date.now()})`;
}

/** Conflit de nom renvoyé par le serveur (409, ou contrainte UNIQUE remontée). */
export function isNameConflict(err: unknown): boolean {
	if (!(err instanceof ApiError)) return false;
	return err.status === 409 || /UNIQUE constraint failed: projects\./.test(err.message);
}

/** Nom libre proposé par le serveur (409 `name_taken`, details.suggestion — lot 32). */
export function nameSuggestion(err: unknown): string | null {
	if (!(err instanceof ApiError) || err.status !== 409) return null;
	const s = (err.details as { suggestion?: unknown } | undefined)?.suggestion;
	return typeof s === 'string' && s.trim() ? s : null;
}

/** Message lisible pour un renommage refusé car le nom est déjà pris. */
export function nameTakenMessage(name: string, suggestion?: string | null): string {
	return suggestion
		? `Un autre projet s'appelle déjà « ${name} ». Nom libre proposé : « ${suggestion} ».`
		: `Un autre projet s'appelle déjà « ${name} ». Choisissez un autre nom.`;
}
