# gatekeeper: one gate kit for every fleet repo (#1)

## Summary

Decision: gatekeeper is one small program and four shared workflows. A repo lists its packs in one file; the program
runs every gate the same way on a lane's host and in CI, and is the fleet's done check. Repos keep no copies of the
rules: each pins one signed release, and a short script in the repo fetches it on any host, with nothing installed.
Slices: the engine with the core and rust packs, gating itself; signed releases and pinning; the skill; hygiene and
PR body; the ts, swift and supabase packs; TestFlight; shards and affected projects; then one adoption per repo.
Open questions: may gatekeeper tag its own releases by version (v1.2.0) where the release contract says build tags?

## Read in this order

| File | What |
|---|---|
| `usage.md` | a consumer's `ci.yml` before and after, a lane's run, a bump, a local gate, kept files, a monorepo, the three other workflows |
| `adr.md` | the question, options, the ten decisions, the criteria mapped, consequences, the issues in order, the open question, the `.drover/arch.toml` lines |
| `packs.md` | each pack's gates, the fixtures that prove each fails, the budgets |
| `inventory.md` | each duplicated file and where it goes: a pack, a drover verb, a workflow, or stays local |
| `shapes.md` | the workflow inputs, the config, a gate, the plan and the report as types |
| `boundaries.md` | the repo layout, which module may call which, four statecharts |

## What the review changed

| The owner's point | Now | Where |
|---|---|---|
| 1. Pin by commit sha, the tag as a comment; named secrets | `uses: …@<sha> # v1.0.0`; `pin` writes it, `core/pin` checks it and refuses `secrets: inherit` | `adr.md` option 3, decisions 1 and 2; `usage.md` |
| 2. A large monorepo does not fit | legs, shards and affected projects join the plan as slice G10; until then its own workflow stays, unfrozen | `adr.md` option 7, decision 9; `shapes.md`; `usage.md` |
| 3. Budgets fail only on GitHub-hosted runners; setup time | a self-hosted runner and a lane's host only report; every job's row shows its setup time | `adr.md` decision 6; `shapes.md` report |
| 4. A gate can name files to upload | `[[upload]]` in a gate file | `usage.md`; `shapes.md` |
| 5. No per-host install | a wrapper in the repo, the same file on a lane's host and a runner | `adr.md` option 6, decision 3; `boundaries.md` |
| 6. Signed `SHA256SUMS`, farrier's checker, who releases first | `release.yml` follows farrier's contract; farrier first, then gatekeeper, then cooee, shed, drover | `adr.md` decision 8; `boundaries.md` |
| Questions 1, 2 and 4: yes | the names list is a CI secret; pgTAP is CI-only on a lane's host; the cap is 4,000 | `adr.md`, "Decided by the owner" |

## The design on one screen

```
 a consumer repo                            gatekeeper (one release = workflows + program + packs)
 ───────────────                            ──────────
 .gatekeeper/config.toml   packs, pin        the pinned program, signed list, sha256 checked
 .gatekeeper/run           the wrapper ────▶   │ plan: config + packs + changed files (+ projects, shards)
 .gatekeeper/gates/*.toml  its own gates       │ run:  each gate, timed against its budget
 Cargo.toml, deny.toml …   stricter is free    │   ├─ the repo's toolchain: cargo, tsc, swift
                                               │   ├─ pinned tools: drover, actionlint, cargo-deny, oxlint, knip
 a lane: .gatekeeper/run check ──┐             │   └─ built-in checks: floors, fixtures, tokens, pin
         drover turn done ───────┼───────────▶ ▼
 CI:     ci.yml uses gates.yml ──┘           report: ok / FAIL / skip, seconds / budget, a fix line, a log file,
         plan ▶ a job per leg ▶ done                 kept files, each job's setup time
```

| Today | After |
|---|---|
| The same gates hand-copied into every repo, drifting | One copy in gatekeeper; consumers hold a config file, the wrapper and their own gates |
| `just check` and `ci.yml` are two lists | `.gatekeeper/run check` is the only list; CI calls it |
| drover, cargo-deny and actionlint installed a different way per repo | One pinned, checksummed version of each, fetched by the program |
| A gate fix lands N times | One PR in gatekeeper, then a one-command bump per repo |
| A private-names list inside public repos | The list stays in the fleet's config and a CI secret |
| No limit on CI time | A budget per gate, failed on GitHub-hosted runners when exceeded |
| Release checksums unsigned; three release workflows | One workflow; a signed list, checked before anything is published |

## What the owner decides

Approve, revise or reject the ADR, and answer its one open question (`adr.md`, near the end): the tag form of
gatekeeper's own releases, which touches a line of farrier's release contract.
