# Feature Specification: Web-Slicer — parité OrcaSlicer multi-utilisateurs

**Feature Branch**: `001-orcaslicer-web-parity`

**Created**: 2026-07-09

**Status**: Draft

**Input**: User description: "Construire une application web multi-utilisateurs qui réplique l'intégralité de l'interface d'OrcaSlicer : import de modèles (STL/3MF/STEP/OBJ), scène 3D (placement, rotation, échelle, coupe, réparation, arrangement auto, multi-plateau), onglets de réglages Process/Filament/Imprimante avec TOUS les paramètres de audit/parameters.json, système de presets complet (héritage, profils vendor, profils utilisateur), slicing via l'engine, prévisualisation G-code par couches (types de lignes, vitesses, temps estimé), export G-code, et envoi vers imprimante Klipper via l'API Moonraker (compatible Mainsail). Sources de vérité exhaustives : audit/parameters.json, audit/ui_inventory.json, audit/presets_inventory.json — chaque entrée doit apparaître dans la spec, aucune exception. Inclure : comptes utilisateurs, bibliothèque de projets persistée, file d'attente de slicing côté serveur. Hors scope v1 : réécriture du cœur de slicing en Rust, calibrations avancées (les lister quand même en backlog v2 pour ne rien perdre)."

## Sources de vérité (normatives)

La parité fonctionnelle est définie par les inventaires régénérables du dossier
`audit/` et matérialisée intégralement dans les annexes jointes à cette spec :

| Annexe | Contenu normatif | Volume |
|---|---|---|
| [Annexe A](annexes/annexe-a-parametres.md) | Les 858 paramètres de `audit/parameters.json` — chaque ligne est une exigence | 858 paramètres |
| [Annexe B](annexes/annexe-b-interface.md) | Tout `audit/ui_inventory.json` : 21 pages de réglages, 100 sections, 525 lignes d'options, 103 items de menus, 70 items contextuels, 11 items de barres d'outils, 16 gizmos, 92 raccourcis | inventaire complet |
| [Annexe C](annexes/annexe-c-presets.md) | Les 65 vendeurs et 11 895 presets de `audit/presets_inventory.json` (liste nominative incorporée par référence, contrôlée par comptage exact) | 11 895 presets |

Les annexes sont régénérées par `python3 audit/generate_parity_annexes.py` après
chaque `python3 audit/run_all.py`. Références visuelles de l'UI cible :
[references/orca-home.png](references/orca-home.png) (accueil/projets),
[references/orca-prepare.png](references/orca-prepare.png) (vue Préparer :
panneau de réglages, plateau, barres d'outils, gizmos),
[references/orca-preview.png](references/orca-preview.png) (vue Aperçu :
légende des types de lignes, statistiques, curseurs, G-code). Toute entrée non implémentée DOIT figurer au
**registre d'exclusions** (`specs/001-orcaslicer-web-parity/exclusions.md`, créé
en phase de plan) avec justification — jamais d'omission silencieuse
(constitution, principe V).

## Clarifications

### Session 2026-07-09

