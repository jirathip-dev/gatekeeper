# core/duplication: what a failure looks like

Each case is one change on the pack's fixture repo. `gatekeeper selftest` runs every case and fails unless the gate fails on it printing the line shown, so these examples cannot go stale.

## a-step-under

Writes `.gatekeeper/config.toml`:

```
packs = ["core", "rust"]
integration = "main"
pin = "checkout"
```

Writes `ci/ratchet/duplication.d/2026-11-26-30.toml`:

```
ceiling = 30
```

Writes `crates/high/src/copy.rs`:

```
pub fn handle(items: &[u32], limit: u32) -> Vec<String> {
    let mut out = Vec::new();
    for item in items {
        if *item > limit {
            out.push(format!("over: {item} > {limit}"));
        } else if *item == limit {
            out.push(format!("at: {item} == {limit}"));
        } else {
            let gap = limit - item;
            out.push(format!("under: {item} by {gap}"));
        }
    }
    out.sort();
    out.dedup();
    out
}
```

Writes `crates/low/src/copy.rs`:

```
pub fn handle(items: &[u32], limit: u32) -> Vec<String> {
    let mut out = Vec::new();
    for item in items {
        if *item > limit {
            out.push(format!("over: {item} > {limit}"));
        } else if *item == limit {
            out.push(format!("at: {item} == {limit}"));
        } else {
            let gap = limit - item;
            out.push(format!("under: {item} by {gap}"));
        }
    }
    out.sort();
    out.dedup();
    out
}
```

Removes `ci/ratchet/duplication.d/2026-11-26-0.toml`.

The gate fails and its log prints `duplication: 16 duplicated lines, a step under the ceiling 30: ./.gatekeeper/run baseline core/duplication`.

Fix: move the copied block into one function both places call; a count a whole step under its ceiling turns the ratchet: ./.gatekeeper/run baseline core/duplication adds a lower file. A ceiling file is never edited or deleted. Rerun: `./.gatekeeper/run check --gate core/duplication`

## fixture-folder

Writes `.gatekeeper/config.toml`:

```
packs = ["core", "rust"]
integration = "main"
pin = "checkout"
```

Writes `crates/high/tests/fixtures/copy.rs`:

```
pub fn handle(items: &[u32], limit: u32) -> Vec<String> {
    let mut out = Vec::new();
    for item in items {
        if *item > limit {
            out.push(format!("over: {item} > {limit}"));
        } else if *item == limit {
            out.push(format!("at: {item} == {limit}"));
        } else {
            let gap = limit - item;
            out.push(format!("under: {item} by {gap}"));
        }
    }
    out.sort();
    out.dedup();
    out
}
```

Writes `crates/low/src/copy.rs`:

```
pub fn handle(items: &[u32], limit: u32) -> Vec<String> {
    let mut out = Vec::new();
    for item in items {
        if *item > limit {
            out.push(format!("over: {item} > {limit}"));
        } else if *item == limit {
            out.push(format!("at: {item} == {limit}"));
        } else {
            let gap = limit - item;
            out.push(format!("under: {item} by {gap}"));
        }
    }
    out.sort();
    out.dedup();
    out
}
```

The gate passes.

## generated-file

Writes `.drover/arch.toml` (not shown).

Writes `.gatekeeper/config.toml`:

```
packs = ["core", "rust"]
integration = "main"
pin = "checkout"
```

Writes `crates/high/src/gen.rs`:

```
pub fn handle(items: &[u32], limit: u32) -> Vec<String> {
    let mut out = Vec::new();
    for item in items {
        if *item > limit {
            out.push(format!("over: {item} > {limit}"));
        } else if *item == limit {
            out.push(format!("at: {item} == {limit}"));
        } else {
            let gap = limit - item;
            out.push(format!("under: {item} by {gap}"));
        }
    }
    out.sort();
    out.dedup();
    out
}
```

Writes `crates/low/src/copy.rs`:

