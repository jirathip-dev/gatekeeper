# rust/bans: what a failure looks like

Each case is one change on the pack's fixture repo. `gatekeeper selftest` runs every case and fails unless the gate fails on it printing the line shown, so these examples cannot go stale.

## floor-allowed

Writes `Cargo.toml` (not shown).

The gate fails and its log prints `unwrap_used is "allow"`.

Fix: put the lint back at the floor's level; a crate that needs unsafe is named in [rust] unsafe_crates with its own [lints] denying clippy::undocumented_unsafe_blocks and a // SAFETY: line over each unsafe block; a floor that is wrong is changed in gatekeeper. Rerun: `./.gatekeeper/run check --gate rust/bans`

## floor-missing

Writes `Cargo.toml` (not shown).

The gate fails and its log prints `has no unwrap_used`.

Fix: put the lint back at the floor's level; a crate that needs unsafe is named in [rust] unsafe_crates with its own [lints] denying clippy::undocumented_unsafe_blocks and a // SAFETY: line over each unsafe block; a floor that is wrong is changed in gatekeeper. Rerun: `./.gatekeeper/run check --gate rust/bans`

## pedantic-off

Writes `Cargo.toml` (not shown).

The gate fails and its log prints `pedantic is "allow"`.

Fix: put the lint back at the floor's level; a crate that needs unsafe is named in [rust] unsafe_crates with its own [lints] denying clippy::undocumented_unsafe_blocks and a // SAFETY: line over each unsafe block; a floor that is wrong is changed in gatekeeper. Rerun: `./.gatekeeper/run check --gate rust/bans`

## unsafe-documented

Writes `.gatekeeper/config.toml`:

```
packs = ["rust"]
integration = "staging"
pin = "checkout"

[rust]
unsafe_crates = ["gpu"]
```

Writes `Cargo.toml` (not shown).

Writes `gpu/Cargo.toml` (not shown).

Writes `gpu/src/lib.rs`:

```
#[must_use]
pub fn first(v: &[u8]) -> Option<u8> {
    if v.is_empty() {
        return None;
    }
    // SAFETY: v is not empty, so index 0 is in bounds.
    Some(unsafe { *v.get_unchecked(0) })
}
```

The gate passes.

## unsafe-no-deny

Writes `.gatekeeper/config.toml`:

```
packs = ["rust"]
integration = "staging"
pin = "checkout"

[rust]
unsafe_crates = ["gpu"]
```

Writes `Cargo.toml` (not shown).

Writes `gpu/Cargo.toml` (not shown).

Writes `gpu/src/lib.rs`:

```
#[must_use]
pub fn first(v: &[u8]) -> Option<u8> {
    if v.is_empty() {
        return None;
    }
    // SAFETY: v is not empty, so index 0 is in bounds.
    Some(unsafe { *v.get_unchecked(0) })
}
```

The gate fails and its log prints `crate gpu: gpu/Cargo.toml: [lints.clippy] has no undocumented_unsafe_blocks`.

Fix: put the lint back at the floor's level; a crate that needs unsafe is named in [rust] unsafe_crates with its own [lints] denying clippy::undocumented_unsafe_blocks and a // SAFETY: line over each unsafe block; a floor that is wrong is changed in gatekeeper. Rerun: `./.gatekeeper/run check --gate rust/bans`

## unsafe-stray

Writes `.gatekeeper/config.toml`:

```
packs = ["rust"]
integration = "staging"
pin = "checkout"

[rust]
unsafe_crates = ["gpu", "ghost"]
```

Writes `Cargo.toml` (not shown).

Writes `gpu/Cargo.toml` (not shown).

Writes `gpu/src/lib.rs`:

```
#[must_use]
pub fn first(v: &[u8]) -> Option<u8> {
    if v.is_empty() {
        return None;
    }
    // SAFETY: v is not empty, so index 0 is in bounds.
    Some(unsafe { *v.get_unchecked(0) })
}
```

The gate fails and its log prints `.gatekeeper/config.toml: unsafe_crates: ghost is no workspace member`.

Fix: put the lint back at the floor's level; a crate that needs unsafe is named in [rust] unsafe_crates with its own [lints] denying clippy::undocumented_unsafe_blocks and a // SAFETY: line over each unsafe block; a floor that is wrong is changed in gatekeeper. Rerun: `./.gatekeeper/run check --gate rust/bans`

## unsafe-undocumented

Writes `.gatekeeper/config.toml`:

```
packs = ["rust"]
integration = "staging"
pin = "checkout"

[rust]
unsafe_crates = ["gpu"]
```

Writes `Cargo.toml` (not shown).

Writes `gpu/Cargo.toml` (not shown).

Writes `gpu/src/lib.rs`:

```
#[must_use]
pub fn first(v: &[u8]) -> Option<u8> {
    if v.is_empty() {
        return None;
    }
    Some(unsafe { *v.get_unchecked(0) })
}
```

The gate fails and its log prints `crate gpu: gpu/src/lib.rs:6: unsafe with no // SAFETY:`.

Fix: put the lint back at the floor's level; a crate that needs unsafe is named in [rust] unsafe_crates with its own [lints] denying clippy::undocumented_unsafe_blocks and a // SAFETY: line over each unsafe block; a floor that is wrong is changed in gatekeeper. Rerun: `./.gatekeeper/run check --gate rust/bans`

## unsafe-unnamed

Writes `.gatekeeper/config.toml`:

```
packs = ["rust"]
integration = "staging"
pin = "checkout"

[rust]
unsafe_crates = []
```

Writes `Cargo.toml` (not shown).

Writes `gpu/Cargo.toml` (not shown).

Writes `gpu/src/lib.rs`:

```
#[must_use]
pub fn first(v: &[u8]) -> Option<u8> {
    if v.is_empty() {
        return None;
    }
    // SAFETY: v is not empty, so index 0 is in bounds.
    Some(unsafe { *v.get_unchecked(0) })
}
```

The gate fails and its log prints `crate gpu: gpu/Cargo.toml: [lints.rust] unsafe_code is "allow", under the floor "forbid"`.

Fix: put the lint back at the floor's level; a crate that needs unsafe is named in [rust] unsafe_crates with its own [lints] denying clippy::undocumented_unsafe_blocks and a // SAFETY: line over each unsafe block; a floor that is wrong is changed in gatekeeper. Rerun: `./.gatekeeper/run check --gate rust/bans`

## unsafe-weakened

Writes `.gatekeeper/config.toml`:

```
packs = ["rust"]
integration = "staging"
pin = "checkout"

[rust]
unsafe_crates = ["gpu"]
```

Writes `Cargo.toml` (not shown).

Writes `gpu/Cargo.toml` (not shown).

Writes `gpu/src/lib.rs`:

```
#[must_use]
pub fn first(v: &[u8]) -> Option<u8> {
    if v.is_empty() {
        return None;
    }
    // SAFETY: v is not empty, so index 0 is in bounds.
    Some(unsafe { *v.get_unchecked(0) })
}
```

The gate fails and its log prints `crate gpu: gpu/Cargo.toml: [lints.clippy] panic is "warn", under the floor "deny"`.

Fix: put the lint back at the floor's level; a crate that needs unsafe is named in [rust] unsafe_crates with its own [lints] denying clippy::undocumented_unsafe_blocks and a // SAFETY: line over each unsafe block; a floor that is wrong is changed in gatekeeper. Rerun: `./.gatekeeper/run check --gate rust/bans`
