// Sauvegarde automatique (audit 45 PREF-b) — Orca Preferences « Auto backup »
// (backup_switch, défaut activé ; backup_interval, défaut 10 s ;
// Plater::priv::… has_restore_data → « Previous unsaved project detected, do
// you want to restore it? »). Version navigateur : tant que le projet a des
// modifications non enregistrées, la scène qui serait enregistrée (et les
// presets actifs) est recopiée périodiquement dans le stockage local ; à la
// réouverture du projet (onglet fermé, plantage, coupure), le brouillon est
// proposé s'il part de la version serveur courante et en diffère.

/** Clé de stockage du brouillon d'un projet. */
export const BACKUP_PREFIX = 'web-slicer.backup.';

/** Intervalle d'Orca par défaut (secondes) et bornes acceptées. */
export const DEFAULT_BACKUP_INTERVAL = 10;
export const MIN_BACKUP_INTERVAL = 1;
export const MAX_BACKUP_INTERVAL = 3600;

export interface ProjectBackup {
	/** Version serveur du projet sur laquelle portent les modifications. */
	version: string;
	/** Horodatage (ms) de la copie. */
	savedAt: number;
	/** Document scène tel que l'enregistrement l'enverrait. */
	scene: unknown;
	/** Presets actifs sérialisés (dont réglages modifiés non enregistrés). */
	activePresets: unknown;
}

/** Intervalle borné (secondes entières). */
export function clampBackupInterval(v: unknown): number {
	const n = Math.round(Number(v));
	if (!Number.isFinite(n)) return DEFAULT_BACKUP_INTERVAL;
	return Math.min(MAX_BACKUP_INTERVAL, Math.max(MIN_BACKUP_INTERVAL, n));
}

function storage(): Storage | null {
	try {
		return typeof localStorage === 'undefined' ? null : localStorage;
	} catch {
		return null;
	}
}

export function writeBackup(projectId: string, backup: ProjectBackup): boolean {
	const s = storage();
	if (!s) return false;
	try {
		s.setItem(BACKUP_PREFIX + projectId, JSON.stringify(backup));
		return true;
	} catch {
		// Quota dépassé : pas de brouillon plutôt qu'une erreur.
		return false;
	}
}

export function readBackup(projectId: string): ProjectBackup | null {
	const raw = storage()?.getItem(BACKUP_PREFIX + projectId);
	if (!raw) return null;
	try {
		const b = JSON.parse(raw) as Partial<ProjectBackup>;
		if (typeof b.version !== 'string' || typeof b.savedAt !== 'number' || !b.scene) return null;
		return b as ProjectBackup;
	} catch {
		return null;
	}
}

export function clearBackup(projectId: string): void {
	try {
		storage()?.removeItem(BACKUP_PREFIX + projectId);
	} catch {
		// Stockage indisponible : rien à effacer.
	}
}

/**
 * Brouillon à proposer à l'ouverture : même version serveur (sinon le projet a
 * été enregistré depuis, ailleurs ou ici : brouillon périmé) et contenu
 * différent de ce qui vient d'être chargé.
 */
export function shouldOfferRestore(
	backup: ProjectBackup | null,
	serverVersion: string,
	serverScene: unknown,
	serverPresets: unknown
): boolean {
	if (!backup || backup.version !== serverVersion) return false;
	return (
		JSON.stringify(backup.scene) !== JSON.stringify(serverScene) ||
		JSON.stringify(backup.activePresets ?? null) !== JSON.stringify(serverPresets ?? null)
	);
}
