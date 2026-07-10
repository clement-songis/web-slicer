// Appels d'authentification, typés sur les DTO générés.
import { api } from './client';
import type { LoginRequest, RegisterRequest, UserResponse } from './types';

export const login = (body: LoginRequest) => api.post<UserResponse>('/auth/login', body);
export const register = (body: RegisterRequest) => api.post<UserResponse>('/auth/register', body);
export const logout = () => api.post<void>('/auth/logout');
export const me = () => api.get<UserResponse>('/auth/me');
export const deleteAccount = (password: string) => api.del<void>('/auth/me', { password });
