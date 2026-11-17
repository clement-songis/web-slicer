-- Historique des calibrations d'avance de pression retenues (par imprimante,
-- filament et buse), comme PACalibResult / CaliHistoryDialog d'Orca.
CREATE TABLE pa_calibrations (
    id                 TEXT PRIMARY KEY,
    user_id            TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    printer_id         TEXT REFERENCES printers(id) ON DELETE SET NULL,
    machine_preset     TEXT NOT NULL,
    filament_preset    TEXT NOT NULL,
    filament_id        TEXT,
    nozzle_diameter    REAL NOT NULL,
    extruder_id        INTEGER NOT NULL DEFAULT 0,
    nozzle_volume_type TEXT,
    k_value            REAL NOT NULL,
    n_coef             REAL NOT NULL DEFAULT 1.0,
    method             TEXT NOT NULL,
    name               TEXT NOT NULL,
    note               TEXT,
    created_at         TEXT NOT NULL,
    updated_at         TEXT NOT NULL
);
CREATE INDEX pa_calibrations_user ON pa_calibrations(user_id, created_at);
