// Administration de l'instance (phase 32), typée sur les DTO générés. Tous ces
// appels exigent un compte `admin` (403 sinon).
import { api } from './client';
import type {
	AdminCreateUserRequest,
	InstanceResponse,
	InvitationResponse,
	PatchInstanceRequest,
	UserResponse
} from './types';

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
