-- Quotas d'instance (cf. SQLite 0010). 0 = illimité.
ALTER TABLE instance_settings ADD COLUMN storage_quota_bytes bigint NOT NULL DEFAULT 0;
ALTER TABLE instance_settings ADD COLUMN max_concurrent_slices bigint NOT NULL DEFAULT 0;