```
pub fn handle(items: &[u32], limit: u32) -> Vec<String> {
    let mut out = Vec::new();
    for item in items {
        if *item > limit {
            out.push(format!("over: {item} > {limit}"));
        } else if *item == limit {
            out.push(format!("at: {item} == {limit}"));
        } else {
            let gap = limit - item;
            out.push(format!("under: {item} by {gap}"));
        }
    }
    out.sort();
    out.dedup();
    out
}
```

The gate passes.

## ignore-comment

Writes `.gatekeeper/config.toml`:

```
packs = ["core", "rust"]
integration = "main"
pin = "checkout"
```

Writes `crates/high/src/copy.rs`:

```
// jscpd:ignore-start
pub fn handle(items: &[u32], limit: u32) -> Vec<String> {
    let mut out = Vec::new();
    for item in items {
        if *item > limit {
            out.push(format!("over: {item} > {limit}"));
        } else if *item == limit {
            out.push(format!("at: {item} == {limit}"));
        } else {
            let gap = limit - item;
            out.push(format!("under: {item} by {gap}"));
        }
    }
    out.sort();
    out.dedup();
    out
}
// jscpd:ignore-end
```

Writes `crates/low/src/copy.rs`:

```
pub fn handle(items: &[u32], limit: u32) -> Vec<String> {
    let mut out = Vec::new();
    for item in items {
        if *item > limit {
            out.push(format!("over: {item} > {limit}"));
        } else if *item == limit {
            out.push(format!("at: {item} == {limit}"));
        } else {
            let gap = limit - item;
            out.push(format!("under: {item} by {gap}"));
        }
    }
    out.sort();
    out.dedup();
    out
}
```

The gate fails and its log prints `crates/high/src/copy.rs: a jscpd:ignore-start comment hides a clone from the count`.

Fix: move the copied block into one function both places call; a count a whole step under its ceiling turns the ratchet: ./.gatekeeper/run baseline core/duplication adds a lower file. A ceiling file is never edited or deleted. Rerun: `./.gatekeeper/run check --gate core/duplication`

## ignore-matches-no-file

Writes `.gatekeeper/config.toml`:

```
packs = ["core", "rust"]
integration = "main"
pin = "checkout"

[duplication.ignore]
"crates/gone/**" = "an old copy"
```

