# core/cap: what a failure looks like

Each case is one change on the pack's fixture repo. `gatekeeper selftest` runs every case and fails unless the gate fails on it printing the line shown, so these examples cannot go stale.

## cap-missing

Removes `ci/cap.toml`.

The gate fails and its log prints `No such file or directory`.

Fix: delete code, or the rules a check now enforces; a raise of the cap is its own issue. Rerun: `./.gatekeeper/run check --gate core/cap`

## over-cap

Writes `ci/cap.toml`:

```
[fleet.fixture]
line_cap = 2
```

The gate fails and its log prints `/ cap 2`.

Fix: delete code, or the rules a check now enforces; a raise of the cap is its own issue. Rerun: `./.gatekeeper/run check --gate core/cap`

## rules-31-lines

Writes `.drover/rules.md` (not shown).

The gate fails and its log prints `over 30`.

Fix: delete code, or the rules a check now enforces; a raise of the cap is its own issue. Rerun: `./.gatekeeper/run check --gate core/cap`
