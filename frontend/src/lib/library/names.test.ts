import { describe, expect, it } from 'vitest';
import { ApiError } from '$lib/api/client';
import { isNameConflict, nameTakenMessage, uniqueProjectName } from './names';

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
