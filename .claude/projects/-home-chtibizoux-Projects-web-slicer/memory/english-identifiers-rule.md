---
name: english-identifiers-rule
description: All code identifiers (functions, variables, types, tests) must be English; French comments are fine
metadata:
  type: feedback
---

The user requires all identifiers — function names, variable names, types,
modules, test names — in English, across Rust/TS/C++. French comments and
docstrings are acceptable (that's the repo's established style).

**Why:** the user reviewed the code and objected to French function names
(e.g. `chaine_bbl_reelle`, `progression_monotone_et_resultat`). Now codified
in CLAUDE.md § "Conventions de code".

**How to apply:** name every new identifier in English. When touching a file
with legacy French identifiers, rename them to English opportunistically.
Keep French for comments/docstrings to match surrounding code.
