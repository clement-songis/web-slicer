# Contrat — API HTTP + WebSocket (backend/http)

Handlers minces (constitution I) : validation DTO → appel domaine → réponse.
DTO annotés `ts-rs` → `frontend/src/generated/` (client typé). Auth par cookie
de session (tower-sessions) ; toutes les routes sauf `/api/auth/*` et
`/api/health` exigent une session. Erreurs : enveloppe
`{ code, message, details? }`, 401 sans session, 403 jamais utilisé pour
masquer l'existence (404 pour les ressources d'autrui — SC-008).

## Auth & instance

| Méthode | Route | Rôle |
|---|---|---|
| POST | `/api/auth/register` | selon `registration_policy` (open/invite+token/closed→403) |
| POST | `/api/auth/login` / `POST /api/auth/logout` | session |
| GET | `/api/auth/me` | profil courant |
| DELETE | `/api/auth/me` | suppression de compte (confirmation mot de passe) — cascade BDD + purge fichiers (edge case spec) |
| DELETE | `/api/admin/users/{id}` | suppression par l'admin (interdit sur le dernier admin) |
| GET/PATCH | `/api/admin/instance` | politique d'inscription, limites (admin) |
| POST | `/api/admin/users` · `/api/admin/users/{id}/reset-password` | comptes gérés par l'admin (pas de SMTP v1) |
| POST | `/api/admin/invitations` | liens d'invitation |
| POST | `/api/admin/presets/reseed` | ré-import des profils système |

## Projets, modèles, scène

| Méthode | Route | Rôle |
|---|---|---|
| GET/POST | `/api/projects` | bibliothèque (vignette, dates, imprimante cible) |
| GET/PUT/DELETE | `/api/projects/{id}` | ouvrir / sauvegarder (manuel, FR-052 ; verrou optimiste : champ `version`, 409 en cas de conflit multi-onglets — edge case spec) / supprimer |
| POST | `/api/projects/{id}/duplicate` · `PATCH …/rename` | FR-052 |
| POST | `/api/projects/{id}/models` | upload multipart (STL/3MF/STEP/OBJ, 500 Mo) ; STEP → conversion asynchrone (R7) ; 3MF projet → import scène+réglages |
| GET | `/api/models/{id}/mesh` | maillage affichable (binaire) |
| POST | `/api/models/{id}/repair` | réparation + rapport (FR-012) |
| POST | `/api/projects/{id}/arrange` · `…/orient` | opérations moteur (plateau ou sélection) |
| GET | `/api/projects/{id}/export/3mf` | export projet 3MF compatible Orca (FR-044) |

La scène (transformations, plateaux, surcharges par objet, peintures) vit dans
le document projet sauvegardé par `PUT /api/projects/{id}` — les manipulations
interactives sont côté client, seules les opérations moteur font un aller-retour.

## Paramètres & presets

| Méthode | Route | Rôle |
|---|---|---|
| GET | `/api/params/registry` | registre généré (clés, types, bornes, enums, modes, tooltips) — servi statiquement, versionné |
| GET | `/api/presets?kind=&printer=` | système + utilisateur, filtre compatibilité (FR-021) |
| GET | `/api/presets/{id}` · `…/resolved` | valeurs brutes / chaîne d'héritage résolue |
| POST/PUT/DELETE | `/api/presets` | presets utilisateur (créer/dupliquer/renommer/supprimer, FR-022) |
| POST | `/api/presets/import` · `GET /api/presets/{id}/export` | JSON Orca, clés legacy converties (FR-023) |

## Tranchage, prévisualisation, G-code

| Méthode | Route | Rôle |
|---|---|---|
| POST | `/api/projects/{id}/slice` | body: plate_index \| all ; crée job(s) `queued` |
| GET | `/api/jobs` · `/api/jobs/{id}` | file + historique de l'utilisateur (FR-031) |
| POST | `/api/jobs/{id}/cancel` | annulation |
| GET | `/api/gcodes/{id}/download` | export G-code (FR-044) |
| GET | `/api/gcodes/{id}/stats` | statistiques (FR-043) |
| GET | `/api/gcodes/{id}/preview/meta` | couches, plages, types présents, échelles |
| GET | `/api/gcodes/{id}/preview/layers?from=&to=` | buffers binaires segments (R6) |

## Imprimantes (Moonraker)

| Méthode | Route | Rôle |
|---|---|---|
| GET/POST/PUT/DELETE | `/api/printers` | déclaration (URL, clé API chiffrée) |
| POST | `/api/printers/{id}/test` | `GET /server/info` relayé (FR-060) |
| POST | `/api/printers/{id}/upload` | body: gcode_id, start_now — upload Moonraker (FR-061) |
| GET | `/api/printers/{id}/status` | état instantané |
| POST | `/api/printers/{id}/pause` · `…/resume` · `…/cancel` | contrôle impression |

## WebSocket `/api/ws`

Canal unique authentifié, messages JSON typés (ts-rs) :

| Événement (serveur → client) | Contenu |
|---|---|
| `job.updated` | { id, status, progress, phase, error? } — file en direct |
| `job.finished` | { id, gcode_id, stats } — notification US7-AS1 |
| `model.converted` | { model_id, mesh_url } — fin de conversion STEP |
| `printer.status` | { printer_id, state, progress, temps } — suivi impression |

## Traçabilité UI

Les actions des menus/barres d'outils/raccourcis (Annexe B §B.2–B.6) se
résolvent soit en interactions client (scène), soit en endpoints ci-dessus ;
la correspondance exhaustive élément → composant/endpoint est maintenue dans
`traceability-map.json` et vérifiée par `audit/check_traceability.py` (R4).