The gate fails and its log prints `[duplication.ignore] `crates/gone/**` matches no file`.

Fix: move the copied block into one function both places call; a count a whole step under its ceiling turns the ratchet: ./.gatekeeper/run baseline core/duplication adds a lower file. A ceiling file is never edited or deleted. Rerun: `./.gatekeeper/run check --gate core/duplication`

## ignored-with-a-reason

Writes `.gatekeeper/config.toml`:

```
packs = ["core", "rust"]
integration = "main"
pin = "checkout"

[duplication.ignore]
"crates/high/src/copied.rs" = "a vendored copy the upstream crate keeps in step"
```

Writes `crates/high/src/copied.rs`:

```
pub fn handle(items: &[u32], limit: u32) -> Vec<String> {
    let mut out = Vec::new();
    for item in items {
        if *item > limit {
            out.push(format!("over: {item} > {limit}"));
        } else if *item == limit {
            out.push(format!("at: {item} == {limit}"));
        } else {
            let gap = limit - item;
            out.push(format!("under: {item} by {gap}"));
        }
    }
    out.sort();
    out.dedup();
    out
}
```

Writes `crates/low/src/copy.rs`:

```
pub fn handle(items: &[u32], limit: u32) -> Vec<String> {
    let mut out = Vec::new();
    for item in items {
        if *item > limit {
            out.push(format!("over: {item} > {limit}"));
        } else if *item == limit {
            out.push(format!("at: {item} == {limit}"));
        } else {
            let gap = limit - item;
            out.push(format!("under: {item} by {gap}"));
        }
    }
    out.sort();
    out.dedup();
    out
}
```

The gate passes.

## no-ceiling

Writes `.gatekeeper/config.toml`:

```
packs = ["core", "rust"]
integration = "main"
pin = "checkout"
```

Removes `ci/ratchet/duplication.d/2026-11-26-0.toml`.

The gate fails and its log prints `duplication: no ceiling: ./.gatekeeper/run baseline core/duplication`.

Fix: move the copied block into one function both places call; a count a whole step under its ceiling turns the ratchet: ./.gatekeeper/run baseline core/duplication adds a lower file. A ceiling file is never edited or deleted. Rerun: `./.gatekeeper/run check --gate core/duplication`

## over-ceiling

Writes `.gatekeeper/config.toml`:

```
packs = ["core", "rust"]
integration = "main"
pin = "checkout"
```

Writes `crates/high/src/copy.rs`:

```
pub fn handle(items: &[u32], limit: u32) -> Vec<String> {
    let mut out = Vec::new();
    for item in items {
        if *item > limit {
            out.push(format!("over: {item} > {limit}"));
        } else if *item == limit {
            out.push(format!("at: {item} == {limit}"));
        } else {
            let gap = limit - item;
            out.push(format!("under: {item} by {gap}"));
        }
    }
    out.sort();
    out.dedup();
    out
}
```

Writes `crates/low/src/copy.rs`:

```
pub fn handle(items: &[u32], limit: u32) -> Vec<String> {
    let mut out = Vec::new();
    for item in items {
        if *item > limit {
            out.push(format!("over: {item} > {limit}"));
        } else if *item == limit {
            out.push(format!("at: {item} == {limit}"));
        } else {
            let gap = limit - item;
            out.push(format!("under: {item} by {gap}"));
        }
    }
    out.sort();
    out.dedup();
    out
}
```

The gate fails and its log prints `duplication: 16 duplicated lines, over the ceiling 0`.

Fix: move the copied block into one function both places call; a count a whole step under its ceiling turns the ratchet: ./.gatekeeper/run baseline core/duplication adds a lower file. A ceiling file is never edited or deleted. Rerun: `./.gatekeeper/run check --gate core/duplication`

## tests-are-read

Writes `.gatekeeper/config.toml`:

```
packs = ["core", "rust"]
integration = "main"
pin = "checkout"
```

Writes `crates/low/src/copy.rs`:

```
pub fn handle(items: &[u32], limit: u32) -> Vec<String> {
    let mut out = Vec::new();
    for item in items {
        if *item > limit {
            out.push(format!("over: {item} > {limit}"));
        } else if *item == limit {
            out.push(format!("at: {item} == {limit}"));
        } else {
            let gap = limit - item;
            out.push(format!("under: {item} by {gap}"));
        }
    }
    out.sort();
    out.dedup();
    out
}
```

Writes `crates/low/tests/copy.rs`:

```
pub fn handle(items: &[u32], limit: u32) -> Vec<String> {
    let mut out = Vec::new();
    for item in items {
        if *item > limit {
            out.push(format!("over: {item} > {limit}"));
        } else if *item == limit {
            out.push(format!("at: {item} == {limit}"));
        } else {
            let gap = limit - item;
            out.push(format!("under: {item} by {gap}"));
        }
    }
    out.sort();
    out.dedup();
    out
}
```

The gate fails and its log prints `duplication: 16 duplicated lines, over the ceiling 0`.

Fix: move the copied block into one function both places call; a count a whole step under its ceiling turns the ratchet: ./.gatekeeper/run baseline core/duplication adds a lower file. A ceiling file is never edited or deleted. Rerun: `./.gatekeeper/run check --gate core/duplication`

## under-the-minimum-size

Writes `.gatekeeper/config.toml`:

```
packs = ["core", "rust"]
integration = "main"
pin = "checkout"
```

Writes `crates/high/src/small.rs`:

```
pub fn small(a: u32, b: u32) -> u32 {
    let c = a * b + a - b;
    let d = c * 2 + a;
    d + c
}
```

Writes `crates/low/src/small.rs`:

```
pub fn small(a: u32, b: u32) -> u32 {
    let c = a * b + a - b;
    let d = c * 2 + a;
    d + c
}
```

The gate passes.
