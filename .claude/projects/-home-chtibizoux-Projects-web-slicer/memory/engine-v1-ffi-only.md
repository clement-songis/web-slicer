---
name: engine-v1-ffi-only
description: v1 engine is FFI-only (no CLI adapter/fallback); CLI cross-validation is deferred backlog
metadata:
  type: project
---

The v1 `SlicerEngine` implementation goes through `adapters/ffi` (cxx bridge
to libslic3r-headless) **only**. The earlier "FFI primary + CLI fallback"
framing was my misread — the user meant "CLI *or* FFI", chose FFI, and asked
to drop CLI implementation tasks (T020, and the CLI legs of T022/T023) from
the specs.

**Why:** FFI already works end-to-end; a second CLI adapter isn't wanted for
v1. A CLI-based *cross-validation* harness (confirm slicing matches orca
desktop exactly) remains a desirable future/backlog item — not planned now,
and the [[orca-cli-segfaults-headless]] blocker is explicitly not to be
investigated for now.

**How to apply:** don't build `adapters/cli`. Trait-suite conformance and
parity are proven against FFI. See CLAUDE.md § "Conventions de code".
