# Harness autonome — parité OrcaSlicer

Claude avance seul, par sessions courtes. Chaque session traite **un lot** et
c'est `verify.py`, pas Claude, qui décide si c'est fait, en comparant l'app
au vrai OrcaSlicer.

```
gen_features.py ──► features.json (~1 000 features : core, golden, 600+ paramètres, gizmos, menus, raccourcis)
make_refs.py    ──► refs/ : G-code de référence produits par orca-slicer CLI
loop.sh ─┬─ next_feature.py ─► lot + contexte court (sources Orca, dernière erreur, note précédente)
         ├─ claude -p (sonnet pour implémenter / opus pour écrire les tests / opus pour relire)
         │     hooks : guard.py (tests gelés), stop_gate.py (re-tente une fois si rouge)
         ├─ verify.py ─► tests du lot + échantillon de non-régression ─► statut
         └─ loopctl.py ─► journal coût/tokens, note pour la session suivante
```

## Mise en place (une fois)

```sh
nix develop
harness/setup.sh          # sous-module Orca, modèles, features, Playwright, références Orca (~1 h de CPU), gel
```

## Lancer

```sh
HARNESS_MAX_USD=20 harness/loop.sh                 # s'arrête au budget ou quand plus rien n'est exécutable
HARNESS_MAX_USD=40 HARNESS_NIGHTLY=1 harness/loop.sh   # la nuit, avec régression complète à la fin
python3 harness/status.py                          # avancement, coût par feature, files d'attente humaines
```

Pour plus d'autonomie et de sûreté, lance la boucle dans un conteneur et passe
`HARNESS_CLAUDE_FLAGS=--dangerously-skip-permissions` : l'état autoritaire
(`$HARNESS_STATE`) n'y est alors pas monté et l'agent ne peut pas l'atteindre.

## Cycle de vie d'une feature

| Statut | Signification | Qui agit |
|---|---|---|
| `needs_ref` | attend la référence Orca | `make_refs.py` |
| `needs_test` | scénario à écrire (gizmo, menu, raccourci…) | session « tests » (opus) |
| `red` | test gelé, échoue | session « impl » (sonnet) |
| `passing` | validé par `verify.py` | — (rejoué en échantillon après chaque session) |
| `blocked` | 3 échecs | session « review » (opus) toutes les 8 sessions, puis toi |
| `needs_review` | valeur de test refusée par Orca, test jugé faux, champ désactivé… | toi |
| `excluded` | sans objet en web (`exclusions.json`, décision humaine) | — |

## Ordre

1. **core** : parcours de bout en bout (API de harness, auth, import, réglages,
   tranchage, export, aperçu, sauvegarde, Moonraker). Tout le reste en dépend.
2. **golden** : corpus 6 modèles × 3 process, G-code identique à Orca.
3. **param** : chaque paramètre est testé deux fois. Côté moteur, le G-code
   doit être identique à Orca pour la même valeur. Côté UI, la valeur saisie
   doit arriver dans le G-code exactement comme chez Orca. Ordre : simple →
   avancé → expert. Lots de 6 par section d'onglet.
4. **toolbar / gizmo / ctx / menu / shortcut** : une session opus écrit d'abord
   le scénario (gelé), puis sonnet l'implémente.

## Garde-fous contre la triche

- `guard.py` (PreToolUse) refuse toute écriture dans `tests/oracle/`,
  `harness/`, `audit/` et `.claude/settings.json`, y compris via Bash.
- `verify.py` contrôle les empreintes des fichiers gelés (manifeste hors
  dépôt). Toute altération est restaurée, la session annulée
  (`git reset --hard`) et la tentative comptée.
- Une régression sur l'échantillon de features validées annule aussi la session.
- Les exclusions et les valeurs de test manuelles (`param_overrides.json`) ne
  sont modifiables que par un humain.

## Ton rôle

- `status.py` : traiter **BLOQUÉES** et **À REVOIR** (souvent une ligne dans
  `param_overrides.json`, puis `gen_features.py && make_refs.py --keys <clé>`).
- Jeter un œil aux tests « verts dès leur création ».
- Après une modification volontaire du harness : `HARNESS_UNLOCK=1` pour
  la session Claude qui l'édite, puis `python3 harness/loopctl.py freeze`.
- Utiliser l'app de temps en temps : c'est le seul test que rien ne remplace.

## Coût

Chaque session repart d'un contexte vide : CLAUDE.md, le contrat et le lot,
soit environ 2 000 tokens, puis des lectures ciblées. Le coût, les tokens et
les tours de chaque session sont dans `logs/runs.jsonl`, et `status.py`
affiche le coût par feature validée. Leviers : `HARNESS_BATCH_PARAMS` (lots
plus gros, moins de démarrages de session), `HARNESS_MAX_TURNS`, et le
modèle utilisé par mode.
