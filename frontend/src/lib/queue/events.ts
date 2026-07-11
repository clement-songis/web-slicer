// Connexion au canal d'événements serveur `/api/ws` (T065/T071). Fine enveloppe
// autour de `WebSocket` : reconnexion simple, décodage des `ServerEvent` typés,
// et rappel `onEvent`. Aucune logique métier (le réducteur vit dans `queue.ts`).

import type { ServerEvent } from '$lib/api/types';

/** URL WebSocket `/api/ws` dérivée de l'origine courante (ws/wss). */
export function wsUrl(): string {
	const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
	return `${proto}//${location.host}/api/ws`;
}

/** Options de connexion. */
export interface EventStreamOptions {
	onEvent: (event: ServerEvent) => void;
	/** Fabrique de socket (injectable pour les tests) ; défaut : `WebSocket`. */
	socketFactory?: (url: string) => WebSocket;
	/** Délai de reconnexion (ms) ; 0 désactive la reconnexion. */
	reconnectDelayMs?: number;
}

/** Abonnement actif : `close()` coupe la connexion et la reconnexion. */
export interface EventSubscription {
	close: () => void;
}

/**
 * Ouvre le flux d'événements et relaie chaque `ServerEvent` décodé. En cas de
 * fermeture inattendue, se reconnecte après `reconnectDelayMs` (sauf `close()`).
 */
export function subscribeEvents(options: EventStreamOptions): EventSubscription {
	const factory = options.socketFactory ?? ((url: string) => new WebSocket(url));
	const reconnectDelay = options.reconnectDelayMs ?? 2000;
	let closed = false;
	let socket: WebSocket | null = null;
	let timer: ReturnType<typeof setTimeout> | null = null;

	const connect = () => {
		if (closed) return;
		socket = factory(wsUrl());
		socket.onmessage = (ev: MessageEvent) => {
			const event = decodeEvent(ev.data);
			if (event) options.onEvent(event);
		};
		socket.onclose = () => {
			if (closed || reconnectDelay <= 0) return;
			timer = setTimeout(connect, reconnectDelay);
		};
	};
	connect();

	return {
		close() {
			closed = true;
			if (timer) clearTimeout(timer);
			socket?.close();
		}
	};
}

/** Décode un message texte en `ServerEvent` (ou `null` si illisible). */
export function decodeEvent(data: unknown): ServerEvent | null {
	if (typeof data !== 'string') return null;
	try {
		const parsed = JSON.parse(data) as { event?: string };
		return parsed && typeof parsed.event === 'string' ? (parsed as ServerEvent) : null;
	} catch {
		return null;
	}
}
