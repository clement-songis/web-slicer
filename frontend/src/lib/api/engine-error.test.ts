import { describe, expect, test } from 'vitest';
import { isTechnicalEngineMessage, readableServerMessage } from './engine-error';
import { ApiError } from './client';

const RAW =
	"3MF projet illisible : lecture libslic3r : worker sorti en code 1 — stderr :  — stdout : E invalid_model The supplied file couldn't be read because it's empty\n";

describe('messages serveur lisibles (phase 38)', () => {
	test('3MF vide : contexte gardé, cause lisible, sans trace du worker', () => {
		expect(isTechnicalEngineMessage(RAW)).toBe(true);
		expect(readableServerMessage(RAW)).toBe(
			'3MF projet illisible : le fichier ne contient aucun objet.'
		);
	});

	test('autres causes et messages ordinaires inchangés', () => {
		expect(
			readableServerMessage(
				'lecture libslic3r : worker sorti en code 2 — stdout : E invalid_model bad xml'
			)
		).toBe('Fichier illisible : le fichier est endommagé ou n’est pas un modèle valide.');
		expect(readableServerMessage('Session requise')).toBe('Session requise');
		expect(readableServerMessage('')).toBe('');
	});

	test('ApiError : message lisible, original dans technical', () => {
		const e = new ApiError(422, 'validation', RAW);
		expect(e.message).toBe('3MF projet illisible : le fichier ne contient aucun objet.');
		expect(e.technical).toBe(RAW);
		expect(new ApiError(500, 'internal', 'Boom').message).toBe('Boom');
	});
});
