-- L'identité d'un preset système est (kind, name, vendor), pas (kind, name) :
-- les presets de base d'OrcaSlicer (`fdm_process_common`, `fdm_filament_pla`…)
-- sont livrés une fois par vendeur avec le même nom mais des valeurs propres
-- (129 homonymes dans les 11 895 profils). L'index initial `presets_unique_system`
-- sur (kind, name) rejetait le seed réel ; on l'élargit à (kind, name, vendor).
-- La résolution d'héritage préférait déjà le même vendeur en cas d'homonymie
-- (`build_chain`/`pick_parent`), ce changement aligne le schéma sur ce modèle.
DROP INDEX presets_unique_system;
CREATE UNIQUE INDEX presets_unique_system
    ON presets (kind, name, vendor) WHERE user_id IS NULL;
