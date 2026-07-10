-- Schéma initial (data-model.md). UUID/JSON/horodatages en TEXT pour rester
-- simple et lisible ; l'isolation repose sur `user_id` + FK ON DELETE CASCADE
-- (la connexion active PRAGMA foreign_keys=ON, cf. adaptateur).

CREATE TABLE users (
    id            TEXT PRIMARY KEY,
    email         TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role          TEXT NOT NULL,
    status        TEXT NOT NULL,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);

CREATE TABLE instance_settings (
    id                  INTEGER PRIMARY KEY CHECK (id = 1),
    registration_policy TEXT NOT NULL,
    upload_limit_bytes  INTEGER NOT NULL
);

CREATE TABLE invitations (
    id         TEXT PRIMARY KEY,
    token      TEXT NOT NULL UNIQUE,
    issued_by  TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at TEXT NOT NULL,
    used       INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);

CREATE TABLE projects (
    id             TEXT PRIMARY KEY,
    user_id        TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name           TEXT NOT NULL,
    thumbnail_path TEXT,
    version        INTEGER NOT NULL DEFAULT 1,
    scene          TEXT NOT NULL,
    active_presets TEXT NOT NULL,
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL,
    UNIQUE (user_id, name)
);

CREATE TABLE models (
    id             TEXT PRIMARY KEY,
    user_id        TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    project_id     TEXT REFERENCES projects(id) ON DELETE SET NULL,
    filename       TEXT NOT NULL,
    format         TEXT NOT NULL,
    file_path      TEXT NOT NULL,
    mesh_path      TEXT,
    size_bytes     INTEGER NOT NULL,
    triangle_count INTEGER NOT NULL,
    repair_report  TEXT
);

CREATE TABLE presets (
    id                  TEXT PRIMARY KEY,
    kind                TEXT NOT NULL,
    name                TEXT NOT NULL,
    origin              TEXT NOT NULL,
    user_id             TEXT REFERENCES users(id) ON DELETE CASCADE,
    vendor              TEXT,
    inherits            TEXT,
    instantiation       INTEGER NOT NULL,
    setting_id          TEXT,
    filament_id         TEXT,
    compatible_printers TEXT,
    values_json         TEXT NOT NULL
);
-- Unicité (kind, name) séparée système / par utilisateur (user_id NULL = système).
CREATE UNIQUE INDEX presets_unique_user
    ON presets (kind, name, user_id) WHERE user_id IS NOT NULL;
CREATE UNIQUE INDEX presets_unique_system
    ON presets (kind, name) WHERE user_id IS NULL;

CREATE TABLE printers (
    id                TEXT PRIMARY KEY,
    user_id           TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name              TEXT NOT NULL,
    moonraker_url     TEXT NOT NULL,
    api_key           TEXT,
    machine_preset_id TEXT NOT NULL REFERENCES presets(id) ON DELETE CASCADE
);

CREATE TABLE slicing_jobs (
    id                TEXT PRIMARY KEY,
    user_id           TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    project_id        TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    plate_index       INTEGER NOT NULL,
    status            TEXT NOT NULL,
    progress          REAL NOT NULL DEFAULT 0,
    phase             TEXT NOT NULL DEFAULT '',
    resolved_settings TEXT NOT NULL,
    error             TEXT,
    gcode_id          TEXT,
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL
);
CREATE INDEX slicing_jobs_queue ON slicing_jobs (status, created_at);

CREATE TABLE gcodes (
    id           TEXT PRIMARY KEY,
    user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    job_id       TEXT NOT NULL REFERENCES slicing_jobs(id) ON DELETE CASCADE,
    file_path    TEXT NOT NULL,
    preview_path TEXT NOT NULL,
    stats        TEXT NOT NULL,
    thumbnails   TEXT NOT NULL
);
