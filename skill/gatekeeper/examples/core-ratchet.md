# core/ratchet: what a failure looks like

Each case is one change on the pack's fixture repo. `gatekeeper selftest` runs every case and fails unless the gate fails on it printing the line shown, so these examples cannot go stale.

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
version = "1.5.1"
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
