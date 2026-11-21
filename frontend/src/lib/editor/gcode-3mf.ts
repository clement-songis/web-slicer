// « Ouvrir un G-code » sur un fichier tranché .gcode.3mf (phase 44b, audit 43
// X-f) : archive ZIP sans géométrie (Orca `store_bbs_3mf` en SkipModel) avec
// le G-code de chaque plateau tranché dans `Metadata/plate_<n>.gcode`. Le
// serveur (lot 44) extrait le plateau `?plate=<n>` pour l'aperçu et la
// simulation ; le client relit ici le même G-code pour la fenêtre G-code et
// les statistiques de repli, et la liste des plateaux tranchés.
import { unzipSync } from 'fflate';
import type { ApiError } from '$lib/api/client';

/** En-tête local d'une archive ZIP (`PK\\x03\\x04`). */
export function isZipBytes(bytes: Uint8Array): boolean {
	return (
		bytes.length >= 4 && bytes[0] === 0x50 && bytes[1] === 0x4b && bytes[2] === 3 && bytes[3] === 4
	);
}

/** Nom de fichier d'un 3MF (projet ou fichier tranché). */
export function is3mfName(name: string): boolean {
	return /\.3mf$/i.test(name);
}

const PLATE_ENTRY = /^Metadata\/plate_(\d+)\.gcode$/;

/** Plateaux tranchés d'une archive (`Metadata/plate_<n>.gcode`), triés, sans doublon. */
export function archivePlates(names: Iterable<string>): number[] {
	const plates = new Set<number>();
	for (const name of names) {
		const m = PLATE_ENTRY.exec(name);
		if (m) plates.add(Number(m[1]));
	}
	return [...plates].sort((a, b) => a - b);
}

async function bytesOf(file: Blob): Promise<Uint8Array> {
	return new Uint8Array(await file.arrayBuffer());
}

/** Le fichier est-il une archive ZIP ? (4 premiers octets seulement : un G-code
 *  texte de plusieurs centaines de Mo n'est pas relu pour rien). */
export async function isZipFile(file: Blob): Promise<boolean> {
	return isZipBytes(new Uint8Array(await file.slice(0, 4).arrayBuffer()));
}

/** Noms des entrées d'une archive, sans rien décompresser. */
function entryNames(bytes: Uint8Array): string[] {
	const names: string[] = [];
	unzipSync(bytes, {
		filter: (f) => {
			names.push(f.name);
			return false;
		}
	});
	return names;
}

/** Plateaux tranchés d'un fichier (`[]` pour un G-code texte ou un 3MF sans G-code). */
export async function gcodeFilePlates(file: Blob): Promise<number[]> {
	if (!(await isZipFile(file))) return [];
	try {
		return archivePlates(entryNames(await bytesOf(file)));
	} catch {
		return [];
	}
}

/**
 * Texte G-code d'un fichier ouvert : le fichier lui-même, ou le G-code du
 * plateau `plate` (à défaut le premier tranché, comme le serveur) d'un
 * .gcode.3mf. Archive sans G-code de plateau → chaîne vide.
 */
export async function gcodeFileText(file: Blob, plate?: number | null): Promise<string> {
	if (!(await isZipFile(file))) return file.text();
	const bytes = await bytesOf(file);
	try {
		const plates = archivePlates(entryNames(bytes));
		const chosen = plate && plates.includes(plate) ? plate : plates[0];
		if (chosen === undefined) return '';
		const name = `Metadata/plate_${chosen}.gcode`;
		const entry = unzipSync(bytes, { filter: (f) => f.name === name })[name];
		return entry ? new TextDecoder().decode(entry) : '';
	} catch {
		// Archive illisible : le serveur l'a déjà signalée (422).
		return '';
	}
}

/** Détails de l'erreur 422 d'un projet ouvert qui est un fichier tranché (lot 44). */
export interface Gcode3mfRefusal {
	plates: number[];
}

/**
 * `POST /api/projects/import` refuse un .gcode.3mf (aucun objet) par un 422
 * `details.kind = "gcode_3mf"` : renvoie ses plateaux tranchés, sinon `null`.
 */
export function gcode3mfRefusal(error: unknown): Gcode3mfRefusal | null {
	const e = error as Partial<ApiError> | null;
	if (!e || typeof e !== 'object' || e.status !== 422) return null;
	const details = e.details as { kind?: unknown; plates?: unknown } | undefined;
	if (!details || details.kind !== 'gcode_3mf') return null;
	const plates = Array.isArray(details.plates)
		? details.plates.filter((p): p is number => Number.isInteger(p) && (p as number) > 0)
		: [];
	return { plates };
}
