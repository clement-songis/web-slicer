import { describe, expect, it } from 'vitest';
import { gcodeThumbnailOverrides, thumbnailFirstFormat } from './preview';

// Audit 48 DLG48-b — la préférence « Vignettes G-code » rejoint le tranchage
// comme surcharge printer de la requête (thumbnails + thumbnails_format
// dérivée, règle Tab.cpp:4510-4533). Vide → null (défauts du moteur, rien
// écrit) ; réglage machine explicite (≠ défaut du registre) → il prime.
describe('surcharges vignettes de la requête de tranchage (DLG48-b)', () => {
	it('préférence vide → null : les défauts ne sont jamais écrits', () => {
		expect(gcodeThumbnailOverrides('', undefined)).toBeNull();
		expect(gcodeThumbnailOverrides('  ', '48x48/PNG,300x300/PNG')).toBeNull();
	});

	it('préférence renseignée, machine au défaut → paire thumbnails', () => {
		expect(gcodeThumbnailOverrides('96x96/PNG', undefined)).toEqual({
			thumbnails: '96x96/PNG',
			thumbnails_format: 'PNG'
		});
		expect(gcodeThumbnailOverrides('96x96/QOI,300x300/PNG', '48x48/PNG,300x300/PNG')).toEqual({
			thumbnails: '96x96/QOI,300x300/PNG',
			thumbnails_format: 'QOI'
		});
	});

	it('réglage machine explicite (≠ défaut) → il prime, pas de surcharge', () => {
		expect(gcodeThumbnailOverrides('96x96/PNG', '96x96/QOI')).toBeNull();
		// « » = vignettes désactivées à la machine : choix explicite aussi.
		expect(gcodeThumbnailOverrides('96x96/PNG', '')).toBeNull();
	});

	it('thumbnails_format : première entrée, extension absente ou inconnue → PNG', () => {
		expect(thumbnailFirstFormat('96x96/PNG,300x300/PNG')).toBe('PNG');
		expect(thumbnailFirstFormat('96x96/JPG')).toBe('JPG');
		expect(thumbnailFirstFormat('96x96')).toBe('PNG');
		expect(thumbnailFirstFormat('96x96/BTT_TFT')).toBe('BTT_TFT');
		expect(thumbnailFirstFormat('96x96/ZIP')).toBe('PNG');
	});
});
