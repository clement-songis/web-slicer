// Brouillon de session client (clarification spec) : la scène en cours d'édition
// est persistée localement (IndexedDB) pour survivre à une fermeture accidentelle
// de l'onglet. Au rechargement d'un projet, on propose la restauration si le
// brouillon est plus récent que la version serveur.
//
// La persistance est abstraite derrière `DraftBackend` : IndexedDB dans le
// navigateur, une implémentation mémoire pour les tests (pas d'IndexedDB en
// environnement de test).

/** Instantané éditable persisté localement. */
export interface DraftRecord {
	/** Projet concerné, ou `null` pour un brouillon « nouveau projet ». */
	projectId: string | null;
	name: string;
	scene: unknown;
	activePresets: unknown;
	/** Horodatage de sauvegarde locale (RFC 3339). */
	savedAt: string;
}

/** Magasin clé→brouillon asynchrone. */
export interface DraftBackend {
	get(key: string): Promise<DraftRecord | undefined>;
	set(key: string, value: DraftRecord): Promise<void>;
	delete(key: string): Promise<void>;
}

/** Clé de stockage d'un brouillon (projet donné ou « nouveau »). */
export function draftKey(projectId: string | null): string {
	return projectId ?? 'new';
}

/** Backend mémoire (tests, ou repli si IndexedDB indisponible). */
export class MemoryDraftBackend implements DraftBackend {
	private readonly map = new Map<string, DraftRecord>();

	get(key: string): Promise<DraftRecord | undefined> {
		return Promise.resolve(this.map.get(key));
	}
	set(key: string, value: DraftRecord): Promise<void> {
		this.map.set(key, value);
		return Promise.resolve();
	}
	delete(key: string): Promise<void> {
		this.map.delete(key);
		return Promise.resolve();
	}
}

const DB_NAME = 'web-slicer';
const STORE = 'drafts';

/** Backend IndexedDB (navigateur). */
export class IndexedDbDraftBackend implements DraftBackend {
	private open(): Promise<IDBDatabase> {
		return new Promise((resolve, reject) => {
			const req = indexedDB.open(DB_NAME, 1);
			req.onupgradeneeded = () => req.result.createObjectStore(STORE);
			req.onsuccess = () => resolve(req.result);
			req.onerror = () => reject(req.error);
		});
	}

	private async tx<T>(
		mode: IDBTransactionMode,
		run: (store: IDBObjectStore) => IDBRequest
	): Promise<T> {
		const db = await this.open();
		try {
			return await new Promise<T>((resolve, reject) => {
				const request = run(db.transaction(STORE, mode).objectStore(STORE));
				request.onsuccess = () => resolve(request.result as T);
				request.onerror = () => reject(request.error);
			});
		} finally {
			db.close();
		}
	}

	get(key: string): Promise<DraftRecord | undefined> {
		return this.tx('readonly', (s) => s.get(key));
	}
	set(key: string, value: DraftRecord): Promise<void> {
		return this.tx('readwrite', (s) => s.put(value, key));
	}
	delete(key: string): Promise<void> {
		return this.tx('readwrite', (s) => s.delete(key));
	}
}

/** Backend par défaut : IndexedDB si disponible, sinon mémoire. */
export function defaultBackend(): DraftBackend {
	return typeof indexedDB !== 'undefined' ? new IndexedDbDraftBackend() : new MemoryDraftBackend();
}

/** Persistance et restauration de brouillons au-dessus d'un backend. */
export class DraftStore {
	constructor(private readonly backend: DraftBackend = defaultBackend()) {}

	/** Sauvegarde un brouillon (horodaté maintenant si non fourni). */
	async save(record: Omit<DraftRecord, 'savedAt'> & { savedAt?: string }): Promise<void> {
		const savedAt = record.savedAt ?? new Date().toISOString();
		await this.backend.set(draftKey(record.projectId), { ...record, savedAt });
	}

	load(projectId: string | null): Promise<DraftRecord | undefined> {
		return this.backend.get(draftKey(projectId));
	}

	clear(projectId: string | null): Promise<void> {
		return this.backend.delete(draftKey(projectId));
	}

	/**
	 * Brouillon à restaurer pour un projet : renvoie le brouillon s'il existe et
	 * qu'il est plus récent que la version serveur (`serverUpdatedAt`, RFC 3339).
	 * `null` si aucun brouillon ou si le serveur est à jour.
	 */
	async pendingRestore(
		projectId: string | null,
		serverUpdatedAt: string | null
	): Promise<DraftRecord | null> {
		const draft = await this.load(projectId);
		if (!draft) return null;
		if (serverUpdatedAt && Date.parse(draft.savedAt) <= Date.parse(serverUpdatedAt)) {
			return null;
		}
		return draft;
	}
}

/** Instance partagée (backend par défaut selon l'environnement). */
export const draftStore = new DraftStore();
