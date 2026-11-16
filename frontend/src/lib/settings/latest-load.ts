// Chargements « dernier gagnant » par portée (process / filament / machine).
// Une requête remplacée par une plus récente n'applique rien, mais sa promesse
// ne se résout qu'avec la plus récente : un appelant qui l'attend (ex.
// `editorReady` attendant le preset machine) voit toujours l'état final, jamais
// un état intermédiaire vide.

export interface LatestLoader<K extends string> {
	/** Lance `task` ; `isCurrent()` dit si l'appel est encore le plus récent de sa portée. */
	run(kind: K, task: (isCurrent: () => boolean) => Promise<void>): Promise<void>;
	/** Promesse de chargement la plus récente de la portée (résolue si aucune). */
	pending(kind: K): Promise<void>;
}

export function createLatestLoader<K extends string>(): LatestLoader<K> {
	const generation = new Map<K, number>();
	const latest = new Map<K, Promise<void>>();
	return {
		run(kind, task) {
			const g = (generation.get(kind) ?? 0) + 1;
			generation.set(kind, g);
			const isCurrent = () => generation.get(kind) === g;
			const promise = (async () => {
				try {
					await task(isCurrent);
				} finally {
					// Remplacé : se résoudre avec le plus récent (qui peut lui-même
					// attendre un plus récent ; jamais de cycle, l'ordre est strict).
					if (!isCurrent()) await latest.get(kind);
				}
			})();
			latest.set(kind, promise);
			return promise;
		},
		pending(kind) {
			return latest.get(kind) ?? Promise.resolve();
		}
	};
}
