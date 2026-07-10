// Réexport centralisé des types d'API générés par ts-rs (source unique côté
// backend). Les composants et stores importent d'ici, jamais des chemins
// profonds `src/generated/…`.
export type { UserResponse } from '../../generated/api/UserResponse';
export type { LoginRequest } from '../../generated/api/LoginRequest';
export type { RegisterRequest } from '../../generated/api/RegisterRequest';
export type { ProjectResponse } from '../../generated/api/ProjectResponse';
export type { CreateProjectRequest } from '../../generated/api/CreateProjectRequest';
export type { ModelResponse } from '../../generated/api/ModelResponse';
export type { ArrangeItem } from '../../generated/api/ArrangeItem';
export type { ArrangeRequest } from '../../generated/api/ArrangeRequest';
export type { ArrangeResponse } from '../../generated/api/ArrangeResponse';
export type { Placement } from '../../generated/api/Placement';
export type { OrientResponse } from '../../generated/api/OrientResponse';
export type { RepairResponse } from '../../generated/api/RepairResponse';
export type { ErrorBody } from '../../generated/api/ErrorBody';
export type { PresetSummary } from '../../generated/api/PresetSummary';
export type { PresetDetail } from '../../generated/api/PresetDetail';
export type { ResolvedPreset } from '../../generated/api/ResolvedPreset';
export type { CreatePresetRequest } from '../../generated/api/CreatePresetRequest';
export type { UpdatePresetRequest } from '../../generated/api/UpdatePresetRequest';
export type { ImportPresetRequest } from '../../generated/api/ImportPresetRequest';
