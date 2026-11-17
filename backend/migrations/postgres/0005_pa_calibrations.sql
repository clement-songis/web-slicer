-- Historique des calibrations d'avance de pression retenues (par imprimante,
-- filament et buse), comme PACalibResult / CaliHistoryDialog d'Orca.
CREATE TABLE pa_calibrations (
    id                 uuid PRIMARY KEY,
    user_id            uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    printer_id         uuid REFERENCES printers(id) ON DELETE SET NULL,
    machine_preset     text NOT NULL,
    filament_preset    text NOT NULL,
    filament_id        text,
    nozzle_diameter    double precision NOT NULL,
    extruder_id        bigint NOT NULL DEFAULT 0,
    nozzle_volume_type text,
    k_value            double precision NOT NULL,
    n_coef             double precision NOT NULL DEFAULT 1.0,
    method             text NOT NULL,
    name               text NOT NULL,
    note               text,
    created_at         timestamptz NOT NULL,
    updated_at         timestamptz NOT NULL
);
CREATE INDEX pa_calibrations_user ON pa_calibrations(user_id, created_at);
