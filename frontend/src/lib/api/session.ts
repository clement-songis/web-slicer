// État de session partagé et garde d'authentification. Le compte courant est un
// store Svelte ; les vues s'y abonnent, la logique d'appel reste dans `auth`.
import { writable } from 'svelte/store';
import * as auth from './auth';
import { ApiError } from './client';
import type { UserResponse } from './types';

/** Compte connecté, ou `null` si la session est absente/expirée. */
export const currentUser = writable<UserResponse | null>(null);

/**
 * Recharge la session depuis le backend (`GET /auth/me`). Met à jour le store et
 * renvoie le compte, ou `null` sur 401. Toute autre erreur est propagée.
 */
export async function refreshSession(): Promise<UserResponse | null> {
	try {
		const user = await auth.me();
		currentUser.set(user);
		return user;
	} catch (e) {
		if (e instanceof ApiError && e.status === 401) {
			currentUser.set(null);
			return null;
		}
		throw e;
	}
}

export async function login(email: string, password: string): Promise<UserResponse> {
	const user = await auth.login({ email, password });
	currentUser.set(user);
	return user;
}

export async function register(
	email: string,
	password: string,
	inviteToken?: string
): Promise<UserResponse> {
	const user = await auth.register({
		email,
		password,
		...(inviteToken ? { invite_token: inviteToken } : {})
	});
	currentUser.set(user);
	return user;
}

export async function logout(): Promise<void> {
	await auth.logout();
	currentUser.set(null);
}
