# API HTTP du backend web-slicer

Référence générée à partir du routeur (`backend/src/http/routes/mod.rs`) au lot 27, tenue à jour
jusqu'au lot 32 (sections « Nouveautés » ci-dessous, la plus récente en premier).
Contrat détaillé historique : `specs/001-orcaslicer-web-parity/contracts/http-api.md`.
Types TypeScript des corps : `frontend/src/generated/api/*.ts` (ts-rs, ne pas éditer).

## Conventions

- **Authentification** : cookie de session (`tower-sessions`). Toute route hors
  `/api/health`, `/api/auth/register`, `/api/auth/login` exige une session → sinon **401**.
  Routes `/api/admin/*` : compte admin → sinon **403**.
- **Isolation (SC-008)** : une ressource d'un autre compte répond **404** (jamais 403).
  Un identifiant mal formé dans le chemin répond aussi 404.
- **Corps** : JSON (`Content-Type: application/json`), sauf uploads (multipart) et
  binaires (WSMh, G-code, 3MF, PNG).
- **Statuts de succès** : **201** création d'une ressource (corps = la ressource),
  **204** action sans corps (suppression, commande), **200** sinon (lecture, calcul,
  validation, mise à jour renvoyant la ressource). **101** pour les WebSocket.

## Format d'erreur (toutes les routes)

Toute réponse 4xx/5xx a le corps `application/json` :

```json
{ "code": "validation", "message": "plate_index hors limites", "details": { "plate_index": 4, "plate_count": 1 } }
```

