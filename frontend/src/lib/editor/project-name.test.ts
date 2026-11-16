import { describe, expect, it } from 'vitest';
import { isDefaultProjectName, projectNameFromFile } from './project-name';

describe('nom de projet (parité Orca « Untitled » → premier modèle)', () => {
	it('reconnaît les noms par défaut', () => {
		expect(isDefaultProjectName('Sans titre')).toBe(true);
		expect(isDefaultProjectName(' Nouveau projet ')).toBe(true);
		expect(isDefaultProjectName('Untitled')).toBe(true);
		expect(isDefaultProjectName('Support de casque')).toBe(false);
	});
	it('stem du fichier', () => {
		expect(projectNameFromFile('cube20.stl')).toBe('cube20');
		expect(projectNameFromFile('Benchy.v2.3mf')).toBe('Benchy.v2');
		expect(projectNameFromFile('dir/sub\\part.STEP')).toBe('part');
		expect(projectNameFromFile('.hidden')).toBe('.hidden');
	});
});
