-- Hôtes d'impression comme Orca (OctoPrint, PrusaLink, Duet, Repetier, MKS,
-- FlashAir, AstroBox, ESP3D…) : type d'hôte et options (authentification
-- PrusaLink, imprimante Repetier ; mot de passe chiffré). moonraker_url porte
-- l'adresse de l'hôte (print_host d'Orca) quel que soit le type.
ALTER TABLE printers ADD COLUMN host_type TEXT NOT NULL DEFAULT 'moonraker';
ALTER TABLE printers ADD COLUMN host_options TEXT NOT NULL DEFAULT '{}';
