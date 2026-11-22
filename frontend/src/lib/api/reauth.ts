// Phase 46 (audit 44 SESS-a) — session expirée pendant l'édition : au lieu de
// « Session requise » (et du réflexe « recharger » qui perd le travail non
// enregistré), le client met la requête refusée (401) en attente, ouvre un
// dialogue de reconnexion sur place (ReauthDialog, monté par la mise en page
// racine) puis **rejoue** la requête une fois reconnecté. L'état de l'éditeur
// reste en mémoire : rien n'est rechargé. Annuler → l'erreur 401 d'origine.
// Module sans dépendance (importé par client.ts : pas de cycle).
import { writable } from 'svelte/store';

/** État du dialogue : ouvert, compte à reconnecter. */
export const reauthState = writable<{ open: boolean; email: string | null }>({
	open: false,
	email: null
});

let host = false;
let email: string | null = null;
let pending: Promise<boolean> | null = null;
let settle: ((ok: boolean) => void) | null = null;

/** Dialogue monté (sinon aucun 401 n'est mis en attente). */
export function setReauthHost(mounted: boolean): void {
	host = mounted;
	if (!mounted) finishReauth(false);
}

/** Compte connecté connu (null = pas de session : 401 normal, ex. page de connexion). */
export function setReauthUser(next: string | null): void {
	email = next;
}

/** Une réponse 401 de `path` doit-elle ouvrir la reconnexion ? */
export function canReauth(path: string, status: number): boolean {
	return status === 401 && host && email !== null && !path.startsWith('/auth/');
}

/**
 * Attend la reconnexion (dialogue partagé par toutes les requêtes refusées en
 * même temps) : `true` si l'utilisateur s'est reconnecté, `false` s'il annule.
 */
export function requestReauth(): Promise<boolean> {
	if (pending) return pending;
	pending = new Promise<boolean>((resolve) => {
		settle = resolve;
	});
	reauthState.set({ open: true, email });
	return pending;
}

/** Fin du dialogue : reconnecté (`true`, requêtes rejouées) ou annulé. */
export function finishReauth(ok: boolean): void {
	const done = settle;
	pending = null;
	settle = null;
	reauthState.set({ open: false, email });
	done?.(ok);
}