- Q: Qui peut créer un compte sur l'instance ? → A: Configurable par instance — inscription ouverte, fermée (admin crée les comptes) ou sur invitation ; défaut : ouverte.
- Q: Quels flux email inclure en v1 (vérification d'adresse, mot de passe oublié) ? → A: Aucun — pas de dépendance SMTP en v1 ; réinitialisation de mot de passe par l'administrateur ; vérification d'email et « mot de passe oublié » autonomes reportés en v2.
- Q: Que deviennent les travaux de tranchage si le serveur redémarre ? → A: File persistée avec reprise automatique — les travaux en attente restent en file, les travaux interrompus en cours sont relancés de zéro.
- Q: Faut-il un quota de stockage par utilisateur ? → A: Pas de quota en v1 (l'admin surveille le disque) ; quota configurable par utilisateur reporté en v2.
- Q: Comment les projets sont-ils sauvegardés ? → A: Sauvegarde manuelle explicite (Ctrl+S, parité OrcaSlicer) complétée d'un brouillon de session restauré automatiquement après fermeture accidentelle (local à l'appareil, non partagé).
- Q: Quelles intégrations d'impression réseau en v1 ? → A: Moonraker (Klipper) uniquement ; les 15 autres types d'hôtes de l'enum `host_type` d'OrcaSlicer sont reportés en v2 (listés au backlog, aucun perdu). Les paramètres `printhost_*`/`host_type` restent pris en charge en données (Annexe A).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Du modèle au G-code (Priority: P1)

Un utilisateur connecté importe un modèle 3D (STL, 3MF, STEP ou OBJ), le voit
apparaître sur le plateau de son imprimante, lance le tranchage avec les presets
actifs et télécharge le G-code produit.

**Why this priority**: c'est la proposition de valeur minimale d'un slicer ;
sans ce flux, rien d'autre n'a de sens.

**Independent Test**: importer un STL de référence, trancher avec le preset
par défaut, vérifier qu'un G-code valide et complet est téléchargeable.

**Acceptance Scenarios**:

1. **Given** un compte connecté et un plateau vide, **When** l'utilisateur importe un fichier STL/3MF/STEP/OBJ valide, **Then** le modèle s'affiche sur le plateau, correctement posé et centré.
2. **Given** un modèle sur le plateau et des presets machine/filament/process actifs, **When** l'utilisateur lance le tranchage, **Then** un travail est créé, sa progression est visible, et il aboutit à un G-code téléchargeable.
3. **Given** un fichier corrompu ou d'un format non supporté, **When** l'utilisateur tente l'import, **Then** un message d'erreur clair est affiché et la scène reste inchangée.
4. **Given** un modèle qui déborde du volume d'impression, **When** l'utilisateur lance le tranchage, **Then** l'application le signale avant de trancher et identifie l'objet fautif.

---

### User Story 2 - Régler tous les paramètres (Priority: P1)

Un utilisateur ouvre les onglets de réglages Process, Filament et Imprimante et
retrouve exactement l'organisation d'OrcaSlicer : mêmes onglets, mêmes groupes,
mêmes paramètres, avec les modes d'affichage simple / advanced / expert.

**Why this priority**: la parité paramètres est l'engagement central du projet
(constitution, principe V) ; c'est ce qui distingue l'application d'un slicer
web simplifié.

**Independent Test**: comparer chaque page/section/option affichée avec
l'Annexe B et chaque paramètre (type, bornes, défaut, tooltip, mode) avec
l'Annexe A ; le contrôle automatisé de traçabilité passe à 100 % (hors registre
d'exclusions justifié).

**Acceptance Scenarios**:

1. **Given** l'onglet Process ouvert, **When** l'utilisateur parcourt les pages Quality, Strength, Speed, Support, Multimaterial, Others, **Then** toutes les sections et options de l'Annexe B s'y trouvent, dans le même ordre de regroupement.
2. **Given** le mode « simple » actif, **When** l'utilisateur bascule en « advanced » puis « expert », **Then** les paramètres apparaissent/disparaissent conformément au champ `mode` de l'Annexe A.
3. **Given** un paramètre numérique borné (ex. `layer_height`, min 0), **When** l'utilisateur saisit une valeur hors bornes, **Then** la valeur est refusée ou corrigée avec un message explicite reprenant la contrainte.
4. **Given** un paramètre à choix (enum), **When** l'utilisateur ouvre la liste, **Then** les valeurs et libellés proposés sont ceux de l'Annexe A.
5. **Given** n'importe quel paramètre, **When** l'utilisateur survole son libellé, **Then** l'infobulle affiche le tooltip d'origine.
6. **Given** un paramètre modifié par rapport au preset, **When** l'utilisateur le consulte, **Then** la modification est signalée visuellement et peut être annulée (retour à la valeur du preset), comme dans OrcaSlicer.

---

### User Story 3 - Presets système et utilisateur (Priority: P2)

Un utilisateur choisit son imprimante parmi les profils vendeurs (65 vendeurs,
Bambu Lab, Creality, Voron, etc.), sélectionne un filament et un process
compatibles, personnalise des valeurs et enregistre ses propres presets, qui
héritent des presets système.

**Why this priority**: sans profils système, chaque utilisateur devrait
configurer des centaines de paramètres à la main ; c'est la deuxième moitié de
la parité.

**Independent Test**: sélectionner « Bambu Lab A1 0.4 nozzle », vérifier que
les presets filament/process proposés sont filtrés par compatibilité, créer un
preset utilisateur dérivé, vérifier l'héritage (seules les valeurs surchargées
diffèrent du parent).

**Acceptance Scenarios**:

