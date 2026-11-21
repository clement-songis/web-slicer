// Compte connecté (lot 32) : changement de mot de passe et usage / quotas.
import { api, ApiError } from './client';
import { tr } from '$lib/i18n';
import type { ChangePasswordRequest, UsageResponse } from './types';

export const changePassword = (body: ChangePasswordRequest) => api.put<void>('/me/password', body);
export const getUsage = () => api.get<UsageResponse>('/me/usage');

/** Message d'un refus de PUT /me/password (403 actuel faux, 422 par champ). */
export function passwordChangeError(err: unknown): string {
	if (!(err instanceof ApiError)) return tr('Could not change the password.');
	const field = (err.details as { field?: unknown } | undefined)?.field;
	if (err.status === 403 || err.code === 'invalid_credentials')
		return tr('Incorrect current password.');
	if (err.status === 422 && field === 'new_password')
		return tr('The new password must be different from the current one.');
	if (err.status === 422 && field === 'password')
		return tr('New password too short (8 characters minimum).');
	if (err.status === 404) return tr('This server does not allow changing your password yet.');
	return err.message;
}

/** Contrôles locaux avant envoi (mêmes règles que le serveur). */
export function validatePasswordChange(
	current: string,
	next: string,
	confirm: string
): string | null {
	if (!current) return tr('Enter your current password.');
	if (next.length < 8) return tr('New password too short (8 characters minimum).');
	if (next === current) return tr('The new password must be different from the current one.');
	if (next !== confirm) return tr('The confirmation does not match.');
	return null;
}

/** Message de connexion refusée (401 invalid_credentials). */
export function loginErrorMessage(err: unknown): string {
	if (!(err instanceof ApiError)) return tr('Unexpected error');
	if (err.code === 'invalid_credentials' || err.status === 401)
		return tr('Incorrect email or password.');
	return err.message;
}
