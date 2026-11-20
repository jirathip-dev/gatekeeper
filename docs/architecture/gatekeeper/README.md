# gatekeeper: one gate kit for every fleet repo (#1)

## Summary

Decision: gatekeeper is one small program and four shared workflows. A repo lists its packs in one file; the program
runs every gate the same way on a lane's host and in CI, and becomes the fleet's done check. The shared rules ship
inside the program, so consumer repos keep no copies; each pins an exact version and may be stricter, never looser.
Slices: the engine with the core and rust packs, gating itself; releases and pinning; the skill; hygiene and PR body
(after drover's new verbs); the ts, swift and supabase packs; TestFlight; then one adoption per repo, smallest first.
Open questions: the private-names list as a CI secret; database tests CI-only; one install per host; a 4,000-line cap.

## Read in this order

| File | What |
|---|---|
| `usage.md` | a consumer's `ci.yml` before and after, a lane's run, a bump, a local gate, the three other workflows |
| `adr.md` | the question, options, the eight decisions, the criteria mapped, consequences, the issues in order, open questions, the `.drover/arch.toml` lines |
| `packs.md` | each pack's gates, the fixtures that prove each fails, the budgets |
| `inventory.md` | each duplicated file and where it goes: a pack, a drover verb, a workflow, or stays local |
| `shapes.md` | the workflow inputs, the config, a gate, the plan and the report as types |
| `boundaries.md` | the repo layout, which module may call which, three statecharts |

## The design on one screen

```
 a consumer repo                            gatekeeper (one tag = workflows + program + packs)
 ───────────────                            ──────────
 .gatekeeper/config.toml   packs, pin ────▶ the pinned program, sha256 checked
 .gatekeeper/gates/*.toml  its own gates      │ plan: config + packs + changed files
 Cargo.toml, deny.toml …   stricter is free   │ run:  each gate, timed against its budget
                                              │   ├─ the repo's toolchain: cargo, tsc, swift
 a lane: gatekeeper check ───────┐            │   ├─ pinned tools: drover, actionlint, cargo-deny, oxlint, knip
         drover turn done ───────┼──────────▶ │   └─ built-in checks: floors, fixtures, tokens, pin
 CI:     ci.yml uses gates.yml ──┘            ▼
         plan ▶ a job per pack ▶ done       report: ok / FAIL / skip, seconds / budget, a fix line, a log file
```

| Today | After |
|---|---|
| The same gates hand-copied into every repo, drifting | One copy in gatekeeper; consumers hold a config file and their own gates |
| `just check` and `ci.yml` are two lists | `gatekeeper check` is the only list; CI calls it |
| drover, cargo-deny and actionlint installed a different way per repo | One pinned, checksummed version of each, fetched by the program |
| A gate fix lands N times | One PR in gatekeeper, then a one-command bump per repo |
| A private-names list inside public repos | The list stays in the fleet's config and a CI secret |
| No limit on CI time | A budget per gate, failed in CI when exceeded |

## What the owner decides

Approve, revise or reject the ADR, and answer its four open questions (`adr.md`, near the end). Three of them
touch things only the owner may change: a secret, the container rule for lanes, and an install on each host.
