-- Hôtes d'impression comme Orca : type d'hôte et options (cf. SQLite 0008).
ALTER TABLE printers ADD COLUMN host_type text NOT NULL DEFAULT 'moonraker';
ALTER TABLE printers ADD COLUMN host_options jsonb NOT NULL DEFAULT '{}'::jsonb;
