# Carte de fidélité — état réel de l'UI (audit honnête)

> But : remplacer le « 139/139 tâches ✓, gate vert » par l'état **comportemental**
> réel, feature par feature. Le gate `check_traceability.py` mesure la
> **présence** (le slot existe et est importé par une route), pas la **fidélité**
> (ça se comporte comme Orca). Ce document mesure la seconde.

## Légende (4 colonnes)

| Col | Sens |
|-----|------|
| **E** — Existe | le composant / l'entrée de menu existe dans le code |
| **C** — Câblé | un clic déclenche un handler (≠ no-op muet) |
| **B** — Se comporte | fait réellement l'action, façon Orca, avec effet observable (et, si pertinent, **envoyé au tranchage**) |
| **V** — Conforme écran | conforme visuellement au screenshot Orca |

Statuts : ✅ oui · ⚠️ partiel · ❌ non · ❔ **non audité** (nécessite comparaison côte à côte — les screenshots Orca ne sont pas dans le dépôt).

## Score réel (vs le « 100 % » affiché)

| Groupe | Total | E (présent) | B (se comporte) | Taux comportemental |
|--------|------:|------------:|----------------:|--------------------:|
| Gizmos | 16 | 16 | 5 (+2 partiels) | ~31 % |
| Barre d'outils plateau | 11 | 11 | 6 (+1 partiel) | ~55 % |
| Menu principal | 55 | 55 | ~24 | ~44 % |
| Menu contextuel objet | 54 | 54 | ~13 | ~24 % |
| **UI globale** | **136** | **136 (100 %)** | **~48** | **~35 %** |
| **Conformité visuelle (V)** | 136 | — | — | **0 % audité** |

**Couverture ≈ 100 %. Comportement ≈ 35 %. Fidélité visuelle : jamais mesurée.**
C'est tout l'écart entre ce que le gate certifiait et ce que tu constates.

---

## Gizmos (16)

| Gizmo | E | C | B | V | Note |
|-------|---|---|---|---|------|
| Move | ✅ | ✅ | ✅ | ❔ | transform réel |
| Rotate | ✅ | ✅ | ✅ | ❔ | |
| Scale | ✅ | ✅ | ✅ | ❔ | |
| Flatten | ✅ | ✅ | ✅ | ❔ | `layFlatRotation` |
| Measure | ✅ | ✅ | ⚠️ | ❔ | mesure locale ; à valider vs Orca |
| BrimEars | ✅ | ✅ | ❌ | ❔ | on pose des points **mais** `save()` remet `brimEars` à vide → **non persisté, non slicé** |
| Assembly | ✅ | ✅ | ⚠️ | ❔ | vue seulement |
| Cut | ✅ | ✅ | ❌ | ❔ | `toolNeedsEngine('Découpe')` |
| MeshBoolean | ✅ | ✅ | ❌ | ❔ | `toolNeedsEngine('Booléen')` |
| Emboss | ✅ | ✅ | ❌ | ❔ | « nécessite le moteur (à venir) » |
| GLGizmoSVG | ✅ | ✅ | ❌ | ❔ | `toolNeedsEngine('Forme SVG')` |
| Simplify | ✅ | ✅ | ❌ | ❔ | `toolNeedsEngine('Simplification')` |
| FdmSupports (peinture) | ✅ | ✅ | ❌ | ❔ | peint à l'écran **mais** ni persisté ni envoyé au moteur → aucun effet sur le print |
| Seam (peinture) | ✅ | ✅ | ❌ | ❔ | idem |
| FuzzySkin (peinture) | ✅ | ✅ | ❌ | ❔ | idem |
| MmSegmentation (peinture) | ✅ | ✅ | ❌ | ❔ | idem |

**Réellement fidèles : 5/16.** Les 4 gizmos de peinture sont le piège le plus trompeur : ils *paraissent* marcher (on peint) mais rien n'atteint le tranchage.

---

## Barre d'outils du plateau (11)

