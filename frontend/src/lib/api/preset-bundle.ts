// Phase 46 (audit 44 PR1-a/b) — lots de préréglages au format d'OrcaSlicer,
// produits et lus par le serveur (lot 46, routes/preset_bundle.rs) :
// - POST /presets/export-bundle `{ presets: [{ id, values? }] }` → archive
//   `<imprimante>.orca_printer` / `<filament>.orca_filament` / `Presets.zip`
//   (fichiers Orca : clés du type, différences avec le parent système,
//   `from: "User"`, `version`, `bundle_structure.json`) ; un preset système
//   actif est exporté en preset autonome (lot 46c : config complète du type,
//   `inherits: ""`), donc les presets actifs système se téléchargent aussi ;
// - POST /presets/import-bundle?filename=… (octets bruts) → `{ imported,
//   skipped }` (PresetBundle::import_json_presets : clés étrangères au type
//   retirées, différences avec le parent stockées).
// `null` = serveur sans ces routes (déploiement antérieur) : l'appelant
// retombe sur l'ancien chemin.
import { API_BASE, ApiError } from './client';
import type { ErrorBody, PresetDetail } from './types';
import { apiErrorMessage } from './quota';

export interface BundleExportEntry {
	id: string;
	/** Valeurs éditées non enregistrées du preset (Orca « (modifié) »). */
	values?: Record<string, unknown>;
}

export interface BundleImportResult {
	imported: PresetDetail[];
	skipped: { file: string; name?: string | null; reason: string }[];
}

/** Nom de fichier d'un en-tête Content-Disposition (filename* UTF-8 prioritaire). */
export function dispositionFilename(header: string | null, fallback: string): string {
	if (!header) return fallback;
	const star = /filename\*=UTF-8''([^;]+)/i.exec(header);
	if (star) {
		try {
			return decodeURIComponent(star[1].trim());
		} catch {
			// encodage invalide : nom simple
		}
	}
	const plain = /filename="([^"]+)"/i.exec(header) ?? /filename=([^;]+)/i.exec(header);
	return plain ? plain[1].trim() : fallback;
}

async function failure(res: Response): Promise<ApiError> {
	let body: Partial<ErrorBody> = {};
	try {
		body = (await res.json()) as Partial<ErrorBody>;
	} catch {
		// corps non JSON
	}
	return new ApiError(
		res.status,
		body.code ?? 'error',
		apiErrorMessage(res.status, body, res.statusText),
		body.details
	);
}

const missing = (status: number) => status === 404 || status === 405;

/** Lot exporté par le serveur, ou `null` si la route n'existe pas. */
export async function exportPresetBundleArchive(
	presets: BundleExportEntry[]
): Promise<{ blob: Blob; filename: string } | null> {
	const res = await fetch(`${API_BASE}/presets/export-bundle`, {
		method: 'POST',
		credentials: 'include',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ presets })
	});
	if (missing(res.status)) return null;
	if (!res.ok) throw await failure(res);
	return {
		blob: await res.blob(),
		filename: dispositionFilename(res.headers.get('content-disposition'), 'Presets.zip')
	};
}

/** Fichier de préréglages (.json, .zip, .orca_printer…) importé par le serveur. */
export async function importPresetBundleFile(file: File): Promise<BundleImportResult | null> {
	const res = await fetch(
		`${API_BASE}/presets/import-bundle?filename=${encodeURIComponent(file.name)}`,
		{
			method: 'POST',
			credentials: 'include',
			headers: { 'content-type': 'application/octet-stream' },
			body: file
		}
	);
	if (missing(res.status)) return null;
	if (!res.ok) throw await failure(res);
	const data = (await res.json()) as Partial<BundleImportResult>;
	return { imported: data.imported ?? [], skipped: data.skipped ?? [] };
}
