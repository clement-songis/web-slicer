-- Sessions `tower-sessions` (store maison sur sqlx 0.9, cf.
-- `adapters::sessions`). `id` = identifiant encodé (base64 url-safe) ;
-- `record` = enregistrement complet sérialisé en JSON ; `expiry_date` isolé
-- pour purger les sessions expirées côté SQL.
CREATE TABLE sessions (
    id          TEXT PRIMARY KEY NOT NULL,
    record      TEXT NOT NULL,
    expiry_date TEXT NOT NULL
);

CREATE INDEX idx_sessions_expiry ON sessions (expiry_date);
