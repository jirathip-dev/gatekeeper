# Hotspots, week of 2026-11-16: how the callers read (#67)

Read this first. These are the call sites once the pull request lands; `adr.md` says why, `shapes.md` has the
types. The feature names in the examples (`baseline.md`, `seed`, `retries`) are made up.

## A pull request documents its feature

```
crates/gatekeeper-engine/src/skill/topics/
  cache.md         today's paragraph on the result cache, word for word
  select.md        today's paragraph on [select], shards and replay, word for word
  baseline.md      new: this pull request's own file, and the only skill source it touches
```

```
$ just gen                      # rebuilds, writes skill/gatekeeper/**
$ git add crates/gatekeeper-engine/src/skill/topics/baseline.md skill/gatekeeper/SKILL.md
```

- A topic is plain markdown. It is copied as it is into `SKILL.md`, after the opening and before `## Packs`,
  in file-name order, one blank line between topics. Nothing is filled in: `{files}` is written `{files}`.
- A topic stands alone. It never says "above" or "below": the file name decides its place.
- Two pull requests that each add a topic share no source line. Both rewrite the built `SKILL.md`; the one that
  merges second runs `just gen` again and commits. It is never merged by hand (it is listed `generated` today).
- A pull request that extends a feature edits that feature's topic. The first two are written one sentence a
  line, so two sentences added in different places merge.

## A pull request changes what a pack page or an example page says

It edits `skill/pack.rs` (a gate's entry: fix, budget, cached, rerun) or `skill/example.rs` (a fixture case told
in words). `skill.rs` itself is frozen: a line in it may be reworded or removed, none added.

## A test builds a run

```rust
let mut cx = RunCtx::lane(&root, &root.join("cache"));
cx.results = cached.then(|| results(&root));
let report = run::run(&plan, &cx, &mut |_, _| {}).expect("run");
```

`RunCtx::lane` is a lane's plain run of the repo at `root`: no base, no result cache, this host's platform. A
caller sets only what it needs. Writing the struct out field by field no longer compiles outside the engine.

## `check` builds the run

```rust
let mut cx = RunCtx::lane(root, &cli::cache());
cx.base = base;
cx.pr_body = args.value("--pr-body").map(PathBuf::from);
cx.host = host;
cx.job_start = cli::job_start();
cx.manifest = config::manifest(&config).to_owned();
cx.after_reports = args.value("--after-reports").map(PathBuf::from);
cx.ts = config.ts.clone();
cx.swift = config.swift.clone();
cx.tools = config::tools(&config, root);
cx.results = results;
cx.timings = args.value("--timings").map(PathBuf::from);
```

A feature that needs a new input, say `--seed`, adds the field with its default in `run/cx.rs` and one line
here. Two files. Today it is seven: the struct, `check`, `selftest` and four tests.

## The plan's loop, in `run.rs`

```rust
let done = match planned.decision {
    Decision::Run if !waited => GateReport::unrun(&planned.gate, Outcome::Skipped(Skip::AfterFailed)),
    Decision::Run => one(planned, plan, job, cx)?,
    Decision::Skip(skip) => GateReport::unrun(&planned.gate, Outcome::Skipped(skip)),
};
```

```rust
fn one(planned: &Planned, plan: &Plan, job: &Job, cx: &RunCtx) -> Result<GateReport> {
    let picked = match select::pick(planned, (plan, job.shard), cx) {
        Ok(picked) => picked,
        Err(ended) => return Ok(*ended),
    };
    let run = |projects: &[String]| gate(&planned.gate, (job.shard, picked.as_ref()), projects, cx);
    let report = match &cx.results {
        Some(results) => {
            let files = picked.as_ref().map(|p| p.digest.as_str());
            cache::gate(results, planned, (job.shard, &plan.workspace, files), &cx.root, run)?
        }
        None => run(&planned.projects)?,
    };
    select::reported(picked.as_ref(), report, &cx.root)
}
```

`one` is the only place that says in what order the layers wrap a gate's run: select, then the result cache,
then the gate. It is today's code, moved out of the loop. A third layer is a few lines here and its own module.

## A gate's report, in `run/gate.rs`

```rust
let mut report = GateReport::unrun(gate, judge(result, took, gate.budget, cx.host));
report.took = took;
report.log = log;                       // the log's path under the root, as today
report.kept = kept::collect(gate, &cx.root, &cx.out)?;
report.projects = projects.to_vec();
report.note = summary.into_inner();
Ok(report)
```

A feature that reports something new, say `retries`, adds the field and its empty value in the model and sets
it in its own module, the way `cache::gate` sets `cache`. `run/` is not opened.

## A feature brings its own places for the run line

```rust
impl Picked {
    /// What a `[select]` gate's line gets: its files as arguments, and where vitest writes its report.
    pub(crate) fn vars(&self) -> Vars {
        vec![("{files}", "\"$@\"".to_owned()), ("{report}", self.report.to_string_lossy().into_owned())]
    }
}
```

```rust
let mut vars = picked.map_or_else(Vars::new, Picked::vars);
vars.extend([("{shard}", shard.to_string()), ("{projects}", projects.join(" "))]);
```

`run/line.rs` fills the gate's own places first, in the order given, then the ones every line has: `{--base}`,
`{pr-body}`, the pack's `{files}` and `{scripts}` folders, `{tool:<name>}`. It no longer asks whether the gate
selects.

## What a user sees

Nothing new. Every verb, flag, row, log and `report.json` is as before. `skill/gatekeeper/packs/` and
`examples/` are the same bytes; `SKILL.md` has the same words, with line breaks inside its two long paragraphs.
