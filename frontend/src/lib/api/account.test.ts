import { describe, expect, it } from 'vitest';
import { ApiError } from './client';
import { loginErrorMessage, passwordChangeError, validatePasswordChange } from './account';

describe('compte (lot 32)', () => {
	it('connexion refusée : invalid_credentials ou 401 générique', () => {
		expect(loginErrorMessage(new ApiError(401, 'invalid_credentials', 'x'))).toBe(
			'E-mail ou mot de passe incorrect.'
		);
		expect(loginErrorMessage(new ApiError(401, 'unauthorized', 'Session requise'))).toBe(
			'E-mail ou mot de passe incorrect.'
		);
		expect(loginErrorMessage(new ApiError(0, 'network', 'Serveur injoignable'))).toBe(
			'Serveur injoignable'
		);
		expect(loginErrorMessage(new Error('x'))).toBe('Erreur inattendue');
	});

	it('changement de mot de passe : erreurs serveur par cas', () => {
		expect(
			passwordChangeError(
				new ApiError(403, 'invalid_credentials', 'x', { field: 'current_password' })
			)
		).toBe('Mot de passe actuel incorrect.');
		expect(
			passwordChangeError(new ApiError(422, 'validation', 'x', { field: 'password' }))
		).toMatch(/trop court/);
		expect(
			passwordChangeError(new ApiError(422, 'validation', 'x', { field: 'new_password' }))
		).toMatch(/différent/);
		expect(passwordChangeError(new ApiError(404, 'not_found', 'x'))).toMatch(
			/ne permet pas encore/
		);
		expect(passwordChangeError(new ApiError(500, 'internal', 'Boom'))).toBe('Boom');
	});

	it('contrôles locaux', () => {
		expect(validatePasswordChange('', 'abcdefgh', 'abcdefgh')).toMatch(/actuel/);
		expect(validatePasswordChange('old', 'short', 'short')).toMatch(/trop court/);
		expect(validatePasswordChange('samesame', 'samesame', 'samesame')).toMatch(/différent/);
		expect(validatePasswordChange('old', 'abcdefgh', 'abcdefgX')).toMatch(/confirmation/);
		expect(validatePasswordChange('old', 'abcdefgh', 'abcdefgh')).toBeNull();
	});
});
