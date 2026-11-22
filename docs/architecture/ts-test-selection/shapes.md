# Per-file test selection: data shapes (#46)

## What a gate writes

```rust
/// `[select]` in a gate file; its presence makes the gate run over selected test files.
struct Select {
    vitest: RepoPath,     // the one vitest config of this gate's run line
    always: Vec<Glob>,    // test files selected on every run. Default empty.
    full: Vec<Glob>,      // required: a changed file matching one selects everything. `.gatekeeper/**` is implied.
}

struct GateFile {
    // …
    select: Option<Select>,
}
```

Refused when the gate file is parsed: `[select]` with `projects`; with `[[step]]` in place of `run`; a `run`
without `{files}`; more than one shard without `{report}`; a config with no `[ts]` or no `[scope]`.

## The change

```rust
/// `git diff --name-status --no-renames <base>` plus untracked files. A rename is `Deleted` and `Added`.
struct Change { path: RepoPath, kind: ChangeKind }
enum ChangeKind { Added, Modified, Deleted }

enum Trigger { Change(Vec<Change>), Everything(Widened) }   // today `Change` holds names only
```

## The evidence and the selection

```rust
struct TestFile(RepoPath);

/// Everything the rules read. `universe` and `closure` come from vitest; the rest from git and the tree.
struct Evidence<'a> {
    universe: &'a BTreeSet<TestFile>,        // `vitest list --filesOnly` under the gate's config
    closure: &'a BTreeSet<TestFile>,         // the same with `--changed <base>`
    texts: &'a BTreeMap<TestFile, String>,   // each test file's own text
    graph: &'a Graph,                        // the projects (G10)
}

fn select(trigger: &Trigger, gate: &Select, evidence: &Evidence) -> Selection

enum Selection {
    Full(Full),                                    // every file of the universe
    Files(BTreeMap<TestFile, BTreeSet<Why>>),      // never empty
}
enum Full {
    NotAChange(Widened),     // a push, --all, scope full, no base
    File(RepoPath),          // a changed file matches `full` or `.gatekeeper/**`
    Empty,                   // no rule selected a test
}
```

`Selection::Files` is built by one function that returns `Full(Empty)` for an empty map, so "nothing to run"
cannot be made.

```rust
enum Why {
    Closure,                    // vitest: its import graph reaches a changed file
    Always(Always),
    Reads(Read),                // fs-readers
    NamesProject(ProjectRoot),  // project readers
    Widened(Widen),
}
enum Always { Path, NoProject }
enum Read { Path(RepoPath), Basename(RepoPath), Walks }
enum Widen { FormerProject(RepoPath), Specifier(RepoPath) }   // the deleted file
```

## The rules, exactly

`select` returns `Full` when the trigger is not a change or a changed file matches `full`. Otherwise it is the
union below, over the tests of the universe; an empty union is `Full(Empty)`.

| Why | A test file is selected when |
|---|---|
| `Closure` | vitest lists it for `--changed <base>` |
| `Always(Path)` | it matches a glob of `always` |
| `Always(NoProject)` | no project of the graph holds it |
| `Reads(Path)` | its text contains a changed file's repo path |
| `Reads(Basename)` | a changed file's basename has an extension, and its text contains the basename as a whole token |
| `Reads(Walks)` | anything changed, and its text has a walker token |
| `NamesProject` | its text contains, as a whole token, the root of a project that holds a changed file |
| `Widened(FormerProject)` | a deleted file that was no test file lay in its project |
| `Widened(Specifier)` | a deleted file that was no test file is named by an import specifier in its text |

- **Changed file** covers added, modified and deleted ones; the two `Widened` rules read only deleted ones.
- **A whole token**: the characters before and after the match are not a letter, a digit, `_`, `-` or `.`.
- **Walker tokens**: a name starting with `readdir`; a name starting with `glob` but not `global`;
  `import.meta.glob`; `git ls-files`; a name starting with `execFile` or `spawn`.
- **Was a test file**: its name has `.test.` or `.spec.` before its last extension. A deleted test file widens
  nothing.
- **Named by an import specifier**: a quoted string in the text ends with `/` and the deleted file's name
  without its extension, or with a JavaScript or TypeScript extension. For `index.*` the name is its folder's.
- A project's root `.` is never matched by `NamesProject`.

vitest adds one rule of its own inside `Closure`: a changed `package.json` or vitest or vite config lists every
test of that config (`forceRerunTriggers`). It stays the repo's setting.

## The split

```rust
/// `<timings>/<gate>.json`: what the last full run on the integration branch measured.
struct Timings { schema: u32, files: BTreeMap<TestFile, Millis> }

fn split(files: &BTreeSet<TestFile>, timings: &Timings, shard: Shard) -> Vec<TestFile>
```

Files are ordered by time, longest first, then by path; each goes to the shard with the least time so far, the
lowest index on a tie. A file with no time takes the median of those that have one, or one second. For any
timings the shards are a partition of `files`. A lane runs `1/1`: the whole list.

## What a job reports

```rust
struct GateReport { /* … */ selection: Option<Selected>, files: BTreeMap<TestFile, Millis> }

struct Selected {
    universe: u32,
    selected: u32,                      // of the whole gate, before the split
    slice: u32,                         // this job's shard
    full: Option<Full>,
    by: BTreeMap<String, u32>,          // closure, always, readers, project-readers, widened
    digest: Sha256,                     // of the sorted selected list and the timings it was split with
    took: Millis,                       // the install and both vitest lists
}

enum Skip { /* … */ EmptyShard }
```

`ci done` fails a gate whose shards report different `digest`s or whose `slice`s do not sum to `selected`. It
merges the `files` of every shard into one `Timings` per gate under `--timings-out`.

## The result cache

```rust
struct KeyParts {
    schema: u32,              // 2: every record misses once
    // …
    files: Option<Sha256>,    // a `[select]` gate: the sorted list this job ran
}
```

A `[select]` gate's inputs are every tracked file, as for any gate without `paths`. One record per job.

## What `ci plan` tells the workflow

```json
{ "include": [ { "leg": "unit", "shard": "2/4", "artifact": "gates-unit-2",
                 "timings": "gatekeeper-timings-1-9113" } ] }
```

and one more output line, `timings-save=gatekeeper-timings-1-` on a push to the integration branch, else empty.
`timings` is the exact entry the `plan` job found, the same string in every shard; empty when there is none yet.

## The replay

```rust
/// One line of `--runs`.
struct ReplayRun { pr: u64, base: Rev, head: Rev, failed: BTreeSet<TestFile> }

/// `replay.json`.
struct Replay { gate: GateId, runs: Vec<Replayed>, totals: Totals }

struct Replayed {
    pr: u64,
    head: Rev,
    base: Rev,                          // the merge base it was diffed against
    changed: u32,
    universe: u32,
    gatekeeper: Picked,
    other: Picked,
    only_gatekeeper: BTreeSet<TestFile>,
    only_other: BTreeSet<TestFile>,
    failed: BTreeSet<TestFile>,
    missed_gatekeeper: BTreeSet<TestFile>,   // failed, and not selected by gatekeeper
    missed_other: BTreeSet<TestFile>,
}
struct Picked { selected: u32, full: Option<String>, by: BTreeMap<String, u32> }   // `by` is empty for `other`

struct Totals {
    runs: u32,
    pull_requests: u32,
    file_runs: u64,                     // the sum of `universe`
    gatekeeper: Side,
    other: Side,
    by: BTreeMap<String, u64>,
}
struct Side { selected: u64, full: u32, missed: u32 }
```

`replay.md` is the table of `usage.md`, one row per run, then the two totals lines.
