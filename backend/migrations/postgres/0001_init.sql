-- Schéma Postgres (T026), miroir du schéma SQLite (data-model.md) mais en types
-- natifs : `uuid`, `jsonb`, `timestamptz`, `boolean`, `bigint`. L'isolation
-- repose sur `user_id` + FK ON DELETE CASCADE (appliquées nativement par PG).
-- État final directement (unicité preset (kind, name, vendor), cf. 0003 SQLite),
-- + table `sessions` du store maison (cf. 0002 SQLite).

CREATE TABLE users (
    id            uuid PRIMARY KEY,
    email         text NOT NULL UNIQUE,
    password_hash text NOT NULL,
    role          text NOT NULL,
    status        text NOT NULL,
    created_at    timestamptz NOT NULL,
    updated_at    timestamptz NOT NULL
);

CREATE TABLE instance_settings (
    id                  integer PRIMARY KEY CHECK (id = 1),
    registration_policy text NOT NULL,
    upload_limit_bytes  bigint NOT NULL
);

CREATE TABLE invitations (
    id         uuid PRIMARY KEY,
    token      text NOT NULL UNIQUE,
    issued_by  uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at timestamptz NOT NULL,
    used       boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL
);

CREATE TABLE projects (
    id             uuid PRIMARY KEY,
    user_id        uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name           text NOT NULL,
    thumbnail_path text,
    version        bigint NOT NULL DEFAULT 1,
    scene          jsonb NOT NULL,
    active_presets jsonb NOT NULL,
    created_at     timestamptz NOT NULL,
    updated_at     timestamptz NOT NULL,
    UNIQUE (user_id, name)
);

CREATE TABLE models (
    id             uuid PRIMARY KEY,
    user_id        uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    project_id     uuid REFERENCES projects(id) ON DELETE SET NULL,
    filename       text NOT NULL,
    format         text NOT NULL,
    file_path      text NOT NULL,
    mesh_path      text,
    size_bytes     bigint NOT NULL,
    triangle_count bigint NOT NULL,
    repair_report  jsonb
);

CREATE TABLE presets (
    id                  uuid PRIMARY KEY,
    kind                text NOT NULL,
    name                text NOT NULL,
    origin              text NOT NULL,
    user_id             uuid REFERENCES users(id) ON DELETE CASCADE,
    vendor              text,
    inherits            text,
    instantiation       boolean NOT NULL,
    setting_id          text,
    filament_id         text,
    compatible_printers jsonb,
    values_json         jsonb NOT NULL
);
-- Unicité (kind, name) par utilisateur ; système = (kind, name, vendor).
CREATE UNIQUE INDEX presets_unique_user
    ON presets (kind, name, user_id) WHERE user_id IS NOT NULL;
CREATE UNIQUE INDEX presets_unique_system
    ON presets (kind, name, vendor) WHERE user_id IS NULL;

CREATE TABLE printers (
    id                uuid PRIMARY KEY,
    user_id           uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name              text NOT NULL,
    moonraker_url     text NOT NULL,
    api_key           text,
    machine_preset_id uuid NOT NULL REFERENCES presets(id) ON DELETE CASCADE
);

CREATE TABLE slicing_jobs (
    id                uuid PRIMARY KEY,
    user_id           uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    project_id        uuid NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    plate_index       bigint NOT NULL,
    status            text NOT NULL,
    progress          double precision NOT NULL DEFAULT 0,
    phase             text NOT NULL DEFAULT '',
    resolved_settings jsonb NOT NULL,
    error             jsonb,
    gcode_id          uuid,
    created_at        timestamptz NOT NULL,
    updated_at        timestamptz NOT NULL
);
CREATE INDEX slicing_jobs_queue ON slicing_jobs (status, created_at);

CREATE TABLE gcodes (
    id           uuid PRIMARY KEY,
    user_id      uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    job_id       uuid NOT NULL REFERENCES slicing_jobs(id) ON DELETE CASCADE,
    file_path    text NOT NULL,
    preview_path text NOT NULL,
    stats        jsonb NOT NULL,
    thumbnails   jsonb NOT NULL
);

-- Sessions `tower-sessions` (store maison, cf. adaptateur sessions).
CREATE TABLE sessions (
    id          text PRIMARY KEY NOT NULL,
    record      text NOT NULL,
    expiry_date timestamptz NOT NULL
);
CREATE INDEX idx_sessions_expiry ON sessions (expiry_date);
