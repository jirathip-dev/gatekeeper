# Code ratchets: how it reads (#100)

Written first: what a repo, a lane and the engine see once the design is built. Numbers marked `n` are not
measured.

## A repo's config after its adoption

```toml
# .gatekeeper/config.toml (drover, as its adoption issue would leave it)
packs = ["core", "rust", "ts"]

[rust.exported]
"crates/sys/src/ffi.rs" = "called from the Swift helper, outside the workspace"

[duplication.ignore]
"crates/cli/tests/golden/**" = "golden output the tests write"

[docs.names]
file = "docs/names.tsv"

[surface]
names = ["flag", "config-key", "refusal"]

[surface.lines]
agent-doc = ["roles/*.md", "usage/**/*.md"]
```

Three new tables, all optional. Dead code and complexity need no key at all. Thresholds and jscpd's settings
are the pack's, not the repo's.

## The adoption PR

```
$ ./.gatekeeper/run pin 1.7.0
$ ./.gatekeeper/run baseline rust/dead-code
.gatekeeper/baseline/rust-dead-code.txt: 315 lines
$ ./.gatekeeper/run baseline rust/complexity
.gatekeeper/baseline/rust-complexity.txt: n lines
$ ./.gatekeeper/run baseline ts/complexity
$ ./.gatekeeper/run baseline core/duplication
ci/ratchet/duplication.d/2026-12-09-1240.toml: ceiling 1240 (1236 duplicated lines)
$ ./.gatekeeper/run baseline core/surface
ci/ratchet/surface-flag.d/2026-12-09-212.toml: ceiling 212
ci/ratchet/surface-config-key.d/2026-12-09-96.toml: ceiling 96
ci/ratchet/surface-refusal.d/2026-12-09-74.toml: ceiling 74
ci/ratchet/surface-agent-doc.d/2026-12-09-640.toml: ceiling 640
```

`ts/knip` and its list are untouched. From here every count is printed on every run:

```
ok    rust/dead-code     315 listed
ok    rust/complexity    n listed
ok    core/duplication   1236 duplicated lines / ceiling 1240 (rust 1102, typescript 134; 88 clones)
ok    core/surface       flag 212/212 · config-key 96/96 · refusal 74/74 · agent-doc 640/640
ok    core/ratchet       no list grew, no ceiling rose
```

## A lane's failing runs

A new `pub` function that only its own crate calls:

```
FAIL  rust/dead-code
crates/core/src/hosts.rs: pub fn spare_slots: no other crate names it
Fix: write pub(crate), and the compiler then says whether it is dead; delete an unused dependency; ...
```

A copied block:

```
FAIL  core/duplication
duplication: 1262 duplicated lines, over the ceiling 1240
  crates/core/src/spawn.rs:210-235 ~ crates/core/src/adopt.rs:88-113 (26 lines)
  ... every clone, by file
Fix: move the copied block into one function both places call
```

A function that grew past a threshold, and one that was already listed and still passes:

```
FAIL  rust/complexity
crates/core/src/drive.rs: fn second_pass: over 60 lines
Fix: split the function; an allow attribute does not hide it from this gate
```

A deleted dead function leaves a list line behind, as knip's does today:

```
FAIL  rust/dead-code
stale: shrink the baseline (./.gatekeeper/run baseline rust/dead-code): crates/core/src/hosts.rs: pub fn held_slots: no other crate names it
```

A removed block of duplication, once a whole step is gone:

```
FAIL  core/duplication
duplication: 1228 duplicated lines, a step under the ceiling 1240: ./.gatekeeper/run baseline core/duplication
$ ./.gatekeeper/run baseline core/duplication
ci/ratchet/duplication.d/2026-12-12-1230.toml: ceiling 1230 (1228 duplicated lines)
```

A hand-edited list:

```
FAIL  core/ratchet
.gatekeeper/baseline/rust-dead-code.txt: 1 line added; a list only shrinks (a raise carries `Ratchet: rust/dead-code <old> → <new> (#issue, q<N>)`)
```

## A raise, with the owner's decision

An issue's spec adds a flag; the lane's run says:

```
FAIL  core/surface
surface-flag: 213 names, over the ceiling 212
```

The lane asks the owner (`drover ask`, an owner ask). On a yes it replaces the folder and says so in the commit:

```
$ git rm -q ci/ratchet/surface-flag.d/*.toml
$ ./.gatekeeper/run baseline core/surface
ci/ratchet/surface-flag.d/2026-12-15-213.toml: ceiling 213
$ git commit -m "spawn: --dry-run for update rounds

Ratchet: surface-flag 212 → 213 (#1790, q3511)"
```

Without the trailer `core/ratchet` fails on the deleted file. A flag added and another removed in one PR needs
none of this: the count did not rise.

## The engine: a list check and a ceiling check

A list check only names what it found. The gate's list, which exists today, does the rest.

```rust
// check/clippy_complexity.rs
/// Clippy's two size lints forced on over the workspace; each function over a threshold is a finding.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    let out = runner::capture("cargo", &clippy_args(cx)?, cx.root)?;
    Ok(reports::clippy(&String::from_utf8_lossy(&out.stdout)))
}
```

A ceiling check measures, then judges against the folder. `baseline <gate>` calls the same `measure`.

```rust
// check/duplication.rs
pub(crate) fn measure(cx: &Cx) -> Result<Vec<Counted>> {
    let report = jscpd(cx)?;
    let found = reports::jscpd(&report)?;
    cx.note.set(found.summary()).ok();
    Ok(vec![Counted { name: CountName::duplication(), n: found.duplicated_lines, step: DUPLICATION_STEP }])
}

/// Pinned jscpd over the repo's code; its duplicated lines under the ceiling in `ci/ratchet/duplication.d`.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    let counts = measure(cx)?;
    Ok(counts.iter().flat_map(|c| judge::ceiling(c, folder::lowest(cx.root, &c.name))).collect())
}
```

```rust
// baseline.rs: one more arm beside arch's and #99's
let measure = match checks.as_slice() {
    [c] if *c == "duplication" => Some(check::duplication::measure(&cx)),
    [c] if *c == "surface" => Some(check::surface::measure(&cx)),
    _ => None,
};
if let Some(counts) = measure {
    return ratchet::folder::write(root, &counts?, today);
}
```
