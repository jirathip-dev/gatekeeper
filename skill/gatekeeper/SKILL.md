---
name: gatekeeper
description: Read, fix and rerun a failing gatekeeper gate (gatekeeper 1.0.0).
---

# gatekeeper 1.0.0

`./.gatekeeper/run check` is what CI runs and what a lane runs. A failing gate prints `FAIL  <pack>/<gate>`, its log's tail, its `fix:` line, its log path and the command that reruns it alone.

1. Read the gate's entry in its pack file below: what it checks and how to fix it.
2. Fix the cause, never the gate: no pack file is copied into the repo or edited there.
3. Rerun the one gate: `./.gatekeeper/run check --gate <pack>/<gate>`, then `./.gatekeeper/run check`.

`./.gatekeeper/run list` prints the planned gates with their budgets and why one is skipped. A repo's own gates live in `.gatekeeper/gates/<gate>.toml` as `local/<gate>` and print their `fix` the same way.

With `[cache]` in `.gatekeeper/config.toml` a gate whose inputs already passed prints `cached` with the run that proved it. A rerun of a cached pass that fails now prints `cached pass does not hold`: the gate reads a file it does not declare; add it to the gate's `paths` or set `cache = false`. `--no-cache` runs and records nothing.

## Packs

- [core](packs/core.md): the line cap, the rules file, arch-check, actionlint, the dependency list
- [rust](packs/rust.md): fmt, clippy with the shared lints, cargo-deny, the banned-API fixtures
- [ts](packs/ts.md): tsc strict, oxlint with the bans and the anti-slop rules, knip's unused-code ratchet
- [supabase](packs/supabase.md): the migration files' names, order and policy rule, and pgTAP on a local stack (CI only on a lane's host)
