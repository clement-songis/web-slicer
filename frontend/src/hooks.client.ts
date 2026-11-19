// Erreurs inattendues côté client (phase 32) : un `load` qui échoue faute de
// serveur (ApiError réseau / 5xx du proxy) affichait « 500 Internal Error » sans
// explication. Le message lisible et le code de l'ApiError sont transmis à la
// page d'erreur (+error.svelte), qui propose de réessayer.
import type { HandleClientError } from '@sveltejs/kit';
import { ApiError } from '$lib/api/client';

export const handleError: HandleClientError = ({ error, message }) => {
	if (error instanceof ApiError) return { message: error.message, code: error.code };
	return { message };
};
