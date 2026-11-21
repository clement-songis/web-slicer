// Lot 42 (M2, M3) : nom de fichier choisi à l'envoi (Orca PrintHostSendDialog)
// et historique des envois (Orca PrintHostQueueDialog, « Print host upload
// queue »). Logique pure : le transport vit dans `lib/api/printers`.
import { ApiError } from '$lib/api/client';
import type { PrinterUploadEntry } from '$lib/api/types';
import { numberLocale, tr } from '$lib/i18n';

/** Raisons de refus d'un nom d'envoi (details.reason du 422 serveur). */
export type UploadNameReason = 'chars' | 'empty' | 'hidden' | 'too_long';

/** Caractères refusés par le serveur (gcode_filename::upload_name). */
export const FORBIDDEN_NAME_CHARS = '/ \\ : * ? " < > |';
// eslint-disable-next-line no-control-regex
const FORBIDDEN = /[/\\:*?"<>|\u0000-\u001f\u007f]/;
/** Longueur maximale (octets UTF-8) du nom complet, extension comprise. */
export const MAX_NAME_BYTES = 200;

/**
 * Contrôle local du nom saisi (mêmes règles que le serveur) : retour immédiat
 * pendant la frappe ; le serveur reste l'arbitre (POST …/upload-name).
 */
export function uploadNameProblem(stem: string, extension = ''): UploadNameReason | null {
	const name = stem.trim();
	if (!name) return 'empty';
	if (FORBIDDEN.test(name)) return 'chars';
	if (name.startsWith('.')) return 'hidden';
	if (new TextEncoder().encode(joinUploadName(name, extension)).length > MAX_NAME_BYTES)
		return 'too_long';
	return null;
}

/** Message lisible d'une raison de refus. */
export function uploadNameReasonMessage(reason: string): string {
	switch (reason) {
		case 'chars':
			return tr('The file name contains forbidden characters: {chars}', {
				chars: FORBIDDEN_NAME_CHARS
			});
		case 'empty':
			return tr('Enter a file name.');
		case 'hidden':
			return tr('The file name cannot start with a dot.');
		case 'too_long':
			return tr('The file name is too long ({n} bytes max).', { n: MAX_NAME_BYTES });
		default:
			return tr('Invalid file name.');
	}
}

/** Raison d'un 422 serveur portant sur le nom de fichier, sinon null. */
export function uploadNameErrorReason(e: unknown): string | null {
	if (!(e instanceof ApiError) || e.status !== 422) return null;
	const d = e.details as { field?: string; reason?: string } | null | undefined;
	return d?.field === 'filename' ? (d.reason ?? '') : null;
}

/** Partie modifiable d'un nom proposé (Orca : nom sans l'extension imposée). */
export function uploadNameStem(filename: string, extension: string): string {
	if (extension && filename.toLowerCase().endsWith(extension.toLowerCase()))
		return filename.slice(0, filename.length - extension.length);
	return filename;
}

/**
 * Nom envoyé : partie saisie + extension imposée (comme Orca, l'extension du
 * nom proposé est rétablie si elle manque ; déjà présente → inchangé).
 */
export function joinUploadName(stem: string, extension: string): string {
	const name = stem.trim();
	if (!extension || name.toLowerCase().endsWith(extension.toLowerCase())) return name;
	return name + extension;
}

/** État d'une ligne d'historique (Orca PrintHostQueueDialog, JobState). */
export function uploadStatusLabel(status: string): string {
	switch (status) {
		case 'uploading':
			return tr('Uploading');
		case 'completed':
			return tr('Completed');
		case 'error':
			return tr('Error');
		default:
			return status;
	}
}

/** Action demandée (Orca : Upload / Upload and Print / Upload to Queue). */
export function uploadActionLabel(action: string): string {
	switch (action) {
		case 'print':
			return tr('Upload and Print');
		case 'queue':
			return tr('Upload to Queue');
		default:
			return tr('Upload');
	}
}

/** Taille lisible (Orca : colonne « Size »), séparateur décimal de la langue. */
export function formatUploadSize(bytes: number | bigint): string {
	const n = Number(bytes);
	if (!Number.isFinite(n) || n <= 0) return '—';
	const fmt = (v: number) =>
		v.toLocaleString(numberLocale(), { minimumFractionDigits: 1, maximumFractionDigits: 1 });
	if (n < 1024) return tr('{n} B', { n });
	if (n < 1024 * 1024) return tr('{n} KB', { n: fmt(n / 1024) });
	return tr('{n} MB', { n: fmt(n / (1024 * 1024)) });
}

/** Date et heure de l'envoi au format de la langue. */
export function formatUploadDate(iso: string): string {
	const d = new Date(iso);
	if (Number.isNaN(d.getTime())) return iso;
	return d.toLocaleString(numberLocale(), {
		year: 'numeric',
		month: '2-digit',
		day: '2-digit',
		hour: '2-digit',
		minute: '2-digit'
	});
}

/** Au moins un envoi encore en cours (rafraîchissement automatique). */
export function hasPendingUploads(entries: readonly PrinterUploadEntry[]): boolean {
	return entries.some((e) => e.status === 'uploading');
}

/** Imprimantes présentes dans une liste (pour vider la vue « toutes »). */
export function uploadPrinterIds(entries: readonly PrinterUploadEntry[]): string[] {
	return [...new Set(entries.map((e) => e.printer_id))];
}
