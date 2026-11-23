# core/cap: what a failure looks like

Each case is one change on the pack's fixture repo. `gatekeeper selftest` runs every case and fails unless the gate fails on it printing the line shown, so these examples cannot go stale.

## cap-missing

Removes `ci/cap.toml`.

The gate fails and its log prints `No such file or directory`.

Fix: delete code; a raise of the cap is its own issue, its commit carries `Cap: <old> → <new> (#issue, q<N>)`. Rerun: `./.gatekeeper/run check --gate core/cap`

## counts-the-globs

Writes `ci/cap.toml`:

```
[fleet.fixture]
line_cap = 11
count = ["web/src/**/*.ts"]
exclude = ["**/*.test.ts", "web/src/gen/**"]
```

Commits with the message:

```
cap: count the TypeScript

Cap: 100 → 11 (#5, q7)
```

Writes `web/src/a.test.ts` (not shown).

Writes `web/src/a.ts`:

```
x
x
x
x
x
x
x
```

Writes `web/src/deep/b.ts`:

```
x
x
x
x
x
```

Writes `web/src/gen/t.ts` (not shown).

The gate fails and its log prints `src lines (formatted): 12 / cap 11`.

Fix: delete code; a raise of the cap is its own issue, its commit carries `Cap: <old> → <new> (#issue, q<N>)`. Rerun: `./.gatekeeper/run check --gate core/cap`

## over-cap

Writes `ci/cap.toml`:

```
[fleet.fixture]
line_cap = 2
```

The gate fails and its log prints `/ cap 2`.

Fix: delete code; a raise of the cap is its own issue, its commit carries `Cap: <old> → <new> (#issue, q<N>)`. Rerun: `./.gatekeeper/run check --gate core/cap`

## raise-in-cap-d

Writes `ci/cap.d/2026-11-13-up.toml`:

```
line_cap = 600
```

Writes `ci/cap.d/2026-11-14-up.toml`:

```
line_cap = 1000
why = "room"
```

Writes `ci/cap.d/notes.md`:

```
line_cap = 1
```

Writes `ci/cap.toml`:

```
[fleet.fixture]
line_cap = 2
```

Commits with the message:

```
cap: raise to 1000

Cap: 2 → 1000 (#5, q7)
```

The gate passes.

## skips-blank-lines-and-the-test-tail

Writes `ci/cap.toml`:

```
line_cap = 5
count = ["crates/a/src/**"]
skip_blank = true
stop_at_cfg_test = true
```

Writes `crates/a/src/lib.rs`:

```
fn a() {}

   
fn b() {}
  #[cfg(test)]
mod tests {}
```

Writes `crates/a/src/m.swift`:

```
a

b
#[cfg(test)]
c
```

Commits with the message:

```
cap: count crate a only

Cap: 100 → 5 (#5, q7)
```

The gate fails and its log prints `src lines (formatted): 6 / cap 5`.

Fix: delete code; a raise of the cap is its own issue, its commit carries `Cap: <old> → <new> (#issue, q<N>)`. Rerun: `./.gatekeeper/run check --gate core/cap`

## trailer-given

Writes `ci/cap.toml`:

```
[fleet.fixture]
line_cap = 200
```

Commits with the message:

```
cap: raise to 200

Cap: 100 → 200 (#5, q7)
```

The gate passes.

## trailer-missing

Writes `ci/cap.toml`:

```
[fleet.fixture]
line_cap = 200
```

Commits with the message:

```
cap: raise to 200
```

The gate fails and its log prints `edits the line cap without a `Cap:`.

Fix: delete code; a raise of the cap is its own issue, its commit carries `Cap: <old> → <new> (#issue, q<N>)`. Rerun: `./.gatekeeper/run check --gate core/cap`
