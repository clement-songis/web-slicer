-- Quotas d'instance (lot 32) : stockage par compte (octets) et nombre de
-- tranchages simultanés (en file + en cours) par compte. 0 = illimité.
ALTER TABLE instance_settings ADD COLUMN storage_quota_bytes INTEGER NOT NULL DEFAULT 0;
ALTER TABLE instance_settings ADD COLUMN max_concurrent_slices INTEGER NOT NULL DEFAULT 0;
