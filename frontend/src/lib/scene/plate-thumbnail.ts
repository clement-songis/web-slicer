// Vignettes de plateau du 3MF (phase 28, lot 26) : comme « Save project »
// d'Orca, chaque plateau porte Metadata/plate_<n>.png (512 × 512). La vue 3D
// enregistre ici une source de capture (rendu forcé puis canvas WebGL) ; la
// page envoie la capture du plateau actif avant l'export
// (PUT /api/projects/{id}/plates/{index}/thumbnail, PNG brut, 204).
import { API_BASE, ApiError } from '../api/client';
import { tr } from '$lib/i18n';

type CaptureSource = () => HTMLCanvasElement | null;
let source: CaptureSource | null = null;

/** Enregistré par la vue 3D (null au démontage). */
export function setCaptureSource(fn: CaptureSource | null): void {
	source = fn;
}

/** Carré centré (côté = plus petite dimension) du canvas, en px source. */
export function squareCrop(
	width: number,
	height: number
): { sx: number; sy: number; side: number } {
	const side = Math.max(1, Math.min(width, height));
	return { sx: Math.max(0, (width - side) / 2), sy: Math.max(0, (height - side) / 2), side };
}

/** Capture PNG carrée size × size de la vue 3D, ou null (pas de vue, échec). */
export async function capturePlatePng(size = 512): Promise<Blob | null> {
	const canvas = source?.();
	if (!canvas || canvas.width === 0 || canvas.height === 0) return null;
	const off = document.createElement('canvas');
	off.width = size;
	off.height = size;
	const ctx = off.getContext('2d');
	if (!ctx) return null;
	const { sx, sy, side } = squareCrop(canvas.width, canvas.height);
	try {
		ctx.drawImage(canvas, sx, sy, side, side, 0, 0, size, size);
	} catch {
		return null;
	}
	return new Promise((resolve) => off.toBlob((b) => resolve(b), 'image/png'));
}

/** Envoie la vignette d'un plateau (index 0-based). */
export async function putPlateThumbnail(
	projectId: string,
	index: number,
	png: Blob
): Promise<void> {
	const res = await fetch(API_BASE + '/projects/' + projectId + '/plates/' + index + '/thumbnail', {
		method: 'PUT',
		credentials: 'include',
		headers: { 'content-type': 'image/png' },
		body: png
	});
	if (!res.ok)
		throw new ApiError(
			res.status,
			'thumbnail',
			tr('thumbnail rejected ({status})', { status: res.status })
		);
}
