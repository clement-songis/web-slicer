# Data Model — Web-Slicer parité OrcaSlicer (Phase 1)

Toutes les entités appartiennent à un utilisateur sauf mention contraire.
Identifiants : UUID v4. Horodatage : `created_at`/`updated_at` UTC partout
(non répétés ci-dessous). Accès exclusivement via le trait `Storage`
(contrat : [contracts/storage-trait.md](contracts/storage-trait.md)).

## users

| Champ | Type | Règles |
|---|---|---|
| id | uuid PK | |
| email | text unique | normalisé lowercase ; identité de connexion |
| password_hash | text | argon2id ; jamais exposé par l'API |
| role | enum `admin` \| `user` | premier compte créé = admin |
| status | enum `active` \| `disabled` | |

Instance (table `instance_settings`, singleton) : `registration_policy`
(`open` (défaut) \| `closed` \| `invite`), limites d'upload. Invitations
(table `invitations`) : token, émetteur, expiration, usage unique.

## projects

| Champ | Type | Règles |
|---|---|---|
| id | uuid PK | |
| user_id | uuid FK users | isolation : toute requête filtrée par user_id |
| name | text | unique par utilisateur |
| thumbnail_path | text nullable | vignette générée à la sauvegarde |
| version | int | verrou optimiste : incrémenté à chaque PUT, 409 si décalé (conflit multi-onglets) |
| scene | jsonb/text | document scène (voir ci-dessous) |
| active_presets | json | {printer, filaments[], process} → preset ids |

**Document scène** (versionné, `schema_version`) : liste de `plates`
(réglages par plateau : type de plaque, position) ; liste d'objets →
`model_id`, transformations (matrice), instances, extrudeur/filament par
objet et par pièce, surcharges de paramètres par objet (clés du registre,
FR-015), peintures (supports/seam/fuzzy/MMU, stockées au format 3MF Orca),
profils de hauteur de couche variable par objet (LayerHeightProfile).
Sauvegarde manuelle (FR-052) ; le brouillon de session est côté client
(IndexedDB), hors modèle serveur.

## models (fichiers 3D importés)

| Champ | Type | Règles |
|---|---|---|
| id | uuid PK | |
| user_id | uuid FK | |
| project_id | uuid FK nullable | un modèle peut être partagé entre projets de l'utilisateur |
| filename | text | nom d'origine |
| format | enum `stl` \| `3mf` \| `step` \| `obj` | FR-010 |
| file_path | text | `data/users/<uid>/models/<id>.<ext>` (source conservée) |
| mesh_path | text nullable | maillage converti (STEP → mesh, décision R7) |
| size_bytes, triangle_count | int | limites d'upload (500 Mo défaut) |
| repair_report | json nullable | erreurs détectées/corrigées (FR-012) |

## presets

| Champ | Type | Règles |
|---|---|---|
| id | uuid PK | |
| kind | enum `machine_model` \| `machine` \| `filament` \| `process` | Annexe C |
| name | text | unique par (kind, origin, user_id) |
| origin | enum `system` \| `user` | system : lecture seule, re-seedable |
| user_id | uuid FK nullable | NULL pour les presets système |
| vendor | text nullable | ex. `BBL` (presets système) |
| inherits | text nullable | nom du parent (chaîne Orca conservée telle quelle) |
| instantiation | bool | presets abstraits masqués (FR-020) |
| setting_id / filament_id | text nullable | identifiants Orca |
| compatible_printers | json nullable | filtre de compatibilité (FR-021) |
| values | json | **uniquement les clés surchargées** (héritage non aplati, R5) |

Règles : la résolution (chaîne `inherits` → valeurs effectives) est calculée
par `engine::presets` et jamais stockée ; suppression d'un preset système
interdite ; suppression d'un preset utilisateur référencé par un projet →
le projet garde une copie figée des valeurs (avertissement à l'ouverture).
Import de profils : clés legacy converties via `params::legacy` (FR-023).

## printers (imprimantes déclarées, FR-060)

| Champ | Type | Règles |
|---|---|---|
| id | uuid PK ; user_id FK | |
| name | text | |
| moonraker_url | text | validée à la création (test `GET /server/info`) |
| api_key | text nullable | chiffrée au repos (clé d'instance) |
| machine_preset_id | uuid FK presets | rattachement au profil machine |

## slicing_jobs (file persistante, R9)

| Champ | Type | Règles |
|---|---|---|
| id | uuid PK ; user_id FK | isolation stricte (SC-008) |
| project_id | uuid FK ; plate_index | int | unité = un plateau (FR-014) |
| status | enum `queued` \| `running` \| `succeeded` \| `failed` \| `cancelled` | |
| progress | real 0–1 ; phase | text | relayés par WS |
| resolved_settings | json | presets résolus figés au lancement (reproductibilité) |
| error | json nullable | message moteur restitué (FR-032) |
| gcode_id | uuid FK nullable | résultat |

**Transitions** : `queued → running → succeeded|failed` ; `queued|running →
cancelled` (kill process) ; au boot : `running → queued` (reprise auto,
clarification). Historique conservé ; suppression en cascade avec le projet.

## gcodes

| Champ | Type | Règles |
|---|---|---|
| id | uuid PK ; user_id FK ; job_id FK | |
| file_path | text | `data/users/<uid>/gcodes/<id>.gcode` |
| preview_path | text | buffers binaires par couches (R6) |
| stats | json | temps total/par type, filament par extrudeur (longueur, volume, masse, coût), changements d'outil (FR-043) |
| thumbnails | json | vignettes embarquées |

## Suppression & rétention

- Suppression de compte → cascade BDD + purge `data/users/<uid>/` (FR-053).
- Pas de quota v1 (clarification) ; tailles suivies pour l'admin.

## Relations (vue d'ensemble)

```text
users 1─n projects 1─n models
users 1─n presets (origin=user) ; presets(system) sans user
projects n─1 presets actifs (par kind)
projects 1─n slicing_jobs 1─1 gcodes
users 1─n printers ─1 presets(machine)
```