- `code` : stable, machine-lisible (tableau ci-dessous) ; `message` : texte clair (FR,
  ou msgid anglais d'Orca quand il vient du moteur) ; `details` : facultatif, contexte
  structuré (clé en faute, bornes, cause…). Type TS : `ErrorBody`.
- Les 5xx ne divulguent jamais de détail (journalisé côté serveur).
- Depuis le lot 27, les rejets générés par axum (JSON illisible, champ de mauvais type,
  Content-Type absent, corps trop gros, route inconnue, méthode non permise) sont réécrits
  dans cette enveloppe par la couche `http::envelope::normalize_error_response`, avec la
  raison technique d'axum dans `details.reason`.
- Routes harness (`/api/_harness/*`, HARNESS_API=1) : `{ error, code, message }`
  (`error` conservé pour le contrat harness).

| Statut | `code` | Sens |
|---|---|---|
| 400 | `bad_request` | corps JSON syntaxiquement illisible (rejet axum) |
| 401 | `unauthorized` | session absente, expirée ou révoquée (mot de passe changé) |
| 401 | `invalid_credentials` | `POST /api/auth/login` : e-mail inconnu ou mot de passe faux (indistinguables) — lot 32 |
| 403 | `forbidden` | politique d'instance (inscription fermée, admin requis, dernier admin) |
| 403 | `invalid_credentials` | `PUT /api/me/password` : mot de passe actuel faux (`details.field = "current_password"`, la session reste valide) — lot 32 |
| 404 | `not_found` | ressource absente **ou d'un autre compte**, route inconnue |
| 405 | `method_not_allowed` | méthode non permise sur la route |
| 409 | `conflict` | verrou optimiste (`expected_version`), conversion en cours, doublon |
| 409 | `name_taken` | renommage vers le nom d'un autre projet du compte — `details: { field, name, suggestion }` (lot 32) |
| 409 | `printer_not_connected` | imprimante déclarée sans URL réseau |
| 413 | `too_large` | upload / corps au-delà de la limite |
| 413 | `storage_quota_exceeded` | quota de stockage du compte atteint — `details: { used_bytes, quota_bytes, incoming_bytes }` (lot 32) |
| 415 | `unsupported_media_type` | Content-Type JSON attendu |
| 422 | `validation` | entrée invalide (type, bornes, enum, brouillon) — `details.key` si une option est en cause |
| 422 | `conversion_failed` | conversion moteur du modèle échouée — `details: { model_id, cause }` |
| 422 | `printer_not_owned` | preset imprimante d'un modèle non possédé |
| 429 | `slice_quota_exceeded` | trop de tranchages simultanés pour le compte — `details: { active, max, requested, retry_after }`, en-tête **`Retry-After: 30`** (secondes, lot 35) |
| 501 | `not_implemented` | fonctionnalité indisponible |
| 502 | `printer_unreachable` | hôte d'impression (Moonraker, OctoPrint…) injoignable ou en erreur |
| 500 | `internal` | erreur interne |

## Nouveautés lot 35 — statut admin, réservation G-code, écarts lot 34

### `PUT /api/admin/users/{id}/status` — désactiver / réactiver un compte (admin)

Corps `SetUserStatusRequest = { status: "active" | "disabled" }`. **204** sans corps.

- Valeur autre → 422 `validation` (`details.field = "status"`) ; compte inconnu → 404 ; non admin → 403.
- Désactivation : les **sessions ouvertes du compte sont invalidées à leur requête suivante**
  (401 `unauthorized`), et le login est refusé (403 `forbidden`, « compte désactivé »).
- Réactivation : le login redevient possible (les sessions précédentes restent révoquées).
- Garde : impossible de désactiver le **dernier administrateur actif** → 403 `forbidden`
  (même garde que `DELETE /api/admin/users/{id}`). L'opération est idempotente.
- Le statut est visible dans `GET /api/admin/users` (`status` de `AdminUserResponse`).

### Réservation de quota G-code avant `/slice` (lot 35)

Avant de créer les jobs, `POST /api/projects/{id}/slice` réserve une **estimation** de la taille du
G-code de chaque plateau ciblé : `128 Kio + 512 octets × triangles des modèles du plateau`. Si le
quota de stockage du compte ne couvre pas cette réservation → 413 `storage_quota_exceeded`
(`details.incoming_bytes` = estimation totale). Heuristique documentée : majorante pour éviter un
dépassement a posteriori, sans bloquer des scènes raisonnables.

### Écarts lot 34 comblés (parité Orca 2.4.1)

- **Arrangement — tour d'essuyage réservée** : `POST /api/projects/{id}/arrange` (portée `all`)
  pose une tour estimée (`wipe_tower_x/y`, `prime_tower_width`, `prime_volume`, brim, marge 1 mm)
  comme obstacle fixe sur chaque plateau potentiel quand `enable_prime_tower` est actif,
  l'impression n'est pas séquentielle et un multi-matériau l'exige (objet multi-extrudeurs,
  timelapse fluide, ou plusieurs matériaux de même température de plateau autorisés ensemble).
  Sans changement d'API : les placements évitent simplement l'emprise de la tour. En portée
  `plate`, pas de tour estimée (comme `prepare_partplate` d'Orca).
- **Import « géométrie seule »** (`POST /api/projects/import?load=geometry`) : les objets sont
  **réaffectés à leur plateau par position** (grille Orca, `printable_area` du 3MF) et les
  **filaments manquants sont ajoutés** — `scene.projectConfig.filament_colour` couvre les
  extrudeurs utilisés (palette système d'Orca, plafond 16).
- **`import.gcode_warning`** : les presets renommés/alias (`renamed_from`, et l'alias dérivé du
  « @ » du nom) sont reconnus comme de confiance — plus de fausse boîte « Customized Presets »
  sur un 3MF référençant un ancien nom.

Types TS nouveaux : `SetUserStatusRequest`.

## Nouveautés lot 32 — comptes, quotas, noms uniques

Types TS : `AdminUserListResponse`, `AdminUserResponse`, `AdminUsersQuery`, `ChangePasswordRequest`,
`UsageResponse`, `InstanceResponse` / `PatchInstanceRequest` (champs ajoutés). Entiers en octets.

### `GET /api/admin/users?page=&per_page=&q=` — liste paginée des comptes (admin)

Réponse **200** `AdminUserListResponse` :

```ts
AdminUserListResponse = { users: AdminUserResponse[], total: number, page: number, per_page: number }
AdminUserResponse = { id, email, role: "admin" | "user", status: "active" | "disabled",
                      created_at: string /* RFC 3339 UTC */, storage_bytes: number, project_count: number }
```

- Ordre : date de création croissante. `page` ≥ 1 (défaut 1), `per_page` 1..=200 (défaut 50) ; hors
  bornes → 422 `validation` (`details.field`) ; valeur non entière → 400 (rejet axum dans l'enveloppe).
  Page au-delà de la fin → `users: []` (`total` inchangé).
- `q` : sous-chaîne de l'e-mail, insensible à la casse ; `total` = nombre de comptes filtrés.
- `storage_bytes` : tout l'espace fichiers du compte (`data/users/<uid>` : modèles, maillages, G-code,
  vignettes, auxiliaires, G-code ouverts), calculé à la demande pour la page seulement.
- Compte non admin → 403, sans session → 401.

### `PUT /api/me/password` — changer son mot de passe

Corps `ChangePasswordRequest = { current_password, new_password }`. **204** sans corps.

- Mot de passe actuel faux → **403** `invalid_credentials` (`details.field = "current_password"`) ;
  pas 401, la session reste ouverte.
- Nouveau < 8 caractères → 422 `validation` (`details.field = "password"`) ; identique à l'actuel →
  422 (`details.field = "new_password"`).
- Succès : **toutes les autres sessions du compte sont révoquées** (leur requête suivante → 401
  `unauthorized`) ; la session courante reste ouverte avec un **nouveau cookie** (`Set-Cookie`,
  identifiant renouvelé). Mécanisme : empreinte du hash du mot de passe posée dans la session à la
  connexion et vérifiée à chaque requête (indépendant du store de sessions). La réinitialisation par
  l'admin (`POST /api/admin/users/{id}/reset-password`) révoque de même toutes les sessions du compte visé.
- La connexion (`POST /api/auth/login`) renouvelle désormais l'identifiant de session (fixation).

### `POST /api/auth/login` — code `invalid_credentials`

Identifiants refusés → **401** `{ code: "invalid_credentials", message: "E-mail ou mot de passe incorrect" }`
(e-mail inconnu et mot de passe faux : réponse identique). Compte désactivé → 403 `forbidden`.
Une session absente ou révoquée reste `unauthorized`.

### Quotas par compte (réglages d'instance)

`GET|PATCH /api/admin/instance` : `InstanceResponse` / `PatchInstanceRequest` gagnent

| Champ | Sens | Défaut | Bornes (PATCH) |
|---|---|---|---|
| `storage_quota_bytes` | espace fichiers maximal **par compte** | 0 = illimité | ≥ 0 |
| `max_concurrent_slices` | jobs `queued` + `running` maximum **par compte** | 0 = illimité | 0..=1000 |

(`upload_limit_bytes`, par fichier, est inchangé ; champ absent du PATCH = conservé.)

Application :

- **413 `storage_quota_exceeded`** si `espace occupé + taille entrante > quota` : upload de modèle
  (`POST /api/projects/{id}/models`), import (`POST /api/projects/import`, `…/import-3mf`), fichier
  auxiliaire (`PUT …/auxiliaries/…`), G-code ouvert (`POST /api/gcode/process`, sauf fichier déjà gardé),
  duplication (`POST /api/projects/{id}/duplicate`, taille des modèles copiés), et tranchage
  (`POST /api/projects/{id}/slice`) dès que le quota est atteint. Message :
  « Quota de stockage atteint : X Mo utilisés sur Y Mo ».
- **429 `slice_quota_exceeded`** si `jobs actifs du compte + plateaux demandés > max` sur
  `POST /api/projects/{id}/slice` (« Trancher tout » compte tous ses plateaux). Les jobs d'autres comptes
  ne comptent jamais. Pas d'en-tête `Retry-After` : la place se libère à la fin ou à l'annulation d'un job.

### `GET /api/me/usage` — usage et quotas du compte connecté

`UsageResponse = { storage_bytes, storage_quota_bytes, active_slices, max_concurrent_slices }` (0 = illimité).

### Noms de projet uniques côté serveur

- `POST /api/projects`, `POST /api/projects/import` (nom tiré du fichier) et `POST …/duplicate`
  (« X (copie) ») ne renvoient plus de conflit : nom libre utilisé tel quel, sinon premier
  « nom (n) » libre, n ≥ 2 (« Sans titre (2) », « cube (copie) (2) » ; un nom déjà suffixé
  « X (2) » pris donne « X (3) », pas « X (2) (2) »). Le nom retenu est dans la réponse (`name`).
- `PATCH /api/projects/{id}/rename` et `PATCH /api/projects/{id}` (`name`) vers le nom d'un **autre**
  projet du compte → **409 `name_taken`**, message « Un autre projet s'appelle déjà « X » »,
  `details: { field: "name", name, suggestion }` (`suggestion` = premier nom libre). Renommer vers son
  propre nom : 200.
- Unicité par compte : un autre compte peut porter le même nom (aucun oracle).
- Toutes les violations d'unicité du stockage (presets, e-mails…) sortent sans message SQL
  (« un élément portant ce nom existe déjà »).

### Hors parité : bibliothèque partagée

OrcaSlicer est une application de bureau mono-utilisateur : ni comptes, ni projets/presets partagés
entre utilisateurs (la synchronisation cloud Bambu ne fait que répliquer les presets d'un même compte).
Une bibliothèque partagée (ACL lecture/écriture entre comptes) n'a donc pas de référence Orca : **hors
périmètre de parité, non implémentée**. Les projets, modèles, presets utilisateur, imprimantes et
calibrations restent strictement privés à chaque compte (SC-008, campagne d'isolation).

## Nouveautés lot 31 — données d'aperçu d'Orca côté serveur

- `active_presets?` (`{ printer?, process?, filaments?: string[] }`, ids de presets) sur
  `POST /api/projects/{id}/slice`, `…/validate` et `…/filament-compatibility` : presets affichés non
  enregistrés ; absent ⇒ presets du projet. Mêmes gardes (visibilité, imprimante possédée) ; format
  invalide → 422 `details.field = "active_presets"`.
- `GET /api/gcodes/{id}/preview/processed/meta` : vue Couleur (palette, M600, pauses / G-code perso),
  temps écoulé par couche, tableau « Filament » par filament (modèle / support / purge / tour).
- `GET /api/gcodes/{id}/preview/processed/seams?from=&to=` → `{ seams: [{ layer, move, x, y, z }] }`
  (coutures émises par GCodeProcessor).
- `POST /api/gcodes/plates-stats` corps `{ gcode_ids: string[] }` (1..=64, ordre des plateaux) →
  `{ plates: [{ gcode_id, time: [normal, silencieux], total_cost, filament_table }], all }`
  (render_all_plates_stats d'Orca ; un G-code d'un autre compte → 404).
- G-code externe simulé (« Open G-code… ») : `POST /api/gcode/process` (corps = G-code brut) →
  `{ token, meta, stats, footer }` ; le fichier est gardé dans l'espace du compte (4 derniers, jeton =
  MD5 du contenu). `GET /api/gcode/opened/{token}/meta|moves|seams` : mêmes réponses que
  `/api/gcodes/{id}/preview/processed/*` (`moves` = binaire `WSPm`, `?from=&to=`). Jeton inconnu,
  invalide ou d'un autre compte → 404. G-code vide ou sans couche → 422.

## Nouveautés lot 27

### `POST /api/projects/{id}/validate` — validation avant tranchage (Print::validate d'Orca)

Corps : identique à `POST …/slice` (`SliceRequest` : `plate_index?`, `draft?`,
`allow_mixed_temperatures?`). Ne crée aucun job, ne tranche pas.
Réponse **200** `ValidateResponse` :

```ts
ValidateResponse = { valid: boolean, plates: PlateValidation[], warnings: SliceWarning[] }
PlateValidation  = { plate_index: number, valid: boolean, errors: ValidationIssue[], warnings: ValidationIssue[] }
ValidationIssue  = { kind, message: string, opt_key?: string, object_id?: string, object_name?: string, params?: string[] }
kind = "general" | "filament_not_match_bed_type" | "filaments_different_temp"
     | "object_collision_in_seq_print" | "object_collision_in_layer_print" | "layer_height_exceeds_limit"
     | "config" | "consistency" | "empty_plate" | "object_outside_plate"
```

- Plateaux : `plate_index` ou, à défaut, tous les plateaux non vides (projet vide →
  plateau 0 rapporté `empty_plate`). `plate_index` hors limites → 422.
- `object_id` = `scene.objects[].id` de l'objet désigné par Orca (collisions « par objet »,
  objet trop haut, dégagement extrudeur, réglages d'objet) ; `opt_key` = option à mettre
  en évidence dans l'éditeur.
- `warnings` (racine) = avertissements de config identiques à `/slice`.
- Refus de `/slice` (plateau vide, hors plateau) rapportés ici en erreurs 200.

### `apply_draft` : `null` = `nil` sur les options nullables

Dans `draft.process|printer|filaments[i]`, `null` (ou `[null]`, ou `"nil"`) sur une option
nullable d'Orca (ex. `filament_retraction_length`, `filament_z_hop`…) vaut `nil` (hériter de
l'imprimante). Sur une option non nullable : 422 `validation`, `details.key`.

### Cause des échecs de conversion

- `ModelResponse.conversion_error?: string` (cause du moteur) ; `conversion_pending` passe à
  `false` dès l'échec.
- 422 `conversion_failed` (`/mesh`, opérations de maillage) : message
  « échec de la conversion du modèle par le moteur : <cause> », `details: { model_id, cause }`.
- Événement WS `{ event: "model.conversion_failed", model_id, error }`.

## Routes

| Méthode | Chemin | Handler | Succès |
|---|---|---|---|
| GET | `/api/health` | `health` | 200 |
| POST | `/api/auth/register` | `auth::register` | 201 |
| POST | `/api/auth/login` | `auth::login` | 200 |
| POST | `/api/auth/logout` | `auth::logout` | 204 |
| GET | `/api/auth/me` | `auth::me` | 200 |
| DELETE | `/api/auth/me` | `auth::delete_me` | 204 |
| PUT | `/api/me/password` | `auth::change_password` | 204 |
| GET | `/api/me/usage` | `auth::usage` | 200 |
| GET | `/api/me/preferences` | `preferences::get` | 200 |
| PUT | `/api/me/preferences` | `preferences::put` | 200 |
| GET | `/api/admin/instance` | `admin::get_instance` | 200 |
| PATCH | `/api/admin/instance` | `admin::patch_instance` | 200 |
| GET | `/api/admin/users` | `admin::list_users` | 200 |
| POST | `/api/admin/users` | `admin::create_user` | 201 |
| DELETE | `/api/admin/users/{id}` | `admin::delete_user` | 204 |
| POST | `/api/admin/users/{id}/reset-password` | `admin::reset_password` | 204 |
| PUT | `/api/admin/users/{id}/status` | `admin::set_user_status` | 204 |
| POST | `/api/admin/invitations` | `admin::create_invitation` | 201 |
| POST | `/api/admin/presets/reseed` | `admin::reseed_presets` | 200 |
| GET | `/api/projects` | `projects::list` | 200 |
| POST | `/api/projects` | `projects::create` | 201 |
| POST | `/api/projects/import` | `projects::import` | 201 |
| POST | `/api/projects/{id}/import-3mf` | `projects::import_into` | 201 |
| GET | `/api/projects/{id}` | `projects::get` | 200 |
| PUT | `/api/projects/{id}` | `projects::save` | 200 |
| PATCH | `/api/projects/{id}` | `projects::patch` | 200 |
| DELETE | `/api/projects/{id}` | `projects::delete` | 204 |
| POST | `/api/projects/{id}/duplicate` | `projects::duplicate` | 201 |
| PATCH | `/api/projects/{id}/rename` | `projects::rename` | 200 |
| GET | `/api/projects/{id}/thumbnail` | `projects::thumbnail` | 200 |
| GET | `/api/projects/{id}/models` | `models::list_for_project` | 200 |
| POST | `/api/projects/{id}/models` | `models::upload` | 201 |
| GET | `/api/models/{id}/mesh` | `models::mesh` | 200 |
| GET | `/api/models/{id}/file` | `models::download_file` | 200 |
| POST | `/api/models/{id}/obj-colors` | `models::obj_colors` | 200 |
| POST | `/api/models/{id}/step-mesh` | `models::step_mesh` | 200 |
| POST | `/api/models/{id}/repair` | `scene::repair` | 200 |
| POST | `/api/models/{id}/simplify` | `scene::simplify` | 200 |
| POST | `/api/models/{id}/remap-paint` | `scene::remap_paint` | 200 |
| POST | `/api/models/{id}/split` | `mesh_ops::split` | 201 |
| POST | `/api/models/{id}/smooth` | `mesh_ops::smooth` | 201 |
| POST | `/api/models/{id}/cut` | `mesh_ops::cut` | 201 |
| POST | `/api/projects/{id}/boolean` | `mesh_ops::boolean` | 201 |
| POST | `/api/projects/{id}/emboss` | `mesh_ops::emboss` | 201 |
| POST | `/api/projects/{id}/emboss-svg` | `mesh_ops::emboss_svg` | 201 |
| POST | `/api/projects/{id}/primitive` | `mesh_ops::primitive` | 201 |
| POST | `/api/projects/{id}/filament-compatibility` | `slice::filament_compatibility_check` | 200 |
| GET | `/api/handy-models` | `handy_models::list` | 200 |
| POST | `/api/projects/{id}/handy-models/{key}` | `handy_models::add` | 201 |
| GET | `/api/projects/{id}/paint` | `paint::paint_ws` | 101 (WebSocket) |
| POST | `/api/models/{id}/layer-profile` | `mesh_ops::layer_profile` | 200 |
| GET | `/api/emboss/fonts` | `mesh_ops::emboss_fonts` | 200 |
| POST | `/api/projects/{id}/arrange` | `scene::arrange` | 200 |
| POST | `/api/projects/{id}/orient` | `scene::orient` | 200 |
| POST | `/api/projects/{id}/slice` | `slice::slice` | 201 |
| POST | `/api/projects/{id}/validate` | `validate::validate` | 200 |
| GET | `/api/projects/{id}/export/3mf` | `export::export_3mf` | 200 |
| PUT | `/api/projects/{id}/plates/{index}/thumbnail` | `export::put_plate_thumbnail` | 204 |
| DELETE | `/api/projects/{id}/plates/{index}/thumbnail` | `export::delete_plate_thumbnail` | 204 |
| POST | `/api/projects/{id}/export/drc` | `export::export_drc` | 200 |
| POST | `/api/projects/{id}/export/stl` | `export::export_stl` | 200 |
| POST | `/api/flush-volumes` | `flush::flush_volumes` | 200 |
| POST | `/api/projects/{id}/calibration` | `calibration::create_project_calibration` | 201 |
| GET | `/api/projects/{id}/auxiliaries` | `auxiliaries::list` | 200 |
| GET | `/api/projects/{id}/auxiliaries/{folder}/{name}` | `auxiliaries::get` | 200 |
| PUT | `/api/projects/{id}/auxiliaries/{folder}/{name}` | `auxiliaries::put` | 201 (création) / 204 (remplacement) |
| DELETE | `/api/projects/{id}/auxiliaries/{folder}/{name}` | `auxiliaries::delete` | 204 |
| GET | `/api/calibrations/pa` | `calibration::list_pa` | 200 |
| POST | `/api/calibrations/pa` | `calibration::create_pa` | 201 |
| GET | `/api/calibrations/pa/{id}` | `calibration::get_pa` | 200 |
| PUT | `/api/calibrations/pa/{id}` | `calibration::update_pa` | 200 |
| DELETE | `/api/calibrations/pa/{id}` | `calibration::delete_pa` | 204 |
| POST | `/api/projects/{id}/export/sliced` | `sliced::export_sliced` | 200 |
| GET | `/api/jobs` | `jobs::list` | 200 |
| GET | `/api/jobs/{id}` | `jobs::get` | 200 |
| POST | `/api/jobs/{id}/cancel` | `jobs::cancel` | 200 |
| GET | `/api/gcodes/{id}/download` | `gcodes::download` | 200 |
| GET | `/api/gcodes/{id}/stats` | `gcodes::stats` | 200 |
| GET | `/api/gcodes/{id}/preview/meta` | `preview::meta` | 200 |
| GET | `/api/gcodes/{id}/preview/layers` | `preview::layers` | 200 |
| GET | `/api/gcodes/{id}/preview/processed/meta` | `preview::processed_meta` | 200 |
| GET | `/api/gcodes/{id}/preview/processed/moves` | `preview::processed_moves` | 200 |
| GET | `/api/gcodes/{id}/preview/processed/seams` | `preview::processed_seams` | 200 |
| POST | `/api/gcodes/plates-stats` | `preview::plates_stats` | 200 |
| POST | `/api/gcode/preview` | `gcode_open::preview` | 200 |
| POST | `/api/gcode/process` | `gcode_open::process` | 200 |
| GET | `/api/gcode/opened/{token}/meta` | `gcode_open::opened_meta` | 200 |
| GET | `/api/gcode/opened/{token}/moves` | `gcode_open::opened_moves` | 200 |
| GET | `/api/gcode/opened/{token}/seams` | `gcode_open::opened_seams` | 200 |
| GET | `/api/ws` | `ws::ws` | 101 (WebSocket) |
| GET | `/api/presets` | `presets::list` | 200 |
| POST | `/api/presets` | `presets::create` | 201 |
| GET | `/api/presets/{id}` | `presets::get` | 200 |
| PUT | `/api/presets/{id}` | `presets::update` | 200 |
| DELETE | `/api/presets/{id}` | `presets::delete` | 204 |
| GET | `/api/presets/{id}/resolved` | `presets::resolved` | 200 |
| POST | `/api/presets/{id}/detach` | `presets::detach` | 201 |
| GET | `/api/presets/{id}/export` | `presets::export` | 200 |
| POST | `/api/presets/import` | `presets::import` | 201 |
| GET | `/api/presets/create-options` | `presets::create_options` | 200 |
| GET | `/api/printer-catalog` | `printers::catalog` | 200 |
| GET | `/api/printers` | `printers::list` | 200 |
| POST | `/api/printers` | `printers::create` | 201 |
| GET | `/api/printers/discover` | `printers::discover` | 200 |
| GET | `/api/printers/{id}` | `printers::get` | 200 |
| PUT | `/api/printers/{id}` | `printers::update` | 200 |
| DELETE | `/api/printers/{id}` | `printers::delete` | 204 |
| POST | `/api/printers/{id}/test` | `printers::test` | 200 |
| GET | `/api/printers/{id}/storages` | `printers::storages` | 200 |
| GET | `/api/printers/{id}/host-printers` | `printers::host_printers` | 200 |
| POST | `/api/printers/{id}/upload` | `printers::upload` | 200 |
| POST | `/api/printers/{id}/upload-sliced` | `sliced::upload_sliced` | 200 |
| GET | `/api/printers/{id}/status` | `printers::status` | 200 |
| POST | `/api/printers/{id}/pause` | `printers::pause` | 204 |
| POST | `/api/printers/{id}/resume` | `printers::resume` | 204 |
| POST | `/api/printers/{id}/cancel` | `printers::cancel` | 204 |
| GET | `/api/printers/{id}/files` | `printers::files` | 200 |
| POST | `/api/printers/{id}/print` | `printers::print` | 200 |
| POST | `/api/printers/{id}/gcode` | `printers::gcode` | 204 |
| GET | `/api/printers/{id}/objects` | `printers::objects` | 200 |
| POST | `/api/printers/{id}/objects/exclude` | `printers::exclude_object` | 200 |
| POST | `/api/_harness/bootstrap` | `harness::bootstrap` | 200 |
| POST | `/api/_harness/slice` | `harness::slice` | 200 (HARNESS_API=1 seulement) |
