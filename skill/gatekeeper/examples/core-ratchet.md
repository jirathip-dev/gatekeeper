# core/ratchet: what a failure looks like

Each case is one change on the pack's fixture repo. `gatekeeper selftest` runs every case and fails unless the gate fails on it printing the line shown, so these examples cannot go stale.

## ceiling-deleted

Commits with the message:

```
duplication: drop the ceiling
```

Removes `ci/ratchet/duplication.d/2026-11-26-0.toml`.

The gate fails and its log prints `ci/ratchet/duplication.d/2026-11-26-0.toml: a ceiling file is never edited or deleted`.

Fix: fix the new finding instead of listing it; a raise is the owner's decision, its commit carries `Ratchet: <pack/gate> <old> → <new> (#issue, q<N>)`. Rerun: `./.gatekeeper/run check --gate core/ratchet`

## ceiling-edited

Writes `ci/ratchet/duplication.d/2026-11-26-0.toml`:

```
ceiling = 10
```

Commits with the message:

```
duplication: raise the ceiling by hand
```

The gate fails and its log prints `ci/ratchet/duplication.d/2026-11-26-0.toml: a ceiling file is never edited or deleted`.

Fix: fix the new finding instead of listing it; a raise is the owner's decision, its commit carries `Ratchet: <pack/gate> <old> → <new> (#issue, q<N>)`. Rerun: `./.gatekeeper/run check --gate core/ratchet`

## list-grew

Writes `.gatekeeper/baseline/rust-dead-code.txt`:

```
crates/core/src/hosts.rs: pub fn held_slots: no other crate names it
```

Commits with the message:

```
dead code: list a new finding
```

The gate fails and its log prints `.gatekeeper/baseline/rust-dead-code.txt: 1 line added; a list only shrinks`.

Fix: fix the new finding instead of listing it; a raise is the owner's decision, its commit carries `Ratchet: <pack/gate> <old> → <new> (#issue, q<N>)`. Rerun: `./.gatekeeper/run check --gate core/ratchet`

## list-grew-with-pin-moved

Writes `.gatekeeper/baseline/rust-dead-code.txt`:

```
crates/core/src/hosts.rs: pub fn held_slots: no other crate names it
```

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"

[pin]
version = "1.6.0"
aarch64-apple-darwin = "0000000000000000000000000000000000000000000000000000000000000000"
x86_64-unknown-linux-gnu = "0000000000000000000000000000000000000000000000000000000000000000"
```

Commits with the message:

```
pin: bump gatekeeper
```

The gate passes.

## list-grew-with-trailer

Writes `.gatekeeper/baseline/rust-dead-code.txt`:

```
crates/core/src/hosts.rs: pub fn held_slots: no other crate names it
```

Commits with the message:

```
dead code: list a new finding

Ratchet: rust/dead-code 0 → 1 (#5, q7)
```

The gate passes.

## local-gate-list-grew

Writes `.gatekeeper/baseline/local-floor.txt`:

```
deny.toml: bans.wildcards is "allow"
```

Commits with the message:

```
floor: list a finding
```

The gate passes.

## new-file-not-lower

Writes `ci/ratchet/duplication.d/2026-11-27-10.toml`:

```
ceiling = 10
```

Commits with the message:

```
duplication: a higher ceiling beside the old one
```

The gate fails and its log prints `ci/ratchet/duplication.d/2026-11-27-10.toml: ceiling 10 is not under the base's ceiling 0: a ceiling only falls`.

Fix: fix the new finding instead of listing it; a raise is the owner's decision, its commit carries `Ratchet: <pack/gate> <old> → <new> (#issue, q<N>)`. Rerun: `./.gatekeeper/run check --gate core/ratchet`

## raised-with-trailer

Writes `ci/ratchet/duplication.d/2026-11-27-10.toml`:

```
ceiling = 10
```

Commits with the message:

```
duplication: the owner raised the ceiling

Ratchet: duplication 0 → 10 (#5, q7)
```

Removes `ci/ratchet/duplication.d/2026-11-26-0.toml`.

The gate passes.

## surface-raised-with-trailer

Writes `ci/ratchet/surface-verb.d/2026-11-27-3.toml`:

```
ceiling = 3
```

Commits with the message:

```
surface: the owner approved a new verb

Ratchet: surface-verb 2 → 3 (#5, q7)
```

Removes `ci/ratchet/surface-verb.d/2026-11-26-2.toml`.

The gate passes.

## surface-raised-without-trailer

Writes `ci/ratchet/surface-verb.d/2026-11-27-3.toml`:

```
ceiling = 3
```

Commits with the message:

```
surface: a new verb

Ratchet: duplication 2 → 3 (#5, q7)
```

Removes `ci/ratchet/surface-verb.d/2026-11-26-2.toml`.

The gate fails and its log prints `ci/ratchet/surface-verb.d/2026-11-26-2.toml: a ceiling file is never edited or deleted; lower it with a new file (a raise carries `Ratchet: surface-verb <old> → <new> (#issue, q<N>)`)`.

Fix: fix the new finding instead of listing it; a raise is the owner's decision, its commit carries `Ratchet: <pack/gate> <old> → <new> (#issue, q<N>)`. Rerun: `./.gatekeeper/run check --gate core/ratchet`

## trailer-names-another-count

Writes `.gatekeeper/baseline/rust-dead-code.txt`:

```
crates/core/src/hosts.rs: pub fn held_slots: no other crate names it
```

Commits with the message:

```
dead code: list a new finding

Ratchet: rust/complexity 0 → 1 (#5, q7)
```

The gate fails and its log prints `.gatekeeper/baseline/rust-dead-code.txt: 1 line added; a list only shrinks`.

Fix: fix the new finding instead of listing it; a raise is the owner's decision, its commit carries `Ratchet: <pack/gate> <old> → <new> (#issue, q<N>)`. Rerun: `./.gatekeeper/run check --gate core/ratchet`
