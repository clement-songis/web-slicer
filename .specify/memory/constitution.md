<!--
Sync Impact Report
==================
Version change: (template) → 1.0.0
Modified principles: n/a (adoption initiale)
Added sections:
  - Principes fondamentaux I–V
  - Contraintes techniques (stack & sécurité)
  - Workflow de développement & portes qualité
  - Gouvernance
Removed sections: n/a
Templates:
  - .specify/templates/plan-template.md ✅ compatible (gates dérivées de ce fichier)
  - .specify/templates/spec-template.md ✅ compatible (aucune section obligatoire ajoutée)
  - .specify/templates/tasks-template.md ✅ mis à jour (tests obligatoires backend/engine,
    conformément au principe IV — le template disait « Tests are OPTIONAL »)
  - .specify/templates/checklist-template.md ✅ compatible
Follow-up TODOs: aucun
-->

# Constitution du projet Web-Slicer

Web-Slicer est un slicer 3D web multi-utilisateurs visant la parité fonctionnelle
complète avec OrcaSlicer (vendored dans `vendor/OrcaSlicer`). Les inventaires
générés dans `audit/` (`parameters.json`, `ui_inventory.json`, `engine_api.json`,
`presets_inventory.json`) constituent la référence opposable de cette parité et
sont régénérables à tout moment via `python3 audit/run_all.py`.

## Principes fondamentaux

### I. Monorepo à trois couches, séparation stricte

Le dépôt est un monorepo composé de trois unités livrables :

- `frontend/` : SvelteKit, TypeScript en mode strict, Tailwind CSS.
- `backend/` : Rust, axum.
- `engine/` : crate Rust isolée qui encapsule libslic3r d'OrcaSlicer.

La séparation domaine / adaptateurs / présentation est obligatoire dans chaque
unité. Il est INTERDIT de placer de la logique métier dans les composants Svelte
ou dans les handlers HTTP : composants et handlers ne font que valider les
entrées, appeler le domaine et mettre en forme les sorties.

**Rationale** : garantir que chaque couche est testable isolément et que les
choix d'infrastructure (framework web, moteur de slicing, base de données)
restent substituables sans réécriture du métier.

### II. Moteur derrière le trait `SlicerEngine`

`engine/` expose un trait Rust `SlicerEngine` dont les types et méthodes
reflètent 1:1 l'API C++ publique de libslic3r ; `audit/engine_api.json` fait
foi pour ce miroir. Règles non négociables :

- Le backend ne dépend QUE du trait `SlicerEngine`, jamais d'une implémentation
  concrète. Aucun `use` d'un module d'implémentation du moteur hors de `engine/`.
- Implémentation v1 : bridge FFI via `cxx` ou, à défaut, fallback CLI vers le
  binaire `orca-slicer`. Les deux se cachent derrière le même trait.
- Le cœur DOIT être remplaçable par une implémentation Rust native sans
  modifier une seule ligne du backend.

**Rationale** : le moteur est la partie la plus risquée et la plus évolutive ;
l'isoler derrière un contrat stable protège tout le reste du système.

### III. Persistance derrière le trait `Storage`

Tout accès aux données passe par un trait `Storage`. Implémentation v1 :
SQLite via `sqlx`. Postgres DOIT être activable par feature flag ou
configuration, sans aucune migration du code métier. Le code métier ne
connaît ni SQL ni le SGBD choisi.

**Rationale** : permettre le passage mono-instance → déploiement multi-nœuds
sans refonte.

### IV. TDD et qualité non négociables

- TDD sur le backend et l'engine : chaque tâche livre ses tests, écrits avant
  ou avec l'implémentation ; une tâche sans tests est incomplète.
- `cargo clippy -D warnings` et `rustfmt` DOIVENT passer sur tout le code Rust.
- `eslint` et `prettier` DOIVENT passer sur tout le code frontend.
- Commits atomiques : un commit par tâche, message décrivant la tâche.

