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

Fix: fix what the named check reports, delete the rules a check now enforces, or retire the check in a lesson PR. Rerun: `./.gatekeeper/run check --gate core/rules`

## empty-enforcer

Writes `.drover/rules.md`:

```
# Rules
- don't X (enforced by: none)
- don't Y (enforced by: )
```

The gate fails and its log prints `line 3 names no enforcer`.

Fix: fix what the named check reports, delete the rules a check now enforces, or retire the check in a lesson PR. Rerun: `./.gatekeeper/run check --gate core/rules`

## rule-without-enforcer

Writes `.drover/rules.md`:

```
# Rules
- don't X (enforced by: none)
- don't Y
```

The gate fails and its log prints `line 3 names no enforcer`.

Fix: fix what the named check reports, delete the rules a check now enforces, or retire the check in a lesson PR. Rerun: `./.gatekeeper/run check --gate core/rules`

## rules-31-lines

Writes `.drover/rules.md` (not shown).

The gate fails and its log prints `31 lines, over 30`.

Fix: fix what the named check reports, delete the rules a check now enforces, or retire the check in a lesson PR. Rerun: `./.gatekeeper/run check --gate core/rules`

## shadow-check-fails

Writes `.drover/rules.md`:

```
- don't add a line to list.txt (enforced by: CI arch-check)
- don't leave a TODO file (enforced by: no-todo `test ! -e TODO`, shadow since 2026-11-20)
- don't drop the list (enforced by: has-list `test -e list.txt`, active since 2026-11-21)
```

Writes `TODO`:

```
later
```

The check writes `checks.json`:

```json
{
  "file": ".drover/rules.md",
  "checks": [
    {
      "line": 2,
      "name": "no-todo",
      "cmd": "test ! -e TODO",
      "state": "shadow",
      "since": "2026-11-20",
      "passed": false
    },
    {
      "line": 3,
      "name": "has-list",
      "cmd": "test -e list.txt",
      "state": "active",
      "since": "2026-11-21",
      "passed": true
    }
  ]
}
```

The gate passes.
