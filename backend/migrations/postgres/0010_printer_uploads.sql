-- Historique des envois vers les hôtes d'impression (lot 42), comme la
-- « Print host upload queue » d'Orca (PrintHostQueueDialog : hôte, taille,
-- fichier, état, message), conservé au-delà de la session.
CREATE TABLE printer_uploads (
    id            uuid PRIMARY KEY,
    user_id       uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    printer_id    uuid NOT NULL REFERENCES printers(id) ON DELETE CASCADE,
    gcode_id      uuid REFERENCES gcodes(id) ON DELETE SET NULL,
    project_id    uuid REFERENCES projects(id) ON DELETE SET NULL,
    filename      text NOT NULL,
    host          text NOT NULL,
    host_type     text NOT NULL,
    size_bytes    bigint NOT NULL DEFAULT 0,
    action        text NOT NULL,
    status        text NOT NULL,
    path          text,
    print_started boolean NOT NULL DEFAULT false,
    error         text,
    created_at    timestamptz NOT NULL,
    finished_at   timestamptz
);
CREATE INDEX printer_uploads_printer ON printer_uploads(user_id, printer_id, created_at);
