-- Préférences de l'interface par compte (équivalent web de l'AppConfig d'Orca :
-- caméra, comportement, unités…). Document JSON libre, taille bornée par l'API.
CREATE TABLE user_preferences (
    user_id     TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    prefs_json  TEXT NOT NULL DEFAULT '{}',
    updated_at  TEXT NOT NULL
);
