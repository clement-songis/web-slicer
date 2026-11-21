// Fichiers joints au projet (Orca : onglet « Fichiers auxiliaires », dossier
// Auxiliaries/ du 3MF) : les 4 dossiers d'Orca, regroupement de la liste du
// serveur, noms sûrs et aperçus. Pur → testable.
import type { AuxiliaryEntry } from '$lib/api/auxiliaries';
import { numberLocale, tr } from '$lib/i18n';

/**
 * Dossiers d'Orca (noms exacts du 3MF), libellés et aide — clés anglaises,
 * traduites à l'affichage (`$t(f.label)`).
 */
export const AUX_FOLDERS = [
	{
		id: 'Model Pictures',
		label: 'Model Pictures',
		hint: 'Images shown with the project (PNG, JPEG…).',
		accept: 'image/*'
	},
	{
		id: 'Bill of Materials',
		label: 'Bill of Materials',
		hint: 'List of parts and materials.',
		accept: ''
	},
	{
		id: 'Assembly Guide',
		label: 'Assembly Guide',
		hint: 'Assembly instructions (PDF, images…).',
		accept: ''
	},
	{ id: 'Others', label: 'Others', hint: 'Any other file related to the project.', accept: '' }
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
	if (bytes < 1024) return tr('{n} B', { n: bytes });
	const fmt = (v: number) => v.toLocaleString(numberLocale(), { maximumFractionDigits: 1 });
	if (bytes < 1024 * 1024) return tr('{n} KB', { n: fmt(bytes / 1024) });
	return tr('{n} MB', { n: fmt(bytes / 1024 / 1024) });
}

/** Limite du serveur par fichier (50 Mo). */
export const AUX_MAX_BYTES = 50 * 1024 * 1024;
