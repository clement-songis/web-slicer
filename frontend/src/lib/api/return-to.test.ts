import { describe, expect, it } from 'vitest';
import { loginUrl, safeNext } from './return-to';

describe('retour après connexion', () => {
	it('accepte les chemins internes seulement', () => {
		expect(safeNext('/queue')).toBe('/queue');
		expect(safeNext('/projects/abc?x=1')).toBe('/projects/abc?x=1');
		expect(safeNext('https://evil.example')).toBeNull();
		expect(safeNext('//evil.example')).toBeNull();
		expect(safeNext('/\\evil')).toBeNull();
		expect(safeNext('/login')).toBeNull();
		expect(safeNext(null)).toBeNull();
	});
	it('construit l’URL de connexion', () => {
		expect(loginUrl('/queue')).toBe('/login?next=%2Fqueue');
		expect(loginUrl('/')).toBe('/login');
		expect(loginUrl(undefined)).toBe('/login');
	});
});
