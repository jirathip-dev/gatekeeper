# core/test-sleeps: what a failure looks like

Each case is one change on the pack's fixture repo. `gatekeeper selftest` runs every case and fails unless the gate fails on it printing the line shown, so these examples cannot go stale.

## listed-passes

Writes `.gatekeeper/baseline/core-test-sleeps.txt`:

```
crates/low/tests/wait.rs: thread::sleep( #1
```

Writes `crates/low/tests/wait.rs`:

```
use std::time::Duration;
#[test]
fn waits() {
    std::thread::sleep(Duration::from_millis(10));
}
```

The gate passes.

## one-more-than-listed

Writes `.gatekeeper/baseline/core-test-sleeps.txt`:

```
crates/low/tests/wait.rs: thread::sleep( #1
```

Writes `crates/low/tests/wait.rs`:

```
use std::time::Duration;
#[test]
fn waits() {
    std::thread::sleep(Duration::from_millis(10));
    std::thread::sleep(Duration::from_millis(10));
}
```

The gate fails and its log prints `crates/low/tests/wait.rs: thread::sleep( #2`.

Fix: wait on the event, a channel or a fake clock instead; a perf or timing test moves into a [tests] soak file. Existing hits: ./.gatekeeper/run baseline core/test-sleeps, a list that only shrinks. Rerun: `./.gatekeeper/run check --gate core/test-sleeps`

## rust-inline-tail

Writes `crates/low/src/late.rs`:

```
pub fn late() -> u8 {
    1
}
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn waits() {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}
```

The gate fails and its log prints `crates/low/src/late.rs: time::sleep( #1`.

Fix: wait on the event, a channel or a fake clock instead; a perf or timing test moves into a [tests] soak file. Existing hits: ./.gatekeeper/run baseline core/test-sleeps, a list that only shrinks. Rerun: `./.gatekeeper/run check --gate core/test-sleeps`

## rust-sleep

Writes `crates/low/tests/wait.rs`:

```
use std::time::Duration;
#[test]
fn waits() {
    std::thread::sleep(Duration::from_millis(10));
}
```

The gate fails and its log prints `crates/low/tests/wait.rs: thread::sleep( #1`.

Fix: wait on the event, a channel or a fake clock instead; a perf or timing test moves into a [tests] soak file. Existing hits: ./.gatekeeper/run baseline core/test-sleeps, a list that only shrinks. Rerun: `./.gatekeeper/run check --gate core/test-sleeps`

## soak-file-passes

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[tests]
soak = ["crates/*/tests/soak.rs"]
```

Writes `crates/low/tests/soak.rs`:

```
use std::time::Duration;
#[test]
fn waits() {
    std::thread::sleep(Duration::from_millis(10));
}
```

The gate passes.

## swift-sleep

Writes `Tests/AppTests/WaitTests.swift`:

```
func testWaits() async throws {
    try await Task.sleep(nanoseconds: 10)
}
```

The gate fails and its log prints `Tests/AppTests/WaitTests.swift: Task.sleep( #1`.

Fix: wait on the event, a channel or a fake clock instead; a perf or timing test moves into a [tests] soak file. Existing hits: ./.gatekeeper/run baseline core/test-sleeps, a list that only shrinks. Rerun: `./.gatekeeper/run check --gate core/test-sleeps`

## ts-timeout

Writes `web/ui/wait.test.ts`:

```
test("waits", async () => {
  await new Promise((done) => setTimeout(done, 10));
});
```

The gate fails and its log prints `web/ui/wait.test.ts: setTimeout( #1`.

Fix: wait on the event, a channel or a fake clock instead; a perf or timing test moves into a [tests] soak file. Existing hits: ./.gatekeeper/run baseline core/test-sleeps, a list that only shrinks. Rerun: `./.gatekeeper/run check --gate core/test-sleeps`

## wall-clock-read

Writes `crates/low/tests/clock.rs`:

```
#[test]
fn clock() {
    let start = std::time::Instant::now();
    assert!(start.elapsed().as_secs() < 1);
}
```

The gate fails and its log prints `crates/low/tests/clock.rs: Instant::now( #1`.

Fix: wait on the event, a channel or a fake clock instead; a perf or timing test moves into a [tests] soak file. Existing hits: ./.gatekeeper/run baseline core/test-sleeps, a list that only shrinks. Rerun: `./.gatekeeper/run check --gate core/test-sleeps`
