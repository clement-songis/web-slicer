-- Historique des envois vers les hôtes d'impression (lot 42), comme la
-- « Print host upload queue » d'Orca (PrintHostQueueDialog : hôte, taille,
-- fichier, état, message), conservé au-delà de la session.
CREATE TABLE printer_uploads (
    id            TEXT PRIMARY KEY,
    user_id       TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    printer_id    TEXT NOT NULL REFERENCES printers(id) ON DELETE CASCADE,
    gcode_id      TEXT REFERENCES gcodes(id) ON DELETE SET NULL,
    project_id    TEXT REFERENCES projects(id) ON DELETE SET NULL,
    filename      TEXT NOT NULL,
    host          TEXT NOT NULL,
    host_type     TEXT NOT NULL,
    size_bytes    INTEGER NOT NULL DEFAULT 0,
    action        TEXT NOT NULL,
    status        TEXT NOT NULL,
    path          TEXT,
    print_started INTEGER NOT NULL DEFAULT 0,
    error         TEXT,
    created_at    TEXT NOT NULL,
    finished_at   TEXT
);
CREATE INDEX printer_uploads_printer ON printer_uploads(user_id, printer_id, created_at);
