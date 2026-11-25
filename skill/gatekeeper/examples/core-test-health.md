# core/test-health: what a failure looks like

Each case is one change on the pack's fixture repo. `gatekeeper selftest` runs every case and fails unless the gate fails on it printing the line shown, so these examples cannot go stale.

## a-step-under

Writes `crates/low/src/more.rs`:

```
pub fn more() -> u32 {
    2
}

pub fn most() -> u32 {
    3
}
```

The gate fails and its log prints `ratio 0.93 is a step under the ceiling 1.20: lower it`.

Fix: delete tests or test code; a ratio a whole step under its ceiling turns the ratchet: ./.gatekeeper/run baseline core/test-health adds a lower file. A ceiling file is never edited or deleted. Rerun: `./.gatekeeper/run check --gate core/test-health`

## ceiling-edited

Writes `ci/tests/ratio.d/2026-11-25-120.toml`:

```
ceiling = 1.2
```

The gate fails and its log prints `ci/tests/ratio.d/2026-11-25-120.toml: a ceiling file is never edited or deleted`.

Fix: delete tests or test code; a ratio a whole step under its ceiling turns the ratchet: ./.gatekeeper/run baseline core/test-health adds a lower file. A ceiling file is never edited or deleted. Rerun: `./.gatekeeper/run check --gate core/test-health`

## ceiling-raised-by-a-new-file

Writes `ci/tests/ratio.d/2026-11-26-125.toml`:

```
ceiling = 1.25
```

The gate fails and its log prints `ci/tests/ratio.d/2026-11-26-125.toml: ceiling 1.25 is not under the base's ceiling 1.20`.

Fix: delete tests or test code; a ratio a whole step under its ceiling turns the ratchet: ./.gatekeeper/run baseline core/test-health adds a lower file. A ceiling file is never edited or deleted. Rerun: `./.gatekeeper/run check --gate core/test-health`

## inline-tail-counts-as-test

Writes `crates/high/src/extra.rs`:

```
pub fn extra() {}
#[cfg(test)]
mod tests {
    #[test]
    fn extra() {}
}
```

The gate fails and its log prints `ratio 1.37 is over the ceiling 1.20`.

Fix: delete tests or test code; a ratio a whole step under its ceiling turns the ratchet: ./.gatekeeper/run baseline core/test-health adds a lower file. A ceiling file is never edited or deleted. Rerun: `./.gatekeeper/run check --gate core/test-health`

## no-ceiling

Removes `ci/tests/ratio.d/2026-11-25-120.toml`.

The gate fails and its log prints `no ceiling in ci/tests/ratio.d`.

Fix: delete tests or test code; a ratio a whole step under its ceiling turns the ratchet: ./.gatekeeper/run baseline core/test-health adds a lower file. A ceiling file is never edited or deleted. Rerun: `./.gatekeeper/run check --gate core/test-health`

## over-ceiling

Writes `crates/low/tests/more.rs`:

```
#[test]
fn seven() {}
```

The gate fails and its log prints `ratio 1.29 is over the ceiling 1.20 (ci/tests/ratio.d/2026-11-25-120.toml)`.

Fix: delete tests or test code; a ratio a whole step under its ceiling turns the ratchet: ./.gatekeeper/run baseline core/test-health adds a lower file. A ceiling file is never edited or deleted. Rerun: `./.gatekeeper/run check --gate core/test-health`

## src-override

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[tests]
src = ["crates/low/tests/**"]
```

The gate fails and its log prints `ratio 0.00 is a step under the ceiling 1.20: lower it`.

Fix: delete tests or test code; a ratio a whole step under its ceiling turns the ratchet: ./.gatekeeper/run baseline core/test-health adds a lower file. A ceiling file is never edited or deleted. Rerun: `./.gatekeeper/run check --gate core/test-health`

## swift-tests-folder

Writes `Tests/AppTests/a.swift`:

```
func testOne() {}
func testTwo() {}
func testThree() {}
```

The gate fails and its log prints `ratio 1.34 is over the ceiling 1.20`.

Fix: delete tests or test code; a ratio a whole step under its ceiling turns the ratchet: ./.gatekeeper/run baseline core/test-health adds a lower file. A ceiling file is never edited or deleted. Rerun: `./.gatekeeper/run check --gate core/test-health`

## ts-spec-files

Writes `web/a.spec.ts`:

```
test("one", () => {});
test("two", () => {});
test("three", () => {});
```

The gate fails and its log prints `ratio 1.34 is over the ceiling 1.20`.

Fix: delete tests or test code; a ratio a whole step under its ceiling turns the ratchet: ./.gatekeeper/run baseline core/test-health adds a lower file. A ceiling file is never edited or deleted. Rerun: `./.gatekeeper/run check --gate core/test-health`

## under-one

Writes `ci/tests/ratio.d/2026-11-26-100.toml`:

```
ceiling = 1.00
```

Removes `crates/low/tests/low.rs`.

The gate passes.