1. **Given** l'assistant de choix d'imprimante, **When** l'utilisateur cherche son modèle, **Then** les 65 vendeurs et leurs modèles (Annexe C) sont disponibles avec leurs variantes de buse.
2. **Given** une imprimante active, **When** l'utilisateur ouvre les listes de presets filament et process, **Then** seuls les presets compatibles (`compatible_printers`) instanciables sont proposés ; les presets abstraits (`instantiation=false`) n'apparaissent jamais.
3. **Given** un preset système modifié, **When** l'utilisateur enregistre sous un nouveau nom, **Then** un preset utilisateur est créé, rattaché à son compte, avec héritage du preset d'origine ; il est exportable et supprimable.
4. **Given** un preset utilisateur hérité, **When** le preset parent système est mis à jour, **Then** les valeurs non surchargées suivent le parent et les surcharges de l'utilisateur sont préservées.
5. **Given** deux comptes distincts, **When** chacun liste ses presets, **Then** aucun ne voit les presets personnels de l'autre.

---

### User Story 4 - Préparer la scène 3D (Priority: P2)

Un utilisateur manipule ses objets comme dans OrcaSlicer : déplacement,
rotation, mise à l'échelle, pose à plat, coupe, réparation de maillage,
opérations booléennes, arrangement automatique, duplication, et répartition sur
plusieurs plateaux.

**Why this priority**: la préparation de scène conditionne la qualité du
tranchage ; les gizmos et outils listés en Annexe B (§B.4, B.5) en font partie
intégrante de la parité.

**Independent Test**: dérouler sur un modèle de test chaque outil de l'Annexe B
§B.4/B.5 (16 gizmos, barres d'outils) et chaque action des menus contextuels
§B.3, et vérifier l'effet attendu sur la scène.

**Acceptance Scenarios**:

1. **Given** un objet sélectionné, **When** l'utilisateur utilise les gizmos Move/Rotate/Scale ou saisit des valeurs numériques, **Then** la transformation s'applique et est réversible (annuler/rétablir).
2. **Given** un objet sélectionné, **When** l'utilisateur active la coupe (Cut), **Then** il positionne le plan de coupe, choisit les options (garder moitiés, connecteurs) et obtient les pièces résultantes comme objets distincts.
3. **Given** un maillage non manifold, **When** l'utilisateur lance la réparation, **Then** les erreurs sont corrigées ou listées, et l'état réparé est signalé.
4. **Given** plusieurs objets sur le plateau, **When** l'utilisateur demande l'arrangement automatique, **Then** les objets sont disposés sans collision en respectant les dégagements de la machine.
5. **Given** plus d'objets que le plateau ne peut en contenir, **When** l'utilisateur ajoute un plateau, **Then** il peut répartir les objets entre plateaux et trancher chaque plateau indépendamment.
6. **Given** la scène active, **When** l'utilisateur utilise les raccourcis clavier de l'Annexe B §B.6 (groupes « Plater », « Objects List »), **Then** chaque raccourci déclenche l'action documentée.

---

### User Story 5 - Prévisualiser le G-code (Priority: P2)

Après tranchage, l'utilisateur explore le résultat couche par couche : types de
lignes colorés (périmètres, remplissage, supports…), vitesses, plage de
couches, déplacement dans une couche, et statistiques (temps estimé par
plateau, longueur/masse de filament).

**Why this priority**: la prévisualisation est l'outil de validation avant
impression ; sans elle, l'export est un saut de foi.

**Independent Test**: trancher un modèle de référence, vérifier que la vue par
couches distingue tous les types de lignes produits, que le curseur de couches
et le curseur intra-couche fonctionnent, et que le temps estimé s'affiche.

**Acceptance Scenarios**:

1. **Given** un tranchage terminé, **When** l'utilisateur ouvre la prévisualisation, **Then** le G-code est rendu par couches avec une légende par type de ligne (coloration par fonction), et chaque type peut être masqué/affiché.
2. **Given** la prévisualisation ouverte, **When** l'utilisateur passe en coloration par vitesse, **Then** l'échelle de vitesses s'affiche et les segments sont colorés en conséquence.
3. **Given** la prévisualisation ouverte, **When** l'utilisateur déplace le curseur vertical (couches) et horizontal (progression dans la couche), **Then** l'affichage se restreint à la sélection, y compris via les raccourcis du groupe « Preview » (Annexe B §B.6).
4. **Given** un tranchage terminé, **When** l'utilisateur consulte les statistiques, **Then** temps total estimé, temps par plateau, filament consommé (longueur, volume, masse) et nombre de changements d'outil sont affichés.

---

### User Story 6 - Comptes et bibliothèque de projets (Priority: P2)

Un visiteur crée un compte, se connecte, et retrouve à chaque session ses
projets (scène + réglages + presets référencés), ses presets personnels et ses
G-codes produits. Chaque espace utilisateur est étanche.

**Why this priority**: le multi-utilisateurs et la persistance sont ce qui
justifie la version web par rapport au logiciel de bureau.

**Independent Test**: créer deux comptes, un projet chacun, vérifier la
persistance après déconnexion/reconnexion et l'impossibilité d'accéder aux
ressources de l'autre compte (y compris par URL directe).

**Acceptance Scenarios**:

1. **Given** un visiteur, **When** il s'inscrit avec email et mot de passe, **Then** son compte est créé, son mot de passe n'est jamais stocké en clair, et il est connecté par session.
2. **Given** un utilisateur connecté, **When** il enregistre son travail comme projet, **Then** le projet (modèles, transformations, plateaux, valeurs de réglages, presets référencés) est restauré à l'identique à la prochaine ouverture.
3. **Given** un projet existant, **When** l'utilisateur rouvre l'application après déconnexion, **Then** sa bibliothèque liste ses projets avec vignette, date et imprimante cible, et permet d'ouvrir, dupliquer, renommer ou supprimer.
4. **Given** l'identifiant d'une ressource (projet, G-code, preset) d'un autre utilisateur, **When** un utilisateur tente d'y accéder, **Then** l'accès est refusé.
5. **Given** un utilisateur connecté, **When** sa session expire ou qu'il se déconnecte, **Then** toute action protégée redirige vers la connexion sans perte du travail sauvegardé.

---

### User Story 7 - File d'attente de slicing côté serveur (Priority: P3)

Les tranchages s'exécutent côté serveur dans une file d'attente : l'utilisateur
peut lancer un tranchage, continuer à travailler, suivre la progression, annuler
un travail, et retrouver l'historique de ses tranchages.

**Why this priority**: nécessaire à la tenue en charge multi-utilisateurs, mais
le flux mono-utilisateur (US1) peut fonctionner avec une exécution directe.

**Independent Test**: lancer plusieurs tranchages simultanés depuis deux
comptes, vérifier l'ordonnancement, la progression indépendante, l'annulation,
et qu'aucun résultat n'est attribué au mauvais compte.

**Acceptance Scenarios**:

1. **Given** un tranchage lancé, **When** l'utilisateur navigue ailleurs dans l'application, **Then** le travail continue côté serveur et une notification signale sa fin.
2. **Given** plusieurs travaux en attente, **When** l'utilisateur consulte sa file, **Then** il voit position, état (en attente, en cours, terminé, échoué, annulé) et progression de chacun, et peut annuler un travail non terminé.
3. **Given** un tranchage échoué (erreur moteur), **When** l'utilisateur consulte le travail, **Then** le message d'erreur du moteur est restitué de façon lisible et actionnable.
4. **Given** des travaux de plusieurs utilisateurs, **When** le serveur les exécute, **Then** l'ordonnancement est équitable et chaque utilisateur ne voit que ses propres travaux.

---

### User Story 8 - Imprimer via Klipper/Moonraker (Priority: P3)

L'utilisateur déclare une ou plusieurs imprimantes Klipper (URL Moonraker,
clé API éventuelle), envoie un G-code tranché vers l'imprimante, peut lancer
l'impression et suivre son état de base (comme depuis Mainsail).

