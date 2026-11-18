-- Préférences de l'interface par compte (cf. SQLite 0009).
CREATE TABLE user_preferences (
    user_id     uuid PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    prefs_json  jsonb NOT NULL DEFAULT '{}'::jsonb,
    updated_at  timestamptz NOT NULL
);
