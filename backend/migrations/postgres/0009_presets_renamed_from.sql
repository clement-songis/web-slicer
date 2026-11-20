-- Lot 35 : anciens noms des presets système (`renamed_from` Orca) pour que
-- validate_presets reconnaisse un preset renommé/alias comme de confiance.
ALTER TABLE presets ADD COLUMN renamed_from JSONB NOT NULL DEFAULT '[]'::jsonb;