**Why this priority**: dernier maillon du flux, dépend de tout le reste ;
l'export fichier (US1) offre déjà une solution de repli.

**Independent Test**: contre une instance Moonraker de test, déclarer
l'imprimante, téléverser un G-code, démarrer l'impression, vérifier l'état
remonté (impression en cours, progression, pause/arrêt).

**Acceptance Scenarios**:

1. **Given** les réglages d'imprimante physique, **When** l'utilisateur saisit l'adresse Moonraker (et clé API le cas échéant), **Then** la connexion est testée et le résultat affiché.
2. **Given** un G-code tranché et une imprimante déclarée joignable, **When** l'utilisateur choisit « Envoyer vers l'imprimante », **Then** le fichier est téléversé dans la bibliothèque G-code de l'imprimante, avec option de démarrage immédiat.
3. **Given** une impression lancée, **When** l'utilisateur consulte l'imprimante, **Then** l'état (impression, progression, températures) est affiché et il peut mettre en pause, reprendre ou annuler.
4. **Given** une imprimante injoignable, **When** l'utilisateur envoie un G-code, **Then** l'échec est signalé immédiatement avec la cause et le G-code reste disponible à l'export.

---

### Edge Cases

- Import d'un fichier volumineux (> 100 Mo) ou d'un maillage de plusieurs millions de triangles : l'application affiche la progression, reste utilisable, et signale les limites atteintes plutôt que d'échouer silencieusement.
- Import d'un 3MF de projet OrcaSlicer existant : les modèles, transformations et réglages embarqués sont repris ; les éléments non supportés sont listés à l'utilisateur.
- Deux onglets/navigateurs ouverts sur le même projet : la dernière sauvegarde gagne et l'utilisateur est averti d'un conflit (pas de corruption).
- Paramètres interdépendants (ex. hauteur de couche vs diamètre de buse, températures min/max du filament) : les incohérences déclenchent les mêmes avertissements/corrections qu'OrcaSlicer.
- Preset système mis à jour entre deux versions de l'application : les presets utilisateur dérivés restent valides ; les clés legacy des profils (287 clés renommées/héritées identifiées dans l'audit) sont acceptées à l'import et converties.
- Tranchage annulé en plein calcul : les ressources serveur sont libérées, aucun G-code partiel n'est proposé.
- Perte de connexion pendant un envoi Moonraker : reprise ou échec propre, jamais de fichier corrompu côté imprimante.
- Utilisateur supprimant son compte : ses projets, presets et G-codes sont supprimés (BDD et fichiers).

