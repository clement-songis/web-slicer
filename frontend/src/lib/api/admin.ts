// Administration de l'instance (phase 32), typée sur les DTO générés. Tous ces
// appels exigent un compte `admin` (403 sinon).
import { api } from './client';
import type {
	AdminCreateUserRequest,
	AdminUserListResponse,
	AdminUsersQuery,
	InstanceResponse,
	InvitationResponse,
	PatchInstanceRequest,
	UserResponse
} from './types';

/** Liste des comptes (lot 32) : page, taille de page, filtre e-mail. */
export function listUsers(query: AdminUsersQuery = {}) {
	const p = new URLSearchParams();
	if (query.page) p.set('page', String(query.page));
	if (query.per_page) p.set('per_page', String(query.per_page));
	if (query.q?.trim()) p.set('q', query.q.trim());
	const qs = p.toString();
	return api.get<AdminUserListResponse>(`/admin/users${qs ? '?' + qs : ''}`);
}
export const getInstance = () => api.get<InstanceResponse>('/admin/instance');
export const patchInstance = (body: PatchInstanceRequest) =>
	api.patch<InstanceResponse>('/admin/instance', body);
export const createUser = (body: AdminCreateUserRequest) =>
	api.post<UserResponse>('/admin/users', body);
export const resetPassword = (id: string, newPassword: string) =>
	api.post<void>(`/admin/users/${id}/reset-password`, { new_password: newPassword });
export const deleteUser = (id: string) => api.del<void>(`/admin/users/${id}`);
export const createInvitation = (validDays: number) =>
	api.post<InvitationResponse>('/admin/invitations', { valid_days: validDays });
