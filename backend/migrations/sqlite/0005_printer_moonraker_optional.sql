-- Phase 14 : une imprimante « possédée » n'est plus forcément connectée en
-- réseau. `moonraker_url` devient optionnel (impression carte SD / USB, ou
-- connexion Moonraker ajoutée plus tard). SQLite ne sait pas assouplir une
-- contrainte NOT NULL en place → reconstruction de la table (aucune table ne
-- référence `printers`, la reconstruction est sûre).
CREATE TABLE printers_new (
    id                TEXT PRIMARY KEY,
    user_id           TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name              TEXT NOT NULL,
    moonraker_url     TEXT,
    api_key           TEXT,
    machine_preset_id TEXT NOT NULL REFERENCES presets(id) ON DELETE CASCADE
);

INSERT INTO printers_new (id, user_id, name, moonraker_url, api_key, machine_preset_id)
SELECT id, user_id, name, moonraker_url, api_key, machine_preset_id FROM printers;

DROP TABLE printers;
ALTER TABLE printers_new RENAME TO printers;