## Requirements *(mandatory)*

### Functional Requirements

**Parité (traçabilité normative)**

- **FR-001**: Chacun des 858 paramètres de l'Annexe A DOIT être pris en charge de bout en bout (stockage, validation selon type/bornes/enum, valeur par défaut, exposition selon son groupe) ou inscrit au registre d'exclusions avec justification.
- **FR-002**: Chaque élément de l'Annexe B (21 pages, 100 sections, 525 lignes d'options, 103 items de menus, 70 items contextuels, 11 items de barres d'outils, 16 gizmos, 92 raccourcis) DOIT être traçable vers un composant ou un point d'entrée de l'application, ou inscrit au registre d'exclusions.
- **FR-003**: La traçabilité FR-001/FR-002 DOIT être vérifiable automatiquement (contrôle croisé entre les inventaires `audit/*.json` et une matrice de traçabilité maintenue dans le dépôt) et exécutée à chaque jalon.
- **FR-004**: Les paramètres du groupe `sla` (76) DOIVENT être stockés et validés (parité de données) sans exposition UI, OrcaSlicer n'exposant pas d'onglet SLA — décision consignée au registre d'exclusions UI.
- **FR-005**: L'UI de réglages DOIT reproduire l'organisation d'OrcaSlicer (mêmes onglets, mêmes groupes, même ordre) avec les trois modes d'affichage simple/advanced/expert pilotés par le champ `mode` de chaque paramètre. Les améliorations UX (recherche, navigation) sont additives uniquement.

**Import et scène 3D**

- **FR-010**: L'application DOIT importer les formats STL, 3MF (y compris projets OrcaSlicer), STEP et OBJ, avec rapport d'erreur clair en cas d'échec.
- **FR-011**: L'utilisateur DOIT pouvoir déplacer, tourner, mettre à l'échelle (uniforme et par axe), poser à plat, et positionner numériquement chaque objet ; toutes les transformations sont annulables (undo/redo).
- **FR-012**: L'application DOIT fournir les outils de scène de l'Annexe B : coupe avec plan positionnable et connecteurs, réparation de maillage avec rapport, opérations booléennes, simplification, texte/SVG en relief, mesure, peinture de supports, de seams et de fuzzy skin, segmentation multi-matériaux, oreilles de bord (brim ears).
- **FR-013**: L'arrangement automatique DOIT disposer les objets sans collision en respectant les dégagements machine, sur un ou plusieurs plateaux ; l'orientation automatique DOIT être disponible par objet et par plateau.
- **FR-014**: L'application DOIT gérer plusieurs plateaux par projet : ajout, suppression, réglages par plateau (type de plaque, position), tranchage par plateau ou global.
- **FR-015**: La liste d'objets DOIT permettre sélection, regroupement, duplication, verrouillage, masquage, attribution d'extrudeur/filament par objet et par pièce, et réglages par objet (surcharge des paramètres process).

**Presets**

