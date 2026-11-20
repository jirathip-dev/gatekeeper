---
name: gatekeeper
description: Read, fix and rerun a failing gatekeeper gate (gatekeeper 0.1.0).
---

# gatekeeper 0.1.0

`./.gatekeeper/run check` is what CI runs and what a lane runs. A failing gate prints `FAIL  <pack>/<gate>`, its log's tail, its `fix:` line, its log path and the command that reruns it alone.

1. Read the gate's entry in its pack file below: what it checks and how to fix it.
2. Fix the cause, never the gate: no pack file is copied into the repo or edited there.
3. Rerun the one gate: `./.gatekeeper/run check --gate <pack>/<gate>`, then `./.gatekeeper/run check`.

`./.gatekeeper/run list` prints the planned gates with their budgets and why one is skipped. A repo's own gates live in `.gatekeeper/gates/<gate>.toml` as `local/<gate>` and print their `fix` the same way.

## Packs

- [core](packs/core.md): the line cap, the rules file, arch-check, actionlint
- [rust](packs/rust.md): fmt, clippy with the shared lints, cargo-deny, the banned-API fixtures
