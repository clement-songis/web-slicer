// Sauvegarde de projet (T060) : verrou optimiste (champ `version`), gestion du
// conflit multi-onglets (409 → avertissement), raccourci Ctrl/Cmd+S et capture
// de vignette. La classification d'erreur est pure et testable ; l'appel réseau
// s'appuie sur le client `api/projects`.
import { ApiError } from '../api/client';
import { saveProject } from '../api/projects';
import type { SceneDocument } from './document';

/** Résultat d'une tentative de sauvegarde. */
export type SaveOutcome =
	| { status: 'saved'; version: number }
	| { status: 'conflict' }
	| { status: 'error'; message: string };

/** Classe une erreur d'API : le 409 (verrou optimiste) est un conflit. */
export function classifySaveError(e: unknown): 'conflict' | 'error' {
	return e instanceof ApiError && e.status === 409 ? 'conflict' : 'error';
}

/**
 * Sauvegarde la scène avec verrou optimiste. En cas de conflit multi-onglets
 * (409), renvoie `conflict` sans écraser la version serveur — l'appelant
 * avertit l'utilisateur (edge case spec, analyse G5).
 */
export async function saveScene(
	projectId: string,
	expectedVersion: number,
	scene: SceneDocument,
	activePresets: unknown
): Promise<SaveOutcome> {
	try {
		const project = await saveProject(projectId, expectedVersion, scene, activePresets);
		return { status: 'saved', version: Number(project.version) };
	} catch (e) {
		if (classifySaveError(e) === 'conflict') return { status: 'conflict' };
		return {
			status: 'error',
			message: e instanceof ApiError ? e.message : 'échec de la sauvegarde'
		};
	}
}

/**
 * Attache un raccourci Ctrl/Cmd+S déclenchant `handler` (sauvegarde manuelle,
 * FR-052). Renvoie une fonction de désabonnement.
 */
export function onSaveShortcut(handler: () => void): () => void {
	const listener = (e: KeyboardEvent) => {
		if ((e.ctrlKey || e.metaKey) && (e.key === 's' || e.key === 'S')) {
			e.preventDefault();
			handler();
		}
	};
	window.addEventListener('keydown', listener);
	return () => window.removeEventListener('keydown', listener);
}

/**
 * Capture une vignette PNG (data URL) depuis le canvas de rendu, réduite pour
 * tenir dans `maxSize` px de côté. Renvoie `null` si la capture échoue.
 */
export function captureThumbnail(canvas: HTMLCanvasElement, maxSize = 256): string | null {
	const scale = Math.min(1, maxSize / Math.max(canvas.width, canvas.height, 1));
	const w = Math.max(1, Math.round(canvas.width * scale));
	const h = Math.max(1, Math.round(canvas.height * scale));
	const off = document.createElement('canvas');
	off.width = w;
	off.height = h;
	const ctx = off.getContext('2d');
	if (!ctx) return null;
	ctx.drawImage(canvas, 0, 0, w, h);
	try {
		return off.toDataURL('image/png');
	} catch {
		return null;
	}
}