- **FR-020**: L'application DOIT embarquer l'intégralité des profils système de l'Annexe C (65 vendeurs, 11 895 presets machine_model/machine/filament/process) avec résolution complète de l'héritage (`inherits`) et respect du flag `instantiation`.
- **FR-021**: Les listes de presets DOIVENT être filtrées par compatibilité (imprimante active, variante de buse, `compatible_printers`).
- **FR-022**: L'utilisateur DOIT pouvoir créer, renommer, dupliquer, supprimer, exporter et importer des presets personnels, héritant d'un preset système ou utilisateur ; les valeurs surchargées sont identifiables et réinitialisables individuellement.
- **FR-023**: Les 287 clés legacy identifiées dans l'audit DOIVENT être acceptées à l'import de profils et converties vers leurs équivalents actuels.
- **FR-024**: La sélection courante (imprimante + filament(s) + process) DOIT être persistée par projet et restaurée à l'ouverture.

**Tranchage et file d'attente**

- **FR-030**: Le tranchage DOIT s'exécuter côté serveur via le moteur de parité OrcaSlicer et produire un G-code identique à celui d'OrcaSlicer pour mêmes entrées (modèle, presets, valeurs), aux métadonnées près.
- **FR-031**: Les tranchages DOIVENT être ordonnancés dans une file serveur : états en attente/en cours/terminé/échoué/annulé, progression, annulation, historique par utilisateur, isolation entre comptes. La file survit au redémarrage du serveur : les travaux en attente sont conservés et les travaux interrompus en cours d'exécution sont automatiquement relancés de zéro.
- **FR-032**: Les avertissements et erreurs du moteur (objets hors plateau, supports requis, paramètres incohérents) DOIVENT être restitués à l'utilisateur en langage clair, rattachés à l'objet ou au paramètre concerné.

**Prévisualisation et export**

- **FR-040**: La prévisualisation DOIT afficher le G-code par couches avec coloration par type de ligne (tous les types produits par le moteur : périmètres externes/internes, surfaces, remplissage, supports, ponts, jupe/bordure, tour de purge, déplacements, rétractions…), chaque type pouvant être masqué.
- **FR-041**: La prévisualisation DOIT proposer les colorations par vitesse, type de ligne, hauteur de couche, largeur de ligne, débit, température et numéro de filament, avec légende/échelle.
- **FR-042**: L'utilisateur DOIT pouvoir naviguer par curseur de couches (plage min–max) et par curseur de progression intra-couche.
- **FR-043**: Les statistiques DOIVENT inclure : temps total et par plateau, décomposition par type de ligne, filament par extrudeur (longueur, volume, masse, coût estimé), nombre de changements de filament.
- **FR-044**: Le G-code DOIT être téléchargeable par plateau, et le projet exportable en 3MF compatible OrcaSlicer.

**Comptes et bibliothèque**

- **FR-050**: Inscription par email + mot de passe ; authentification par session ; mots de passe stockés uniquement sous forme hachée robuste ; déconnexion et expiration de session. La politique d'inscription est configurable par instance : ouverte (défaut), fermée (comptes créés par un administrateur) ou sur invitation. Aucun flux email en v1 (pas de dépendance SMTP) : la réinitialisation de mot de passe s'effectue par l'administrateur de l'instance ; vérification d'email et « mot de passe oublié » autonomes sont reportés en v2.
- **FR-051**: Chaque ressource (projet, modèle, preset utilisateur, G-code, imprimante déclarée, travail de tranchage) DOIT appartenir à un compte et être inaccessible aux autres comptes.
- **FR-052**: La bibliothèque DOIT lister les projets avec vignette, dates, imprimante cible, et permettre ouverture, duplication, renommage, suppression ; la sauvegarde d'un projet capture scène, plateaux, réglages et références de presets. La sauvegarde est manuelle et explicite (Ctrl+S, parité OrcaSlicer) ; un brouillon de session est restauré automatiquement après fermeture accidentelle (local à l'appareil, sans conflit avec la sauvegarde serveur).
- **FR-053**: Les fichiers utilisateur (modèles importés, G-codes) DOIVENT être conservés et supprimés avec le compte ou la ressource parente.

**Impression réseau**

- **FR-060**: L'utilisateur DOIT pouvoir déclarer des imprimantes Klipper par URL Moonraker (+ clé API optionnelle), tester la connexion, et les rattacher à un profil machine. Moonraker est la seule intégration d'impression réseau de la v1 ; les 15 autres types d'hôtes d'OrcaSlicer sont reportés au backlog v2. Les paramètres `printhost_*` et l'enum `host_type` complet restent stockés et validés (parité de données, Annexe A).
- **FR-061**: L'envoi d'un G-code vers Moonraker DOIT couvrir : téléversement dans la bibliothèque de l'imprimante, démarrage immédiat optionnel, suivi d'état (progression, températures), pause/reprise/annulation.
- **FR-062**: Les échecs réseau DOIVENT être signalés immédiatement avec leur cause, sans perte du G-code.

