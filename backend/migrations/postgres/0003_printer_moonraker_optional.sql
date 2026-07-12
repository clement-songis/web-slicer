-- Phase 14 : `moonraker_url` optionnel (imprimante possédée sans connexion
-- réseau ; connexion Moonraker ajoutée plus tard). Cf. sqlite/0005.
ALTER TABLE printers ALTER COLUMN moonraker_url DROP NOT NULL;
