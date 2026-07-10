// Réexport centralisé des types d'API générés par ts-rs (source unique côté
// backend). Les composants et stores importent d'ici, jamais des chemins
// profonds `src/generated/…`.
export type { UserResponse } from '../../generated/api/UserResponse';
export type { LoginRequest } from '../../generated/api/LoginRequest';
export type { RegisterRequest } from '../../generated/api/RegisterRequest';
export type { ProjectResponse } from '../../generated/api/ProjectResponse';
export type { CreateProjectRequest } from '../../generated/api/CreateProjectRequest';
export type { ErrorBody } from '../../generated/api/ErrorBody';
