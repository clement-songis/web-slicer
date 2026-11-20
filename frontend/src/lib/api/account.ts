// Compte connecté (lot 32) : changement de mot de passe et usage / quotas.
import { api, ApiError } from './client';
import type { ChangePasswordRequest, UsageResponse } from './types';

export const changePassword = (body: ChangePasswordRequest) => api.put<void>('/me/password', body);
export const getUsage = () => api.get<UsageResponse>('/me/usage');

/** Message d'un refus de PUT /me/password (403 actuel faux, 422 par champ). */
export function passwordChangeError(err: unknown): string {
	if (!(err instanceof ApiError)) return 'Changement impossible.';
	const field = (err.details as { field?: unknown } | undefined)?.field;
	if (err.status === 403 || err.code === 'invalid_credentials')
		return 'Mot de passe actuel incorrect.';
	if (err.status === 422 && field === 'new_password')
		return 'Le nouveau mot de passe doit être différent de l’actuel.';
	if (err.status === 422 && field === 'password')
		return 'Nouveau mot de passe trop court (8 caractères minimum).';
	if (err.status === 404) return 'Ce serveur ne permet pas encore de changer son mot de passe.';
	return err.message;
}

/** Contrôles locaux avant envoi (mêmes règles que le serveur). */
export function validatePasswordChange(
	current: string,
	next: string,
	confirm: string
): string | null {
	if (!current) return 'Saisissez votre mot de passe actuel.';
	if (next.length < 8) return 'Nouveau mot de passe trop court (8 caractères minimum).';
	if (next === current) return 'Le nouveau mot de passe doit être différent de l’actuel.';
	if (next !== confirm) return 'La confirmation ne correspond pas.';
	return null;
}

/** Message de connexion refusée (401 invalid_credentials). */
export function loginErrorMessage(err: unknown): string {
	if (!(err instanceof ApiError)) return 'Erreur inattendue';
	if (err.code === 'invalid_credentials' || err.status === 401)
		return 'E-mail ou mot de passe incorrect.';
	return err.message;
}
