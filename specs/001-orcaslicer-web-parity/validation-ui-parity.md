# Validation de parité UI (T119, US4/US6/US8)

Campagne de vérification visuelle de la parité d'interface entre web-slicer et
OrcaSlicer desktop, couvrant les cinq écrans de référence (accueil, préparer,
aperçu, appareil, projet). Clôt la phase 12 (câblage de l'éditeur), en tandem
avec la **garde anti-récidive** ajoutée à `audit/check_traceability.py`
(contrôle « construit-mais-non-câblé », voir plus bas).

## Protocole

1. **Références OrcaSlicer** : `specs/001-orcaslicer-web-parity/references/`
   — `orca-home.png`, `orca-prepare.png`, `orca-preview.png` (captures de la
   version desktop 2.4.1, thème sombre).
2. **Captures web-slicer** : pilotage headless de `playwright-core` (store Nix)
   à 1600×900, script `scratchpad/capture-parity.mjs`. Compte utilisateur créé
   à la volée (`/api/auth/register` + `/login`), deux projets semés
   (`Papa.3mf`, `Cube demo`), un cube STL binaire injecté dans `Cube demo` pour
   peupler la scène. Sorties dans `references/captures/{home,prepare,preview,
   device,project}.png`.
3. **Critère** : concordance **structurelle** — mêmes zones, mêmes groupes de
   contrôles, mêmes libellés (parité i18n), au même emplacement relatif. Les
   écarts résiduels sont listés et, s'ils sont volontaires, tracés dans
   `exclusions.md` ; sinon ils deviennent des tâches de suivi.
4. **Santé runtime** : aucune exception page (`pageerror`) tolérée pendant la
   traversée des cinq écrans.

### Reproduire la campagne

```sh
# serveur de dev sur :5174 (frontend) + backend sur :8080
nix develop --command node scratchpad/capture-parity.mjs
# → references/captures/*.png
python3 audit/check_traceability.py   # garde de câblage incluse
```

## Résultats

**Exceptions page sur la traversée complète : 0.** Les cinq onglets se montent
et se démontent sans erreur console.

| Écran | Référence | Capture | Concordance structurelle |
|-------|-----------|---------|--------------------------|
| Accueil / bibliothèque | `orca-home.png` | `captures/home.png` | ✅ barre latérale (compte + Récent + lien externe), 2 cartes (Nouveau projet / Ouvrir un projet 3mf), grille « Récemment ouvert » avec vignettes + pastille Supprimer |
| Préparer | `orca-prepare.png` | `captures/prepare.png` | ✅ 3 zones : colonne de config (Imprimante → Filament → Traitement, onglets Traitement/Filament/Imprimante, Simple/Advanced/Expert, recherche, onglets Quality/Strength/Support/Multimaterial/Others), viewport central (plateau + modèle), rail de gizmos vertical ; barre de menus Fichier/Édition/Vue/Aide + onglets supérieurs + barre d'outils de plateau |
| Aperçu | `orca-preview.png` | `captures/preview.png` | ⚠️ onglet actif + état vide correct (« Aperçu G-code : tranchez la scène pour l'afficher ») ; l'aperçu **peuplé** exige un slicing (non déclenché ici, voir écart 3) |
| Appareil | — (parité fonctionnelle) | `captures/device.png` | ✅ panneau « Appareils », état vide « Aucune imprimante déclarée » + liens « Gérer / Déclarer une imprimante » |
| Projet | — (parité fonctionnelle) | `captures/project.png` | ✅ panneau « Projet » : nom éditable + Renommer, métadonnées Identifiant/Version/Créé le/Modifié le |

## Écarts résiduels

1. **Thème de l'accueil** — l'éditeur force le thème sombre (parité prepare/
   preview) ; la bibliothèque respecte la préférence utilisateur et s'affiche
   en clair par défaut. La référence `orca-home.png` est sombre. Écart
   **cosmétique**, non bloquant : la structure est identique et le thème sombre
   reste disponible via la bascule de la barre latérale.
2. **Colonne de config persistante hors Préparer** — sous les onglets Aperçu/
   Appareil/Projet, la colonne de réglages de gauche reste montée, alors
   qu'OrcaSlicer réattribue toute la fenêtre à ces onglets. Écart **mineur** de
   disposition, sans perte de fonction (le panneau de l'onglet occupe la zone
   centrale). Candidat au raffinement v2.
3. **Aperçu peuplé** — le rendu des couches G-code suppose un slicing réussi,
   qui exige des presets imprimante/filament/traitement sélectionnés. Sur un
   compte neuf sans preset, l'onglet affiche son état vide (validé ci-dessus).
   La parité de l'aperçu **peuplé** (types de ligne, StatsPanel, scrubber de
   couches) est couverte par les tests unitaires du module `lib/preview` et par
   la campagne de slicing SC-004 ; elle n'est pas rejouée ici faute de presets
   semés dans la traversée.

Aucun de ces écarts n'introduit d'omission de parité non tracée : (1) et (2)
sont des raffinements de disposition, (3) est une limite du décor de test, pas
du produit.

## Garde anti-récidive « construit-mais-non-câblé » (T119)

`audit/check_traceability.py` gagne un contrôle P4 (`check_wired`) qui vérifie
que **chaque chemin cible** de `traceability-map.json` (registres `gizmos`,
`toolbars`, `context_menu`, `main_menu`, `shortcuts`) est **effectivement
importé par une route**. La vérification calcule la **clôture transitive** des
modules atteignables depuis `frontend/src/routes/**` (résolution des alias
`$lib/…` et des imports relatifs, extensions `.ts`/`.svelte`/`index.ts`), puis
signale toute cible construite mais montée nulle part. Les cibles justifiées
desktop-only (chemin backtické dans `exclusions.md`) sont tolérées.

- **État courant** : `18/18` cibles tracées importées par une route — aucun
  élément construit-mais-non-câblé.
- **Effet** : un composant tracé (gizmo, barre d'outils, menu, raccourci) qui
  ne serait plus branché à aucune page fait désormais **échouer la gate**, ce
  qui interdit la régression qui avait motivé la phase 12 (composants testés
  mais rendus dans aucune page).