| Item | B | Note |
|------|---|------|
| add (menu Load + primitives) | ✅ | ouvre le menu d'ajout |
| addplate | ✅ | ajoute un plateau |
| arrange | ✅ | arrangement moteur réel |
| orient | ✅ | orientation auto moteur |
| more / fewer | ✅ | duplique / retire l'instance |
| assembly_view | ⚠️ | bascule la vue |
| layersediting | ⚠️ | ouvre le panneau de hauteur variable, **mais le profil n'est pas envoyé au tranchage** |
| splitobjects | ❌ | `toolNeedsEngine` |
| splitvolumes | ❌ | `toolNeedsEngine` |

---

## Menu principal (55) — l'essentiel des manques

Câblés et réels (~24) : Nouveau / Ouvrir / Enregistrer / Enregistrer sous / Importer géométrie / Export 3MF générique / Recharger ; Copier / Couper / Coller / Supprimer (sélection & tout) / Cloner / Tout sélectionner / Désélectionner ; 7 vues caméra + grille ; Raccourcis.

**Stubs (`default → menuTodo`, ~31)** — non disponibles en v1 :

- **Undo / Redo** ❌ (absence majeure pour un slicer)
- **Export G-code…**, Export plate sliced file, Export all objects STL/DRC, Export toolpaths OBJ, **Export Preset Bundle** ❌
- **Import Zip Archive**, **Import Configs** ❌
- **Préférences** ❌, **Preset Bundle** ❌
- **Open G-code…**, **Reload from Disk (F5)** ❌
- Use Perspective / Orthogonal View, Auto Perspective ❌
- Show G-code Window, Show Labels, Show Overhang, Show Selected Outline ❌
- « Duplicate Current Plate » ⚠️ ajoute un plateau **vide** (« contenu à venir »)

---

## Menu contextuel objet (54) — ⚠️ pas de `default`

`onObjectAction` n'a **aucun cas par défaut** : les entrées non gérées sont des
**no-ops silencieux** (pire qu'un message).

Réels (~13) : Masquer/Afficher, Éditer, Cloner / Add instance, Remove instance / Supprimer, Auto-orientation, Center, Drop, + primitives (Cube/Cylinder/Sphere/Cone/Disc/Torus).

Stubs explicites : Rename (`menuTodo`), Fix model, To objects / To parts / Split (`toolNeedsEngine`), Simplify (ouvre un outil lui-même stub).

**Muets / non implémentés (majorité des 54)** : Fill bed with copies, Set as an individual object, Height range Modifier, Export as STL/DRC (×4), Reload from disk, Replace 3D file / all, Scale to build volume, Flush into infill/object/support, Printable, Fix model, Set number of instances, Fill bed with instances, Subdivision mesh, Assemble, Mesh boolean, Along X/Y/Z axis, Add Models, Show Labels, Delete Plate / All, Reload All, Auto Rotate…

---

## Défauts transverses (les plus trompeurs)

1. **La peinture ne va nulle part.** Supports/Seam/FuzzySkin/MmSegmentation + BrimEars : UI présente, mais `save()` sérialise un `painting`/`brimEars` **vide**, et le runner de tranchage (`build_stl_model`) n'envoie que la géométrie brute. Zéro effet sur le G-code.
2. **Hauteur de couche variable** : panneau présent, profil **non transmis** au moteur.
3. **Undo/Redo inexistants.**
4. **Exports** : seul l'export 3MF générique marche ; G-code / STL / DRC / bundle de presets = stubs.
5. **Fidélité visuelle jamais mesurée** : le layout vient de `ui_inventory.json` (le *set* d'options est juste), mais la composition/interaction n'a pas été reproduite depuis les vrais panneaux Orca ni comparée aux screenshots.

## Pourquoi le gate ne l'a pas vu

`check_wired` (garde « construit-mais-non-câblé ») prouve seulement qu'un
composant est **importé par une route**. Un handler qui appelle `menuTodo()` ou
`toolNeedsEngine()`, ou un `case` manquant qui tombe en no-op, satisfont le gate.
Il n'existe **aucun** contrôle « pas de stub dans un handler censé être câblé »,
ni aucun test qui **conduit l'app** et observe l'effet. D'où : vert ≠ fonctionne.