**Interface générale**

- **FR-070**: Les actions des menus et barres d'outils de l'Annexe B §B.2–B.5 DOIVENT être disponibles ; celles sans objet dans un contexte web ou liées à des services propriétaires sont consignées au registre d'exclusions.
- **FR-071**: Les raccourcis clavier de l'Annexe B §B.6 DOIVENT être actifs dans les contextes correspondants (global, plateau, liste d'objets, prévisualisation), adaptés aux conventions du navigateur lorsque le raccourci d'origine est réservé (consigné en exclusions le cas échéant).
- **FR-072**: L'interface DOIT être utilisable en français et en anglais (les libellés d'origine d'OrcaSlicer étant en anglais, la parité des libellés prime ; la traduction est additive).

### Key Entities

- **Utilisateur** : compte (email, mot de passe haché), sessions, espace de stockage isolé.
- **Projet** : scène (objets, transformations, plateaux), valeurs de réglages, références de presets, vignette ; appartient à un utilisateur.
- **Modèle** : fichier 3D importé (STL/3MF/STEP/OBJ) + maillage dérivé ; rattaché à un projet.
- **Plateau** : sous-ensemble de la scène avec réglages propres ; unité de tranchage.
- **Preset** : profil de type machine_model/machine/filament/process ; système (vendeur, en lecture seule, avec héritage) ou utilisateur (dérivé, modifiable) ; ensemble de valeurs de paramètres du registre (Annexe A).
- **Paramètre** : définition issue de l'Annexe A (clé, type, défaut, bornes, enum, mode, groupe, tooltip, catégorie).
- **Travail de tranchage** : plateau + presets résolus + état + progression + résultat (G-code, statistiques, avertissements) ; appartient à un utilisateur.
- **G-code** : fichier produit + métadonnées (temps estimé, filament, vignettes) ; exportable ou envoyable à une imprimante.
- **Imprimante déclarée** : cible Moonraker (URL, clé API) liée à un profil machine ; appartient à un utilisateur.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Le contrôle de traçabilité automatisé rapporte 100 % des 858 paramètres et 100 % des éléments d'interface de l'Annexe B couverts ou justifiés au registre d'exclusions ; le registre ne contient aucune entrée « non justifiée ».
- **SC-002**: Les 11 895 presets système sont importés et résolus (héritage) sans erreur ; les comptages par type correspondent exactement à l'Annexe C ; 100 % des presets instanciables sont sélectionnables via le filtre de compatibilité.
- **SC-003**: Pour un panel de 10 modèles de référence et 5 combinaisons de presets, le G-code produit est fonctionnellement identique à celui d'OrcaSlicer de bureau (mêmes trajectoires aux métadonnées près) et le temps estimé diffère de moins de 1 %.
- **SC-004**: Un utilisateur qui connaît OrcaSlicer retrouve n'importe quel paramètre au même endroit (onglet + groupe) sans recherche dans 95 % des cas testés (test utilisateur sur 40 paramètres tirés au sort).
- **SC-005**: Le flux complet import STL (< 20 Mo) → tranchage (preset standard) → prévisualisation → export s'accomplit en moins de 3 minutes, dont moins de 90 secondes de tranchage pour un modèle de 100 000 triangles.
- **SC-006**: 10 tranchages simultanés lancés par des comptes différents aboutissent tous, sans mélange de résultats et sans indisponibilité de l'interface.
- **SC-007**: La scène 3D reste fluide (interactions sans à-coups perceptibles) avec 20 objets totalisant 2 millions de triangles sur un poste de bureau courant.
- **SC-008**: Aucun accès inter-comptes possible : une campagne de tests d'isolation (accès direct par identifiants de ressources d'autrui) échoue à 100 %.
- **SC-009**: L'envoi d'un G-code de 50 Mo vers une instance Moonraker aboutit en moins de 2 minutes sur réseau local, avec suivi d'état correct dans 100 % des cas de test.

## Assumptions

