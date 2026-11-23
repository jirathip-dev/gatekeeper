# core/arch: what a failure looks like

Each case is one change on the pack's fixture repo. `gatekeeper selftest` runs every case and fails unless the gate fails on it printing the line shown, so these examples cannot go stale.

## ban-case-sensitive

Writes `.drover/arch.toml`:

```
[[ban]]
pattern = "codex"
paths = ["crates/*/src/**"]
except = ["crates/core/src/providers/**"]
reason = "core never names a provider"
comments = true
```

Writes `crates/core/src/providers/codex.rs`:

```
const L: &str = "codex login";
```

Writes `crates/core/src/setup.rs`:

```
fn a() {}
const L: &str = "Codex login";
```

The gate passes.

## ban-hit

Writes `crates/low/src/lib.rs`:

```
pub fn low() -> u32 {
    1
}

pub fn quit() {
    std::process::exit(1)
}
```

The gate fails and its log prints `is banned here`.

Fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file. Rerun: `./.gatekeeper/run check --gate core/arch`

## ban-ignore-case

Writes `.drover/arch.toml`:

```
[[ban]]
pattern = "codex"
paths = ["crates/*/src/**"]
except = ["crates/core/src/providers/**"]
reason = "core never names a provider"
comments = true
ignore_case = true
```

Writes `crates/core/src/providers/codex.rs`:

```
const L: &str = "codex login";
```

Writes `crates/core/src/setup.rs`:

```
fn a() {}
const L: &str = "Codex login";
```

The gate fails and its log prints `crates/core/src/setup.rs:2: `codex` is banned here: core never names a provider`.

Fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file. Rerun: `./.gatekeeper/run check --gate core/arch`

## ban-outside-its-except

Writes `.drover/arch.toml` (not shown).

Writes `crates/cli/src/main.rs`:

```
fn main() {
    let _ = Command::new("sh");
}
```

Writes `crates/core/src/gate.rs`:

```
// never Command::new here
fn f() {}
```

Writes `crates/core/src/runner.rs`:

```
pub fn run() {
    std::process::Command::new("git");
}
```

The gate fails and its log prints `crates/cli/src/main.rs:2: `Command::new` is banned here: #713: spawn processes through the Runner`.

Fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file. Rerun: `./.gatekeeper/run check --gate core/arch`

## ban-skips-a-comment

Writes `.drover/arch.toml`:

```
[[ban]]
pattern = "@ts-expect-error"
paths = ["src/**"]
reason = "type it"
```

Writes `src/a.ts`:

```
const a = 1;
  // @ts-expect-error
const b: number = a;
```

The gate passes.

## ban-with-comments-hits-a-comment

Writes `.drover/arch.toml`:

```
[[ban]]
pattern = "@ts-expect-error"
paths = ["src/**"]
reason = "type it"
comments = true
```

Writes `src/a.ts`:

```
const a = 1;
  // @ts-expect-error
const b: number = a;
```

The gate fails and its log prints `src/a.ts:2: `@ts-expect-error` is banned here: type it`.

Fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file. Rerun: `./.gatekeeper/run check --gate core/arch`

## ban-workflow-yml-and-test-mjs

Writes `.drover/arch.toml`:

```
[[ban]]
pattern = "run: exit 1"
paths = [".github/workflows/*.yml"]
except = [".github/workflows/allowed.yml"]
reason = "no bare exit 1 step"

[[ban]]
pattern = "readFileSync("
paths = ["**/*.test.mjs"]
except = ["scripts/ok.test.mjs"]
reason = "read fixtures through the helper"
```

Writes `.github/workflows/allowed.yml`:

```
jobs:
  a:
    steps:
      - run: exit 1
```

Writes `.github/workflows/ci.yml`:

```
jobs:
  a:
    steps:
      - run: exit 1
```

Writes `scripts/a.test.mjs`:

```
import fs from 'node:fs';
const t = fs.readFileSync('x');
```

Writes `scripts/ok.test.mjs`:

```
import fs from 'node:fs';
const t = fs.readFileSync('x');
```

