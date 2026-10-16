# Contrat de testabilité (gelé)

Les tests oracle (`tests/oracle/`) ne connaissent l'application **que** par ce
contrat. L'implémentation est libre derrière ; le contrat ne change que par
décision humaine.

## §1 API de harness (active seulement si `HARNESS_API=1`)

### `POST /api/_harness/slice`
- multipart : `model` (fichier STL/3MF/OBJ/STEP), `config` (JSON).
- `config` = **config Orca plate et complète** (impression + filament +
  imprimante fusionnés), au format des profils Orca aplatis : valeurs en
  chaînes, vecteurs en tableaux de chaînes. C'est exactement ce que produit
  `tests/oracle/tools/flatten_profile.py`. Clé absente = valeur par défaut
  d'Orca (`FullPrintConfig::defaults()`), comme le fait le CLI.
- Le modèle est placé comme le fait `orca-slicer --arrange 1 --orient 0` (un
  seul plateau).
- **Doit passer par le même moteur que l'UI** (le test `core.export-gcode`
  compare les deux sorties).
- 200 : corps = G-code (`text/plain`). 422 : `{"error": "..."}` si Orca
  refuse la config (même validation que `Print::validate`).

### `POST /api/_harness/bootstrap`
- JSON `{ "email", "password", "printer", "filament", "process" }` (noms de
  presets système Orca).
- Crée l'utilisateur s'il n'existe pas, ouvre la session (cookie), le rend
  propriétaire de l'imprimante, marque l'assistant de démarrage comme fait,
  crée un projet vide avec ces trois presets actifs.
- 200 : `{ "project_id": "..." }`.

## §2 Sélecteurs UI (`data-testid` sauf mention)

| Zone | Sélecteurs |
|---|---|
| Auth (`/register`, `/login`) | `auth-email`, `auth-password`, `auth-submit` ; une fois connecté : `user-menu` |
| Bibliothèque | `new-project` → navigue vers `/projects/<id>` |
| Projet | `scene-canvas` ; `import-input` (`<input type=file>`, peut être caché) ; lignes `object-row` (texte = nom de l'objet) |
| Réglages (presets) | boutons `open-settings-print`, `open-settings-filament`, `open-settings-printer` ; sélecteurs de preset `preset-select-print/filament/printer` avec l'attribut `data-modified="true"` dès qu'une valeur diffère du preset enregistré (astérisque d'Orca ; le preset système n'est jamais écrasé) ; dialogue `settings-dialog` ; onglets `settings-tab-<slug du titre Orca>` (slug : minuscules, non-alphanumériques → `-` ; la page extrudeur dynamique s'appelle `Extruder 1`) ; bouton `settings-apply` |
| Modes | `settings-mode-simple`, `settings-mode-advanced`, `settings-mode-expert`, `settings-mode-develop` dans `settings-dialog`. Une option de niveau Orca N (`mode` dans `audit/parameters.json`) est **visible à partir du mode N et absente du DOM ou masquée en dessous**, comme dans Orca |
| Réglages du plateau | `plate-settings-button` (un par plateau) → `settings-dialog` ne contenant que les options propres au plateau (`TabPrintPlate` : type de plateau, séquence d'impression…) |
| Réglages de l'objet | sélection d'une `object-row` puis `object-settings-button` → `settings-dialog` limité à l'objet (surcharges par objet, `TabPrintModel`) |
| Option de réglage | conteneur avec l'attribut **`data-param="<clé Orca>"`** contenant un seul contrôle : `input[type=checkbox]`, `select` (valeurs = `enum_values` Orca) ou `input`/`textarea` texte. Vecteurs : contrôle du 1er élément porte `data-index="0"`. Pourcentages : saisie `60%` acceptée. **Pré-rempli** avec la valeur du preset actif. **Grisé (`disabled`)** quand Orca le grise (dépendances de `ConfigManipulation.cpp`). Options « nullable » (surcharges filament) : case `input[type=checkbox][data-override-toggle]` à cocher pour activer la surcharge, comme dans Orca |
| Tranchage | `slice-button` ; `slice-progress` (pendant) ; `preview-view` avec l'attribut `data-layer-count="<n>"` une fois fini |
| Aperçu | `layer-slider` (`input[type=range]`, attribut `data-current-layer`) ; `preview-legend` avec des `legend-item` portant `data-role="<rôle d'extrusion Orca>"` |
| Export | `export-gcode-button` (déclenche un téléchargement `.gcode`) |
| Projet | `save-project` ; après rechargement, objets et réglages modifiés sont restaurés |
| Imprimantes (`/printers`) | `printer-add`, `printer-moonraker-url`, `printer-save`, lignes `printer-row` |
| Envoi | `send-to-printer` → `send-printer-select` (`select`), `send-confirm` ; succès : `send-success` |

Les scénarios écrits par l'auteur de tests (gizmos, menus, raccourcis…)
peuvent **ajouter** des sélecteurs : ils les déclarent en tête de leur fichier
de test (commentaire `// CONTRAT: ...`), l'implémenteur les ajoute.
