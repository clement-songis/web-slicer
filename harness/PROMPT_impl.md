Tu travailles seul, sans humain, sur web-slicer : un slicer web à parité totale avec OrcaSlicer (`vendor/OrcaSlicer`, lecture seule). Cette session traite UN lot de fonctionnalités, décrit en bas. Le harness juge le résultat avec des tests oracle comparés au vrai Orca.

Règles :
1. Contexte minimal : lis `CLAUDE.md` et `harness/CONTRACT.md`, puis seulement ce qu'il faut. Ne lis pas `specs/` ni de gros fichiers entiers : `rg -n`, puis lecture ciblée (offset/limit). Va droit aux sources Orca indiquées.
2. Réutilise la logique d'Orca (libslic3r côté moteur/serveur) au lieu de la réécrire en TypeScript. Le front affiche, édite et envoie des commandes.
3. Vérifie avec `python3 harness/verify.py --check <ids>` (démarre la pile de test, lance les tests du lot, affiche un diff compact). Lis le test concerné dans `tests/oracle/` pour savoir exactement ce qui est attendu.
4. `tests/oracle/`, `harness/`, `audit/` et `.claude/settings.json` sont gelés et contrôlés : toute modification est annulée et compte comme un échec. Si un test te paraît faux, ne le contourne pas : écris `TEST_SUSPECT: <raison précise>` dans ta note finale.
5. Interdit : détecter l'environnement de test, coder en dur des valeurs attendues, court-circuiter le vrai chemin UI → moteur.
6. Commits atomiques en anglais (`feat(scope): …`) avec les trailers habituels, dès qu'une partie du lot passe. Ne casse pas ce qui marche : un échantillon de features validées est rejoué après toi, une régression annule ta session.
7. Si tu bloques deux fois sur la même cause, arrête-toi.
8. Termine par une note de 5 lignes max qui commence par `NOTE:` : ce qui est fait, ce qui reste, la cause d'un échec éventuel et la piste suivante. Elle sera donnée à la prochaine session.
