import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { setLocale } from '$lib/i18n';
import { ApiError } from '$lib/api/client';
import { isNameConflict, nameSuggestion, nameTakenMessage, uniqueProjectName } from './names';

beforeAll(() => setLocale('fr'));
afterAll(() => setLocale('en'));

describe('409 name_taken (lot 32)', () => {
	it('lit details.suggestion et le propose dans le message', () => {
		const err = new ApiError(409, 'name_taken', 'pris', {
			field: 'name',
			name: 'cube',
			suggestion: 'cube (2)'
		});
		expect(isNameConflict(err)).toBe(true);
		expect(nameSuggestion(err)).toBe('cube (2)');
		expect(nameTakenMessage('cube', nameSuggestion(err))).toBe(
			"Un autre projet s'appelle déjà « cube ». Nom libre proposé : « cube (2) »."
		);
		expect(nameSuggestion(new ApiError(409, 'conflict', 'x'))).toBeNull();
		expect(nameSuggestion(new ApiError(422, 'name_taken', 'x', { suggestion: 'a' }))).toBeNull();
		expect(nameSuggestion(new Error('x'))).toBeNull();
	});
});

describe('noms de projet uniques', () => {
	it('garde le nom libre, suffixe sinon', () => {
		expect(uniqueProjectName('Sans titre', [])).toBe('Sans titre');
		expect(uniqueProjectName('Sans titre', ['Sans titre'])).toBe('Sans titre (2)');
		expect(uniqueProjectName('Sans titre', ['Sans titre', 'Sans titre (2)'])).toBe(
			'Sans titre (3)'
		);
	});
	it('repart de la racine et reste sensible à la casse', () => {
		expect(uniqueProjectName('cube (2)', ['cube', 'cube (2)'])).toBe('cube (3)');
		expect(uniqueProjectName('Cube', ['cube'])).toBe('Cube');
		expect(uniqueProjectName('  ', [])).toBe('Sans titre');
	});
	it('reconnaît un conflit de nom', () => {
		expect(isNameConflict(new ApiError(409, 'conflict', 'x'))).toBe(true);
		expect(
			isNameConflict(
				new ApiError(500, 'error', 'UNIQUE constraint failed: projects.user_id, projects.name')
			)
		).toBe(true);
		expect(isNameConflict(new ApiError(422, 'validation', 'x'))).toBe(false);
		expect(isNameConflict(new Error('x'))).toBe(false);
		expect(nameTakenMessage('cube')).toContain('« cube »');
	});
});
