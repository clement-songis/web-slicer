-- Reproductibilité des nouveaux jobs ; NULL conserve les lignes historiques.
ALTER TABLE slicing_jobs ADD COLUMN scene TEXT;
