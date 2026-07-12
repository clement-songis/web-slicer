-- Décodage des modèles côté serveur (T123) : trace persistante d'un échec de
-- conversion moteur. NULL = pas d'échec (conversion réussie, en cours, ou
-- format sans conversion). Non NULL = message d'erreur → `/mesh` renvoie 422.
ALTER TABLE models ADD COLUMN conversion_error TEXT;