**Rationale** : la parité avec OrcaSlicer représente des centaines de
paramètres interdépendants ; sans filet de tests systématique, les régressions
sont indétectables.

### V. Parité totale avec OrcaSlicer (NON NÉGOCIABLE)

Aucune fonctionnalité d'OrcaSlicer ne peut être omise :

- Chaque paramètre de `audit/parameters.json` et chaque élément de
  `audit/ui_inventory.json` (onglets, sections, options, menus, barres
  d'outils, gizmos, raccourcis) DOIT être traçable jusqu'à un composant
  frontend et/ou un endpoint backend.
- L'UI reprend l'organisation d'OrcaSlicer : mêmes onglets, mêmes groupes,
  mêmes modes simple/advanced/expert.
- Les améliorations UX sont permises si et seulement si elles n'enlèvent
  rien : tout écart est additif.
- Toute exception (paramètre inapplicable au web, fonctionnalité liée au
  matériel Bambu, etc.) DOIT être documentée avec justification dans un
  registre d'exclusions versionné, jamais omise silencieusement.

**Rationale** : la proposition de valeur du projet est « OrcaSlicer dans le
navigateur », pas un sous-ensemble.

## Contraintes techniques (stack & sécurité)

- Stack imposée : SvelteKit + TypeScript strict + Tailwind (frontend), Rust +
  axum (backend), Rust + cxx/CLI (engine), sqlx (persistance). Tout ajout de
  dépendance structurante se justifie dans le plan de la feature concernée.
- Comptes utilisateurs : authentification par sessions côté serveur ; mots de
  passe hachés avec argon2 ; jamais de mot de passe ni de hash réversible en
  clair où que ce soit.
- Chaque utilisateur dispose d'un espace isolé pour ses profils, projets et
  G-codes, matérialisé en base ET sur le filesystem. Aucune requête ni aucun
  chemin de fichier ne peut franchir la frontière d'un utilisateur sans
  contrôle d'accès explicite.
- `vendor/OrcaSlicer` est en lecture seule : on le parse, on le wrappe, on ne
  le modifie pas.

## Workflow de développement & portes qualité

- Chaque feature suit le cycle spec → plan → tasks → implémentation
  (Spec Kit), avec le « Constitution Check » du plan comme porte bloquante.
- Portes de merge obligatoires : compilation sans warning
  (`cargo clippy -D warnings`), formatage (`rustfmt`, `prettier`), lint
  (`eslint`), tests (`cargo test`, tests frontend) au vert.
- Contrôle de parité : `python3 audit/run_all.py` régénère les inventaires ;
  la traçabilité paramètres/UI ↔ composants/endpoints est vérifiée à chaque
  jalon et lors du contrôle final du projet. Un écart non documenté dans le
  registre d'exclusions bloque le jalon.
- La revue de code vérifie explicitement les principes I–V (dépendances
  inter-couches, absence de logique métier dans la présentation, présence
  des tests, traçabilité de parité).

## Gouvernance

- Cette constitution prévaut sur toute autre pratique du dépôt. En cas de
  conflit entre un document et la constitution, la constitution gagne.
- Amendement : proposition écrite (PR modifiant ce fichier) décrivant le
  changement, sa justification et son impact sur les templates
  `.specify/templates/*` et les specs en cours ; les artefacts dépendants
  sont mis à jour dans la même PR.
- Versionnage sémantique de la constitution :
  - MAJOR : suppression ou redéfinition incompatible d'un principe.
  - MINOR : nouveau principe ou section, ou extension matérielle d'une règle.
  - PATCH : clarification, reformulation, correction sans effet normatif.
- Conformité : toute PR doit être vérifiable au regard des principes I–V ;
  toute complexité dérogatoire est justifiée dans la section « Complexity
  Tracking » du plan de la feature concernée.

**Version**: 1.0.0 | **Ratified**: 2026-07-09 | **Last Amended**: 2026-07-09
