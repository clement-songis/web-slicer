---
name: orca-cli-segfaults-headless
description: orca-slicer CLI segfaults slicing in this Nix devShell; FFI path works — affects CLI fallback (T020) and parity oracle (T022)
metadata:
  type: project
---

In the Nix devShell, `orca-slicer --slice 0 --outputdir <dir> <project>.3mf`
**segfaults** (SIGSEGV) even on `engine/tests/fixtures/orca_project.3mf`,
whereas the FFI worker path (`engine-worker slice`) slices the same project
successfully. Likely a GUI/display dependency in the CLI binary under headless.

**Why:** validates the R1 revision (FFI primary, CLI fallback) — the FFI
bridge is the reliable slicing path here.

**How to apply:** T020 (CLI adapter `adapters/cli/`) cannot be validated by
the generic trait suite via real slicing in this environment, and T022
(triple diff FFI vs CLI vs desktop) loses its CLI + desktop oracles. Before
building T020/T022, decide with the user: (a) mark CLI slicing degraded in
[[exclusions]]-style note, (b) find a headless-capable orca invocation, or
(c) treat FFI as the sole validated engine for P1 and defer CLI parity.

Confirmed 2026-07-10 during P1 implementation (T017–T021 + FfiEngine done).