The gate fails and its log prints `.github/workflows/ci.yml:4: `run: exit 1` is banned here: no bare exit 1 step
scripts/a.test.mjs:2: `readFileSync(` is banned here: read fixtures through the helper`.

Fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file. Rerun: `./.gatekeeper/run check --gate core/arch`

## comment-added

Writes `crates/low/src/lib.rs`:

```
pub fn low() -> u32 {
    // one
    1
}
```

The gate fails and its log prints `a comment`.

Fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file. Rerun: `./.gatekeeper/run check --gate core/arch`

## comments-off-fails-only-added-ones

Writes `.drover/arch.toml`:

```
[comments]
policy = "off"
```

Writes `lib/old.rs`:

```
// an old comment
fn old() {}
// SAFETY: the pointer is valid
unsafe { f() }
/// One line.
pub fn a() {}
/// First.
/// Second.
pub fn b() {}
fn c() {
    // new
}
/* block
  still */
fn d() { let x = 1; // trailing
    let s = 2; /* inline */ }
const U: &str = "https://x // y /* z";
const R: &str = r#"a " // b"#;
const C: char = '"'; // after a char
fn e<'a>(v: &'a str) -> &'a str { v }
```

Writes `web/a.ts`:

```
/** Shown. */
export const a = 1;
// not ok
const b = '//' + `/*`; /* not ok */
const c = "// fine";
```

The gate fails and its log prints `lib/old.rs:7: a comment (only `// SAFETY:` over unsafe, `// TODO(#N)` and one `///` on a pub item): put the why in `drover why`
lib/old.rs:8: a comment (only `// SAFETY:` over unsafe, `// TODO(#N)` and one `///` on a pub item): put the why in `drover why`
lib/old.rs:11: a comment (only `// SAFETY:` over unsafe, `// TODO(#N)` and one `///` on a pub item): put the why in `drover why`
lib/old.rs:13: a comment (only `// SAFETY:` over unsafe, `// TODO(#N)` and one `///` on a pub item): put the why in `drover why`
lib/old.rs:14: a comment (only `// SAFETY:` over unsafe, `// TODO(#N)` and one `///` on a pub item): put the why in `drover why`
lib/old.rs:15: a comment (only `// SAFETY:` over unsafe, `// TODO(#N)` and one `///` on a pub item): put the why in `drover why`
lib/old.rs:16: a comment (only `// SAFETY:` over unsafe, `// TODO(#N)` and one `///` on a pub item): put the why in `drover why`
lib/old.rs:19: a comment (only `// SAFETY:` over unsafe, `// TODO(#N)` and one `///` on a pub item): put the why in `drover why`
web/a.ts:3: a comment (only `// SAFETY:` over unsafe, `// TODO(#N)` and one `///` on a pub item): put the why in `drover why`
web/a.ts:4: a comment (only `// SAFETY:` over unsafe, `// TODO(#N)` and one `///` on a pub item): put the why in `drover why``.

Fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file. Rerun: `./.gatekeeper/run check --gate core/arch`

## comments-strict-allowed-ones

Writes `.drover/arch.toml`:

```
[comments]
policy = "strict"
except = ["gen/**", "lib/**"]
```

Writes `gen/x.rs`:

```
// generated
```

Writes `src/lib.rs`:

```
// SAFETY: the pointer is valid
unsafe fn s() {}
// TODO(#12)
fn t() {}
/// Shown.
pub fn p() {}
```

The gate passes.

## comments-strict-old-plain-and-private-doc

Writes `.drover/arch.toml`:

```
[comments]
policy = "strict"
except = ["gen/**", "lib/**"]
```

Writes `gen/x.rs`:

```
// generated
```

Writes `src/bad.rs`:

```
// plain
fn a() {}
/// Private.
fn b() {}
// TODO: no issue
fn c() {}
```

Writes `src/lib.rs`:

```
// SAFETY: the pointer is valid
unsafe fn s() {}
// TODO(#12)
fn t() {}
/// Shown.
pub fn p() {}
```

The gate fails and its log prints `src/bad.rs:1: a comment (only `// SAFETY:` over unsafe, `// TODO(#N)` and one `///` on a pub item): put the why in `drover why`
src/bad.rs:3: a comment (only `// SAFETY:` over unsafe, `// TODO(#N)` and one `///` on a pub item): put the why in `drover why`
src/bad.rs:5: a comment (only `// SAFETY:` over unsafe, `// TODO(#N)` and one `///` on a pub item): put the why in `drover why``.

Fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file. Rerun: `./.gatekeeper/run check --gate core/arch`

## config-unparsable

Writes `.drover/arch.toml`:

```
[[layer
```

The gate fails and its log prints `parsing .drover/arch.toml`.

Fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file. Rerun: `./.gatekeeper/run check --gate core/arch`

## crossing-baselined

Writes `.drover/arch-baseline.txt`:

```
crates/core/src/lib.rs -> server
web/src/api/client.ts -> web-ui
```

Writes `.drover/arch.toml` (not shown).

Writes `crates/core/src/lib.rs`:

```
use drover_server::api;
```

Writes `crates/core/src/ok.rs`:

```
use std::fmt;
```

Writes `web/src/api/client.ts`:

```
import { draw } from '../ui/board';
```

Writes `web/src/ui/board.ts`:

```
export const draw = 1;
```

The gate passes.

## crossing-new-past-its-baseline

Writes `.drover/arch-baseline.txt`:

```
crates/core/src/lib.rs -> server
web/src/api/client.ts -> web-ui
```

Writes `.drover/arch.toml` (not shown).

Writes `crates/core/src/lib.rs`:

```
use drover_server::api;
```

Writes `crates/core/src/ok.rs`:

```
use drover_server::x;
```

Writes `web/src/api/client.ts`:

```
import { draw } from '../ui/board';
import { s } from 'drover_server/s';
```

Writes `web/src/ui/board.ts`:

```
export const draw = 1;
```

The gate fails and its log prints `crates/core/src/ok.rs:1: layer `core` uses `server` (drover_server::x); its may_use is []
web/src/api/client.ts:2: layer `web-api` uses `server` (drover_server/s); its may_use is []`.

Fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file. Rerun: `./.gatekeeper/run check --gate core/arch`

## docs-changed-flag-documented

Writes `.drover/arch.toml`:

```
[docs]
paths = ["docs/**/*.md", "README.md"]
watch = ["lib/cli.rs"]
```

With the PR body (`{head}` is the case's commit, `{base}` its parent):

```
Closes #1
```

Writes `docs/a.md`:

```
`--new-flag`
```

Writes `lib/cli.rs`:

```
struct Args {
    new_flag: bool,
}
```

The gate passes.

## docs-changed-flag-no-doc

Writes `.drover/arch.toml`:

```
[docs]
paths = ["docs/**/*.md", "README.md"]
watch = ["lib/cli.rs"]
```

With the PR body (`{head}` is the case's commit, `{base}` its parent):

```
no doc: internal only
```

Writes `lib/cli.rs`:

```
struct Args {
    new_flag: bool,
}
```

The gate passes.

## docs-changed-flag-undocumented

Writes `.drover/arch.toml`:

```
[docs]
paths = ["docs/**/*.md", "README.md"]
watch = ["lib/cli.rs"]
```

With the PR body (`{head}` is the case's commit, `{base}` its parent):

```
Closes #1
```

Writes `lib/cli.rs`:

```
struct Args {
    new_flag: bool,
}
```

The gate fails and its log prints `lib/cli.rs:2: `old_flag` removed, but no changed doc line names it (or `no doc: <why>` in the PR body)
lib/cli.rs:2: `new_flag` added, but no changed doc line names it (or `no doc: <why>` in the PR body)`.

Fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file. Rerun: `./.gatekeeper/run check --gate core/arch`

## docs-name-real-paths

Writes `.drover/arch.toml`:

```
[docs]
paths = ["docs/**/*.md", "README.md"]
watch = ["lib/cli.rs"]
```

Writes `README.md`:

```
See `src/lib.rs:2`, `src/lib.rs`, [doc](docs/a.md) and `drover spawn --dry-run <f>`.
```

Writes `docs/a.md`:

```
`../src/lib.rs` and `drover queue <fleet> add --top 3`
```
`src/gone.rs`
```
```

Writes `src/lib.rs`:

```
fn a() {}
fn b() {}
```

The gate passes.

## docs-stale-path-line-and-link

Writes `.drover/arch.toml`:

```
[docs]
paths = ["docs/**/*.md", "README.md"]
watch = ["lib/cli.rs"]
```

Writes `README.md`:

```
See `src/lib.rs:2`, `src/lib.rs`, [doc](docs/a.md) and `drover spawn --dry-run <f>`.
```

Writes `docs/a.md`:

```
`../src/lib.rs` and `drover queue <fleet> add --top 3`
```
`src/gone.rs`
```
```

Writes `docs/b.md`:

```
`src/gone.rs`, `src/lib.rs:3`, [x](docs/none.md), `drover spawn --gone`, `drover nope`
```

Writes `src/lib.rs`:

```
fn a() {}
fn b() {}
```

The gate fails and its log prints `docs/b.md:1: `src/gone.rs`: no such path
docs/b.md:1: `src/lib.rs:3`: src/lib.rs has fewer than 3 lines
docs/b.md:1: link `docs/none.md` does not exist`.

Fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file. Rerun: `./.gatekeeper/run check --gate core/arch`

## drover-809-bans-at-home

Writes `.drover/arch.toml` (not shown).

Writes `crates/core/src/clock.rs`:

```
fn a() {
    let _ = Instant::now("x");
}
```

Writes `crates/core/src/config.rs`:

```
fn a() {
    let _ = env::var("x");
}
```

Writes `crates/core/src/runner.rs`:

```
fn a() {
    let _ = std::process("x");
}
```

The gate passes.

## drover-809-bans-outside-home

Writes `.drover/arch.toml` (not shown).

Writes `crates/core/src/clock.rs`:

```
fn a() {
    let _ = Instant::now("x");
}
```

Writes `crates/core/src/config.rs`:

```
fn a() {
    let _ = env::var("x");
}
```

Writes `crates/core/src/runner.rs`:

```
fn a() {
    let _ = std::process("x");
}
```

Writes `crates/server/src/f0.rs`:

```
fn a() {
    let _ = Command::new("x");
}
```

Writes `crates/server/src/f1.rs`:

```
fn a() {
    let _ = std::process("x");
}
```

Writes `crates/server/src/f2.rs`:

```
fn a() {
    let _ = thread::sleep("x");
}
```

Writes `crates/server/src/f3.rs`:

```
fn a() {
    let _ = env::var("x");
}
```

Writes `crates/server/src/f4.rs`:

```
fn a() {
    let _ = SystemTime::now("x");
}
```

Writes `crates/server/src/f5.rs`:

```
fn a() {
    let _ = Instant::now("x");
}
```

The gate fails and its log prints `crates/server/src/f0.rs:2: `Command::new` is banned here: #809: processes start only in core's runner (crate::runner)
crates/server/src/f1.rs:2: `std::process` is banned here: #809: processes start only in core's runner (crate::runner, runner::pid, runner::exit)
crates/server/src/f2.rs:2: `thread::sleep` is banned here: #809: time is read and waited on only in core's clock (crate::clock::sleep)
crates/server/src/f3.rs:2: `env::var` is banned here: #809: the environment is read only in core's config (crate::config::env)
crates/server/src/f4.rs:2: `SystemTime::now` is banned here: #809: time is read and waited on only in core's clock (crate::clock)
crates/server/src/f5.rs:2: `Instant::now` is banned here: #809: time is read and waited on only in core's clock (crate::clock::instant)`.

Fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file. Rerun: `./.gatekeeper/run check --gate core/arch`

## drover-promise-phrase-in-its-module

Writes `.drover/arch.toml`:

```
[[ban]]
pattern = "next pass"
paths = ["crates/*/src/**"]
except = ["crates/core/src/promise.rs"]
reason = "#854: a waiting/next-step line is a typed promise: build it with crate::promise::Say"
```

Writes `crates/core/src/drive.rs`:

```
fn a() {}
```

Writes `crates/core/src/promise.rs`:

```
fn a() -> String {
    format!("merging next pass")
}
```

The gate passes.

## drover-promise-phrase-outside-its-module

Writes `.drover/arch.toml`:

```
[[ban]]
pattern = "next pass"
paths = ["crates/*/src/**"]
except = ["crates/core/src/promise.rs"]
reason = "#854: a waiting/next-step line is a typed promise: build it with crate::promise::Say"
```

Writes `crates/core/src/drive.rs`:

```
fn a() -> String {
    format!("merging next pass")
}
```

The gate fails and its log prints `crates/core/src/drive.rs:2: `next pass` is banned here: #854: a waiting/next-step line is a typed promise: build it with crate::promise::Say`.

Fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file. Rerun: `./.gatekeeper/run check --gate core/arch`

## drover-provider-in-a-web-file

Writes `.drover/arch.toml`:

```
[[ban]]
pattern = "claude"
paths = ["crates/*/src/**", "web/src/**"]
except = ["crates/core/src/providers/**"]
reason = "#806: core never names a provider; its code goes in crates/core/src/providers/ (docs/providers.md); #830: nor does web/src"
comments = true
ignore_case = true
```

Writes `web/src/accounts.ts`:

```
export const v = 'x';
const p = 'Claude';
```

Writes `web/src/main.ts`:

```
export const v = 'x';
```

The gate fails and its log prints `web/src/accounts.ts:2: `claude` is banned here: #806: core never names a provider; its code goes in crates/core/src/providers/ (docs/providers.md); #830: nor does web/src`.

Fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file. Rerun: `./.gatekeeper/run check --gate core/arch`

## exclude-skips-a-test-file

Writes `.drover/arch.toml`:

```
[[layer]]
name = "ui"
paths = ["apps/*/src/**"]
exclude = ["**/*.test.*", "**/__tests__/**"]

[[layer]]
name = "db"
paths = ["db/**"]

[[size]]
paths = ["apps/*/src/**"]
exclude = ["**/*.test.*", "**/__tests__/**"]
max_file_lines = 2
```

Writes `apps/web/src/__tests__/b.ts`:

```
import { q } from "../../../db/q";
const a = 1;
const b = 2;
```

Writes `apps/web/src/a.test.ts`:

```
import { q } from "../../../db/q";
const a = 1;
const b = 2;
```

Writes `apps/web/src/c.ts`:

```
import { q } from "../../../db/q";
const a = 1;
const b = 2;
```

Writes `db/q.ts`:

```
export const q = 1;
```

The gate fails and its log prints `apps/web/src/c.ts:1: layer `ui` uses `db` (db/q); its may_use is []
apps/web/src/c.ts:1: `apps/web/src/c.ts` is 3 lines, over 2 (baseline None)`.

Fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file. Rerun: `./.gatekeeper/run check --gate core/arch`

## file-too-long

Writes `crates/low/src/lib.rs` (not shown).

The gate fails and its log prints `over 20`.

Fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file. Rerun: `./.gatekeeper/run check --gate core/arch`

## frozen-at-the-base-dropped-at-the-head

Writes `.drover/arch.toml`:

```

```

Writes `list.txt`:

```
a
b
```

The gate fails and its log prints `list.txt:2: `b` added to a frozen list`.

Fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file. Rerun: `./.gatekeeper/run check --gate core/arch`

## frozen-file-created

Writes `new-list.txt`:

```
[fleet.x]
line_cap = 10
```

The gate passes.

## frozen-line-added

Writes `list.txt`:

```
a
b
```

The gate fails and its log prints `added to a frozen list`.

Fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file. Rerun: `./.gatekeeper/run check --gate core/arch`

## frozen-line-added-names-the-collected-folder

Writes `.drover/arch.toml`:

```
[[collect]]
dir = "src/features"
glob = "*.rs"
```

Writes `list.txt`:

```
a
b
```

Writes `src/features/a.rs`:

```
fn a() {}
```

The gate fails and its log prints `list.txt:2: `b` added to a frozen list: add one file per entry in `src/features/*.rs` instead`.

Fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file. Rerun: `./.gatekeeper/run check --gate core/arch`

## frozen-line-edited-or-collected

Writes `.drover/arch.toml`:

```
[[collect]]
dir = "src/features"
glob = "*.rs"
```

Writes `list.txt`:

```
bee
```

Writes `src/features/d.rs`:

```
fn d() {}
```

The gate passes.

## frozen-only-at-the-head

Writes `.drover/arch.toml`:

```
frozen = ["src/registry.rs", "src/root.rs"]
```

Writes `src/registry.rs`:

```
mod a;
mod b;
```

Writes `src/root.rs`:

```
mod a;
mod b;
```

The gate passes.

## generated-fresh

Writes `.drover/arch.toml`:

```
generated = ["gen/types.ts"]
generate = "mkdir -p gen && printf 'export type A = 1;\\n' > gen/types.ts"
```

Writes `gen/types.ts`:

```
export type A = 1;
```

The gate passes.

## generated-glob-fresh

Writes `.drover/arch.toml`:

```
generated = ["gen/*.ts"]
generate = "mkdir -p gen && for f in a b c; do echo \"export const $f = 1;\" > gen/$f.ts; done"
```

Writes `gen/a.ts`:

```
export const a = 1;
```

Writes `gen/b.ts`:

```
export const b = 1;
```

Writes `gen/c.ts`:

```
export const c = 1;
```

The gate passes.

## generated-glob-stale-and-extra-match

Writes `.drover/arch.toml`:

```
generated = ["gen/*.ts"]
generate = "mkdir -p gen && for f in a b c; do echo \"export const $f = 1;\" > gen/$f.ts; done"
```

Writes `gen/a.ts`:

```
export const a = 1;
```

Writes `gen/b.ts`:

```
export const b = 2;
```

Writes `gen/c.ts`:

```
export const c = 1;
```

Writes `gen/old.ts`:

```
export const old = 1;
```

The gate fails and its log prints `gen/b.ts: differs from what `mkdir -p gen && for f in a b c; do echo "export const $f = 1;" > gen/$f.ts; done` writes: run it and commit the output, never hand-merge a generated file
gen/old.ts: differs from what `mkdir -p gen && for f in a b c; do echo "export const $f = 1;" > gen/$f.ts; done` writes: run it and commit the output, never hand-merge a generated file`.

Fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file. Rerun: `./.gatekeeper/run check --gate core/arch`

## generated-glob-unwritten-match

Writes `.drover/arch.toml`:

```
generated = ["gen/*.ts"]
generate = "mkdir -p gen && for f in a b c; do echo \"export const $f = 1;\" > gen/$f.ts; done"
```

Writes `gen/a.ts`:

```
export const a = 1;
```

Writes `gen/b.ts`:

```
export const b = 1;
```

The gate fails and its log prints `gen/c.ts: differs from what`.

Fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file. Rerun: `./.gatekeeper/run check --gate core/arch`

## generated-hand-merged

Writes `.drover/arch.toml`:

```
generated = ["gen/types.ts"]
generate = "mkdir -p gen && printf 'export type A = 1;\\n' > gen/types.ts"
```

Writes `gen/types.ts`:

```
export type A = 1;
export type B = 2;
```

The gate fails and its log prints `gen/types.ts: differs from what `mkdir -p gen`.

Fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file. Rerun: `./.gatekeeper/run check --gate core/arch`

## generated-stale

Writes `gen.txt`:

```
edited by hand
```

The gate fails and its log prints `differs from what`.

Fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file. Rerun: `./.gatekeeper/run check --gate core/arch`

## layer-crossed

Writes `crates/low/src/lib.rs`:

```
pub fn low() -> u32 {
    high::high()
}
```

The gate fails and its log prints `layer `low` uses `high``.

Fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file. Rerun: `./.gatekeeper/run check --gate core/arch`

## layers-allowed

Writes `.drover/arch.toml` (not shown).

Writes `crates/core/src/lib.rs`:

```
use std::fmt;
```

Writes `crates/server/src/lib.rs`:

```
use drover_core::{gate, queue};
fn f() { drover_core::x(); }
```

Writes `web/src/api/client.ts`:

```
export const get = 1;
```

Writes `web/src/ui/board.ts`:

```
import { get } from "../api/client";
```

The gate passes.

## layers-crossed-rust-and-ts

Writes `.drover/arch.toml` (not shown).

Writes `crates/core/src/lib.rs`:

```
//! core
use std::fmt;
use drover_server::api;
```

Writes `crates/server/src/lib.rs`:

```
use drover_core::{gate, queue};
fn f() { drover_core::x(); }
```

Writes `web/src/api/client.ts`:

```
// the client
import {
  draw,
} from '../ui/board';
```

Writes `web/src/ui/board.ts`:

```
import { get } from "../api/client";
```

The gate fails and its log prints `crates/core/src/lib.rs:3: layer `core` uses `server` (drover_server::api); its may_use is []
web/src/api/client.ts:4: layer `web-api` uses `web-ui` (web/src/ui/board); its may_use is []`.

Fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file. Rerun: `./.gatekeeper/run check --gate core/arch`

## no-config

Removes `.drover/arch.toml`.

The gate passes.

## shared-under-a-size-rule

Writes `.drover/arch.toml` (not shown).

The gate passes.

## shared-under-no-size-rule

Writes `.drover/arch.toml` (not shown).

The gate fails and its log prints `shared `crates/*/tests/**` falls under no `[[size]]` rule`.

Fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file. Rerun: `./.gatekeeper/run check --gate core/arch`

## size-at-its-baseline

Writes `.drover/arch-baseline.txt`:

```
crates/core/src/big.rs 10
crates/core/src/big.rs:big 5
```

Writes `.drover/arch.toml` (not shown).

Writes `crates/core/src/big.rs`:

```
fn big() {
    a();
    b();
    c();
}




// 10 lines
```

The gate passes.

## size-grown-past-its-baseline

Writes `.drover/arch-baseline.txt`:

```
crates/core/src/big.rs 10
crates/core/src/big.rs:big 5
```

Writes `.drover/arch.toml` (not shown).

Writes `crates/core/src/big.rs`:

```
fn big() {
    a();
    b();
    c();
    d();
}




// 11 lines
```

Writes `crates/core/src/new.rs`:

```
fn big() {
    a();
    b();
    c();
}
```

The gate fails and its log prints `crates/core/src/big.rs:1: `crates/core/src/big.rs` is 11 lines, over 8 (baseline Some(10))
crates/core/src/big.rs:1: `crates/core/src/big.rs:big` is 6 lines, over 4 (baseline Some(5))
crates/core/src/new.rs:1: `crates/core/src/new.rs:big` is 5 lines, over 4 (baseline None)`.

Fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file. Rerun: `./.gatekeeper/run check --gate core/arch`

## size-over-no-baseline

Writes `.drover/arch.toml` (not shown).

Writes `crates/core/src/big.rs`:

```
fn big() {
    a();
    b();
    c();
}




// 10 lines
```

Writes `crates/core/src/ok.rs`:

```
fn small() {
    a();
}
```

The gate fails and its log prints `crates/core/src/big.rs:1: `crates/core/src/big.rs` is 10 lines, over 8 (baseline None)
crates/core/src/big.rs:1: `crates/core/src/big.rs:big` is 5 lines, over 4 (baseline None)`.

Fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file. Rerun: `./.gatekeeper/run check --gate core/arch`

## size-swift-justfile-and-dot-folder

Writes `.drover/arch.toml`:

```
[[size]]
paths = ["**/*.swift"]
max_file_lines = 100
max_fn_lines = 3

[[size]]
paths = ["justfile", ".drover/arch.toml"]
max_file_lines = 6
```

Writes `apple/Fn.swift`:

```
struct S {
    private func big() {
        a()
        b()
        c()
    }
}
```

Writes `apple/Long.swift` (not shown).

Writes `apple/Ok.swift`:

```
func small() {
    a()
}
```

Writes `justfile`:

```
let x0 = 0
let x1 = 1
let x2 = 2
let x3 = 3
let x4 = 4
let x5 = 5
let x6 = 6
let x7 = 7
let x8 = 8
let x9 = 9
let x10 = 10
let x11 = 11
let x12 = 12
let x13 = 13
let x14 = 14
let x15 = 15
let x16 = 16
let x17 = 17
let x18 = 18
let x19 = 19
```

The gate fails and its log prints `.drover/arch.toml:1: `.drover/arch.toml` is 8 lines, over 6 (baseline None)
apple/Fn.swift:2: `apple/Fn.swift:big` is 5 lines, over 3 (baseline None)
apple/Long.swift:1: `apple/Long.swift` is 300 lines, over 100 (baseline None)
justfile:1: `justfile` is 20 lines, over 6 (baseline None)`.

Fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file. Rerun: `./.gatekeeper/run check --gate core/arch`
