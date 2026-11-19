// État de la liaison avec le serveur (phase 32). Le client HTTP signale chaque
// échec réseau (fetch rejeté, 502/503/504 du proxy) et chaque réponse reçue ;
// tant que le serveur est injoignable, une sonde légère (/api/health) vérifie
// son retour. Le bandeau global (ConnectionBanner) et l'éditeur s'y abonnent.
import { writable, get } from 'svelte/store';

export interface ConnectionState {
	/** `false` dès qu'une requête n'a pas pu joindre le serveur. */
	reachable: boolean;
	/** Navigateur hors ligne (`navigator.onLine === false`). */
	browserOffline: boolean;
	/** Horodatage de la perte de liaison (ms), `null` si joignable. */
	since: number | null;
	/** Incrémenté à chaque retour de liaison (abonnés : resynchroniser). */
	restored: number;
}

const initial: ConnectionState = {
	reachable: true,
	browserOffline: false,
	since: null,
	restored: 0
};
export const connection = writable<ConnectionState>(initial);

/** Intervalle de la sonde de retour (ms). */
export const PROBE_INTERVAL_MS = 4000;
let probeTimer: ReturnType<typeof setTimeout> | null = null;
let probeImpl: () => Promise<boolean> = defaultProbe;

async function defaultProbe(): Promise<boolean> {
	try {
		const res = await fetch('/api/health', { cache: 'no-store', credentials: 'include' });
		return res.ok;
	} catch {
		return false;
	}
}

function browserIsOffline(): boolean {
	return typeof navigator !== 'undefined' && navigator.onLine === false;
}

/** Message affiché pour une requête qui n'a pas pu aboutir. */
export function unreachableMessage(status?: number): string {
	if (browserIsOffline())
		return 'Vous êtes hors ligne : action impossible. Elle pourra être relancée dès le retour de la connexion.';
	if (status)
		return `Serveur indisponible (${status}) : action impossible pour l'instant. Réessayez dans quelques instants.`;
	return "Serveur injoignable : action impossible pour l'instant. Vérifiez la connexion puis réessayez.";
}

/** Une requête a échoué faute de liaison : passe hors ligne, lance la sonde. */
export function reportUnreachable(): void {
	const cur = get(connection);
	if (cur.reachable || cur.browserOffline !== browserIsOffline())
		connection.set({
			...cur,
			reachable: false,
			browserOffline: browserIsOffline(),
			since: cur.since ?? Date.now()
		});
	scheduleProbe();
}

/** Une réponse du serveur est arrivée : liaison rétablie si elle était perdue. */
export function reportReachable(): void {
	const cur = get(connection);
	if (probeTimer) {
		clearTimeout(probeTimer);
		probeTimer = null;
	}
	if (!cur.reachable)
		connection.set({
			reachable: true,
			browserOffline: false,
			since: null,
			restored: cur.restored + 1
		});
}

/** Sonde immédiate (bouton « Réessayer », événement `online`). */
export async function probeNow(): Promise<boolean> {
	const ok = await probeImpl();
	if (ok) reportReachable();
	else reportUnreachable();
	return ok;
}

function scheduleProbe(): void {
	if (probeTimer || typeof window === 'undefined') return;
	probeTimer = setTimeout(async () => {
		probeTimer = null;
		if (get(connection).reachable) return;
		await probeNow();
	}, PROBE_INTERVAL_MS);
}

/** Branche les événements `online` / `offline` du navigateur (une fois, au
 *  montage du layout). Renvoie la fonction de débranchement. */
export function watchBrowserConnectivity(): () => void {
	if (typeof window === 'undefined') return () => {};
	const offline = () => reportUnreachable();
	const online = () => void probeNow();
	window.addEventListener('offline', offline);
	window.addEventListener('online', online);
	return () => {
		window.removeEventListener('offline', offline);
		window.removeEventListener('online', online);
	};
}

/** Tests : remplace la sonde et réinitialise l'état. */
export function __resetConnectivity(probe?: () => Promise<boolean>): void {
	if (probeTimer) clearTimeout(probeTimer);
	probeTimer = null;
	probeImpl = probe ?? defaultProbe;
	connection.set(initial);
}
