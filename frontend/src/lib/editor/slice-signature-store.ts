// Audit 41 V1 : à la réouverture d'un projet, le dernier G-code n'est restauré
// que s'il correspond encore au plateau (Orca : l'ouverture d'un projet ne
// restaure jamais un résultat de tranchage périmé — Plater::priv::load_files
// réinitialise le processus d'arrière-plan). La signature de tranchage du
// plateau (même calcul que B1, phase 40) est mémorisée par G-code au moment du
// tranchage, puis comparée à celle de la scène rouverte. Pas d'empreinte connue
// (autre navigateur, stockage effacé) → pas de restauration : retour en Préparer.

import { api, ApiError } from '#lib/api/client';
import type { PlateSliceSignature, SliceSignatureResponse } from '#lib/api/types';

const PREFIX = 'web-slicer:slice-signatures:';
/** G-code retenus par projet (un par plateau, plusieurs lots récents). */
const KEEP = 24;

type Store = Pick<Storage, 'getItem' | 'setItem'>;
type Entries = Record<string, { h: string; t: number }>;

function defaultStore(): Store | null {
	try {
		return typeof localStorage === 'undefined' ? null : localStorage;
	} catch {
		return null;
	}
}

/** Empreinte compacte (cyrb53, 2 × 32 bits) d'une signature de tranchage. */
export function signatureHash(text: string): string {
	let h1 = 0xdeadbeef ^ text.length;
	let h2 = 0x41c6ce57 ^ text.length;
	for (let i = 0; i < text.length; i++) {
		const c = text.charCodeAt(i);
		h1 = Math.imul(h1 ^ c, 2654435761);
		h2 = Math.imul(h2 ^ c, 1597334677);
	}
	h1 = Math.imul(h1 ^ (h1 >>> 16), 2246822507) ^ Math.imul(h2 ^ (h2 >>> 13), 3266489909);
	h2 = Math.imul(h2 ^ (h2 >>> 16), 2246822507) ^ Math.imul(h1 ^ (h1 >>> 13), 3266489909);
	return (h2 >>> 0).toString(16).padStart(8, '0') + (h1 >>> 0).toString(16).padStart(8, '0');
}

function readEntries(store: Store, projectId: string): Entries {
	try {
		const raw = store.getItem(PREFIX + projectId);
		const parsed = raw ? (JSON.parse(raw) as unknown) : null;
		return parsed && typeof parsed === 'object' && !Array.isArray(parsed)
			? (parsed as Entries)
			: {};
	} catch {
		return {};
	}
}

/** Mémorise la signature de tranchage du plateau qui a produit `gcodeId`. */
export function rememberSliceSignature(
	projectId: string,
	gcodeId: string,
	signature: string,
	store: Store | null = defaultStore(),
	now = Date.now()
): void {
	if (!store || !gcodeId) return;
	const entries = readEntries(store, projectId);
	const h = signatureHash(signature);
	if (entries[gcodeId]?.h === h) return;
	entries[gcodeId] = { h, t: now };
	const kept = Object.entries(entries)
		.sort((a, b) => b[1].t - a[1].t)
		.slice(0, KEEP);
	try {
		store.setItem(PREFIX + projectId, JSON.stringify(Object.fromEntries(kept)));
	} catch {
		// Stockage plein ou interdit : le résultat ne sera simplement pas restauré.
	}
}

/**
 * Le G-code `gcodeId` correspond-il encore au plateau ? Vrai seulement si sa
 * signature mémorisée est égale à `signature` (signature courante du plateau).
 */
export function restorableResult(
	projectId: string,
	gcodeId: string,
	signature: string,
	store: Store | null = defaultStore()
): boolean {
	if (!store || !gcodeId) return false;
	const entry = readEntries(store, projectId)[gcodeId];
	return !!entry && entry.h === signatureHash(signature);
}

/** Une empreinte locale existe-t-elle pour ce G-code (tranché dans ce navigateur) ? */
export function hasRememberedSignature(
	projectId: string,
	gcodeId: string,
	store: Store | null = defaultStore()
): boolean {
	return !!store && !!gcodeId && !!readEntries(store, projectId)[gcodeId];
}