- **Parité UI web** : les éléments d'OrcaSlicer sans objet en contexte web (fenêtres système, périphériques 3Dconnexion, services cloud/compte Bambu, écrans « Device » propriétaires, flux vidéo caméra) sont candidats au registre d'exclusions, jamais omis silencieusement. L'impression réseau est assurée par Moonraker uniquement en v1 ; les autres hôtes (`host_type`) sont au backlog v2.
- **Paramètres SLA** : OrcaSlicer n'expose pas d'onglet SLA ; les 76 paramètres `sla` sont pris en charge en données/API sans UI (FR-004).
- **Paramètres CLI et placeholders** : les groupes `cli:*` (52) correspondent aux actions serveur (tranchage, transformations) ; les groupes `other:*` (55) sont les variables de templates G-code — pris en charge par le moteur de placeholders, pas par l'UI de réglages.
- **Langue** : libellés de parité en anglais (source OrcaSlicer), traduction française additive (FR-072).
- **Limites d'upload** : taille maximale de fichier importé fixée à 500 Mo par défaut (configurable), au-delà des besoins courants.
- **Stockage** : pas de quota par utilisateur en v1 (la surveillance du disque relève de l'administrateur d'instance) ; quota configurable reporté en v2.
- **Mono-instance v1** : dimensionnement initial ~50 utilisateurs actifs, extensible ensuite (la constitution impose déjà la substituabilité de la persistance).

## Out of Scope (v1)

- **Réécriture du cœur de slicing en Rust natif** : la v1 s'appuie sur le moteur OrcaSlicer existant derrière l'abstraction prévue par la constitution. La substituabilité est garantie, la réécriture est reportée.
- **Calibrations avancées** : reportées en v2, listées exhaustivement ci-dessous pour ne rien perdre (source : menu Calibration et assistants d'OrcaSlicer, Annexe B §B.2).

### Backlog v2 — Calibrations (aucune perte)

| Élément OrcaSlicer | Description |
|---|---|
| Temperature | Tour de température (choix de la température de buse optimale) |
| Max flowrate | Test de débit volumétrique maximal |
| Pressure advance | Calibration du pressure advance / linear advance (tour et motif) |
| Flow ratio | Calibration du ratio de débit (passes 1 et 2, YOLO) |
| Retraction | Tour de test de rétraction |
| Cornering (Jerk/Junction deviation) | Calibration de la gestion des angles |
| VFA | Test de vibrations verticales (Vertical Fine Artifacts) |
| Calibration Guide | Guide de calibration intégré |
| Assistants de calibration guidés | CalibrationWizard d'OrcaSlicer (flux préréglés par imprimante) |
| Réglages `calib.cpp` associés | Génération procédurale des modèles de test du moteur |

- **Intégrations d'impression réseau autres que Moonraker** : reportées en v2, listées exhaustivement ci-dessous pour ne rien perdre (source : enum `host_type`, Annexe A).

### Backlog v2 — Intégrations d'hôtes d'impression (aucune perte)

| Valeur `host_type` | Libellé OrcaSlicer |
|---|---|
| `prusalink` | PrusaLink |
| `prusaconnect` | PrusaConnect |
| `octoprint` | Octo/Klipper (OctoPrint) |
| `duet` | Duet |
| `flashair` | FlashAir |
| `astrobox` | AstroBox |
| `repetier` | Repetier |
| `mks` | MKS |
| `esp3d` | ESP3D |
| `crealityprint` | CrealityPrint |
| `obico` | Obico |
| `flashforge` | Flashforge |
| `simplyprint` | SimplyPrint |
| `elegoolink` | Elegoo Link |
| `3dprinteros` | 3DPrinterOS |

(`moonraker` — Moonraker (Klipper) — est la seule intégration v1.)

- **Quotas de stockage par utilisateur** : reportés en v2 (quota configurable par instance) ; en v1 la surveillance du disque relève de l'administrateur.
- **Flux email autonomes** : vérification d'adresse et « mot de passe oublié » par email reportés en v2 ; en v1 la réinitialisation passe par l'administrateur.
- **Fonctionnalités propriétaires Bambu** : impression via le cloud Bambu, AMS/gestion RFID, flux caméra — consignées au registre d'exclusions v1 (l'impression passe par Moonraker).
- **SLA/résine** : pas d'UI ni de tranchage SLA (parité de données seulement, FR-004).

## Dependencies

- Inventaires `audit/*.json` régénérables (`audit/run_all.py`) et annexes synchronisées (`audit/generate_parity_annexes.py`) — sources de vérité de cette spec.
- `vendor/OrcaSlicer` (lecture seule) : moteur de tranchage, profils système (`resources/profiles`), textures/modèles de plateaux.
- Une instance Klipper/Moonraker accessible pour les tests d'impression réseau (une instance simulée suffit pour l'intégration continue).
