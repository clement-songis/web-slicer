// Fichiers joints au projet (Orca : onglet « Fichiers auxiliaires », dossier
// Auxiliaries/ du 3MF) : les 4 dossiers d'Orca, regroupement de la liste du
// serveur, noms sûrs et aperçus. Pur → testable.
import type { AuxiliaryEntry } from '$lib/api/auxiliaries';

/** Dossiers d'Orca (noms exacts du 3MF), libellés et aide. */
export const AUX_FOLDERS = [
	{
		id: 'Model Pictures',
		label: 'Photos du modèle',
		hint: 'Images montrées avec le projet (PNG, JPEG…).',
		accept: 'image/*'
	},
	{
		id: 'Bill of Materials',
		label: 'Nomenclature',
		hint: 'Liste des pièces et matériaux.',
		accept: ''
	},
	{
		id: 'Assembly Guide',
		label: "Guide d'assemblage",
		hint: 'Instructions de montage (PDF, images…).',
		accept: ''
	},
	{ id: 'Others', label: 'Autres', hint: 'Tout autre fichier lié au projet.', accept: '' }
] as const;

export type AuxFolder = (typeof AUX_FOLDERS)[number]['id'];

export interface AuxFile {
	folder: AuxFolder;
	name: string;
	size: number;
}

const FOLDER_IDS = new Set<string>(AUX_FOLDERS.map((f) => f.id));

/** Liste du serveur → fichiers par dossier (ordre des dossiers d'Orca, noms triés). */
export function groupAuxiliaries(entries: readonly AuxiliaryEntry[]): Record<AuxFolder, AuxFile[]> {
	const out = Object.fromEntries(AUX_FOLDERS.map((f) => [f.id, [] as AuxFile[]])) as Record<
		AuxFolder,
		AuxFile[]
	>;
	for (const e of entries) {
		const slash = e.path.indexOf('/');
		if (slash < 0) continue;
		const folder = e.path.slice(0, slash);
		const name = e.path.slice(slash + 1);
		if (!FOLDER_IDS.has(folder) || !name || name.includes('/')) continue;
		out[folder as AuxFolder].push({ folder: folder as AuxFolder, name, size: Number(e.size) || 0 });
	}
	for (const files of Object.values(out)) files.sort((a, b) => a.name.localeCompare(b.name, 'fr'));
	return out;
}

/**
 * Nom accepté par le serveur (pas de séparateur, pas de « . » initial, non
 * vide) : séparateurs et caractères de contrôle remplacés par « _ ».
 */
export function safeAuxName(name: string): string {
	// eslint-disable-next-line no-control-regex
	const control = /[\u0000-\u001f]/g;
	let s = name.split(/[\\/]/).pop()!.replace(control, '_').trim();
	s = s.replace(/^\.+/, '');
	return s || 'fichier';
}

/** Nom libre dans le dossier (« photo.png » → « photo (2).png »). */
export function uniqueAuxName(name: string, taken: readonly string[]): string {
	const set = new Set(taken.map((n) => n.toLowerCase()));
	if (!set.has(name.toLowerCase())) return name;
	const dot = name.lastIndexOf('.');
	const stem = dot > 0 ? name.slice(0, dot) : name;
	const ext = dot > 0 ? name.slice(dot) : '';
	for (let i = 2; ; i++) {
		const candidate = `${stem} (${i})${ext}`;
		if (!set.has(candidate.toLowerCase())) return candidate;
	}
}

/** Aperçu en vignette (images affichables par le navigateur). */
export function isPreviewableImage(name: string): boolean {
	return /\.(png|jpe?g|gif|webp|bmp|svg)$/i.test(name);
}

/** Taille lisible (o, Ko, Mo). */
export function formatSize(bytes: number): string {
	if (bytes < 1024) return `${bytes} o`;
	if (bytes < 1024 * 1024)
		return `${(bytes / 1024).toLocaleString('fr-FR', { maximumFractionDigits: 1 })} Ko`;
	return `${(bytes / 1024 / 1024).toLocaleString('fr-FR', { maximumFractionDigits: 1 })} Mo`;
}

/** Limite du serveur par fichier (50 Mo). */
export const AUX_MAX_BYTES = 50 * 1024 * 1024;
