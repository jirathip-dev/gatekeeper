# core/rules: what a failure looks like

Each case is one change on the pack's fixture repo. `gatekeeper selftest` runs every case and fails unless the gate fails on it printing the line shown, so these examples cannot go stale.

## active-check-fails

Writes `.drover/rules.md`:

```
- don't add a line to list.txt (enforced by: CI arch-check)
- don't leave a TODO file (enforced by: no-todo `test ! -e TODO`, active since 2026-11-20)
```

Writes `TODO`:

```
later
```

The gate fails and its log prints `FAIL no-todo`.

Fix: fix what the named check reports, or retire the check in a lesson PR. Rerun: `./.gatekeeper/run check --gate core/rules`