/**
 * Empreinte locale d'un résultat (phase 42c) : cherchée sous chacune des clés
 * données, dans l'ordre (G-code, puis job). Un job lancé dans ce navigateur a
 * son empreinte mémorisée **au lancement**, sous son id (`rememberSliceSignature`
 * avec l'id du job) : un rechargement pendant le tranchage — avant que le
 * G-code n'existe — retrouve ainsi la signature du plateau tranché.
 */
export function localSignatureVerdict(
	projectId: string,
	keys: readonly (string | null | undefined)[],
	signature: string,
	store: Store | null = defaultStore()
): { known: boolean; matches: boolean } {
	for (const key of keys) {
		if (key && hasRememberedSignature(projectId, key, store))
			return { known: true, matches: restorableResult(projectId, key, signature, store) };
	}
	return { known: false, matches: false };
}

/**
 * Décision de restauration (lot 42 V1) : le verdict du serveur (signature du
 * projet **enregistré**, valable depuis n'importe quel navigateur) prime ; s'il
 * dit « à jour » mais que ce navigateur a mémorisé une empreinte différente de
 * la scène affichée (brouillon local non enregistré), on ne restaure pas.
 * Serveur muet (route absente) → empreinte locale seule.
 */
export function shouldRestore(
	server: boolean | null,
	local: { known: boolean; matches: boolean }
): boolean {
	if (server === null) return local.known && local.matches;
	if (!server) return false;
	return !local.known || local.matches;
}

/**
 * Réponse de `GET /api/projects/{id}/slice-signature` (lot 42 : signature
 * calculée sur le projet enregistré, `fresh` ⇔ le dernier G-code du plateau
 * correspond encore) — sous-ensemble structurel du DTO généré
 * `SliceSignatureResponse` (champs lus ici seulement).
 */
type ServerSliceSignature = {
	plates: (Pick<PlateSliceSignature, 'fresh' | 'last_gcode_id'> & {
		plate_index: number | bigint;
		/** Signature actuelle du plateau (comparée à celle d'un job en cours). */
		signature?: PlateSliceSignature['signature'];
	})[];
};
// Le DTO complet reste assignable (contrôle à la compilation).
export type _SliceSignatureCompat = SliceSignatureResponse extends ServerSliceSignature
	? true
	: never;

/**
 * Verdict du serveur pour le G-code `gcodeId` du plateau `plateIndex` :
 * `true` / `false` si le serveur sait calculer la signature, `null` s'il ne
 * l'expose pas encore (404/405 : backend antérieur) ou en cas d'erreur —
 * l'appelant retombe alors sur l'empreinte locale.
 */
export async function serverSliceFreshness(
	projectId: string,
	plateIndex: number,
	gcodeId: string,
	get: (path: string) => Promise<ServerSliceSignature> = (path) =>
		api.get<ServerSliceSignature>(path)
): Promise<boolean | null> {
	try {
		const res = await get(`/projects/${projectId}/slice-signature`);
		if (!res || !Array.isArray(res.plates)) return null;
		const plate = res.plates.find((p) => Number(p.plate_index) === plateIndex);
		return !!plate && plate.fresh && plate.last_gcode_id === gcodeId;
	} catch (e) {
		if (e instanceof ApiError && e.status !== 404 && e.status !== 405) return false;
		return null;
	}
}

/**
 * Verdict du serveur pour un job **en cours** (phase 42c) : sa signature figée
 * au lancement (`JobResponse.slice_signature`, lot 42) est-elle celle du
 * plateau `plateIndex` du projet enregistré ? `null` (repli sur l'empreinte
 * locale) si le job n'a pas de signature (job antérieur au lot 42) ou si le
 * serveur n'expose pas la route ; `false` sur une autre erreur.
 */
export async function serverJobFreshness(
	projectId: string,
	plateIndex: number,
	jobSignature: string | null | undefined,
	get: (path: string) => Promise<ServerSliceSignature> = (path) =>
		api.get<ServerSliceSignature>(path)
): Promise<boolean | null> {
	if (!jobSignature) return null;
	try {
		const res = await get(`/projects/${projectId}/slice-signature`);
		if (!res || !Array.isArray(res.plates)) return null;
		const plate = res.plates.find((p) => Number(p.plate_index) === plateIndex);
		return !!plate && plate.signature === jobSignature;
	} catch (e) {
		if (e instanceof ApiError && e.status !== 404 && e.status !== 405) return false;
		return null;
	}
}
