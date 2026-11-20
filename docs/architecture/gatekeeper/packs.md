# gatekeeper: the packs (#1)

Each gate lists what it runs, the fixture cases that prove it fails, and its budget in seconds. A budget is the
gate's own wall time. It fails a gate only on a GitHub-hosted runner with a warm cache; a self-hosted runner, a
cold cache and a lane's host report the time and pass. Setup (checkout, toolchain, cache restore, the fetch) is
not in a gate's time: it is measured per job and shown in the same table, with no budget.

The numbers start from today's CI (clippy 3 s in swag, 18 s in blocks, 19 s in shed; arch-check 8 s in shed, 34 s
in blocks where it reruns the generators) with room for the largest repo. A repo may set its own in `[budgets]`.

Every gate also has a `clean` case that must pass. `selftest` fails a gate with no failing case.

## core (every repo; linux; 68 s)

| Gate | Runs | Fixture cases that must fail | Budget |
|---|---|---|---|
| `core/pin` | every `uses: …/gatekeeper/…@<ref> # <tag>`: the ref is the 40-hex commit the running binary was built from, the comment is `v<pin.version>`, the call has no `secrets: inherit`; `.gatekeeper/run` equals the pinned version's wrapper | `sha-differs`, `ref-is-a-tag`, `comment-differs`, `secrets-inherit`, `wrapper-edited` | 1 |
| `core/cap` | `drover cap-check`: the line cap in `ci/cap.toml`, and `.drover/rules.md` at most 30 lines | `over-cap`, `cap-missing`, `rules-31-lines` | 5 |
| `core/rules` | `drover lesson-check`: the rules file's format and its active checks | `no-enforcer`, `active-check-fails` | 5 |
| `core/arch` | `drover arch-check` (with `--base` and `--pr-body` on a pull request) | `layer-crossed`, `ban-hit`, `file-too-long`, `frozen-line-added`, `generated-stale`, `comment-added` | 45 |
| `core/hygiene` | `drover hygiene check`: the tree, the PR's commit messages and its body against the private-names list | `name-in-tree`, `name-in-commit`, `name-in-pr-body`, `list-missing` | 5 |
| `core/pr-body` | `drover pr-body`: the body names the head, has `## For humans`, has no "Generated with" line | `no-head-sha`, `no-for-humans`, `generated-with` | 2 |
| `core/actionlint` | pinned actionlint on `.github/workflows/**` | `bad-workflow` | 5 |

Hygiene: the list never enters the repo. A lane's host reads the fleet's `private_names` (drover's config); CI
reads the `private-names` secret, which `drover hygiene push` writes from the same list. A hit prints
`private-name #N at file:line`, never the name. A private repo skips the gate. A public repo's same-repo PR with no
list fails (`list-missing`); a fork's PR has no secrets, so there the gate is skipped and runs on the merge push.

## rust (linux; 150 s)

| Gate | Runs | Fixture cases that must fail | Budget |
|---|---|---|---|
| `rust/fmt` | `cargo fmt --check` | `unformatted` | 10 |
| `rust/clippy` | clippy pedantic, `-D warnings`, with the pack's `clippy.toml` by reference; lib and bins strict, tests with the test allow-list | `pedantic-warning`, `rustc-warning` | 90 |
| `rust/bans` | the 12 clippy denies and `unsafe_code = "forbid"` are in `[workspace.lints]`, none weakened; then each banned-API file is refused by clippy under the repo's own lints and toolchain | `floor-missing`, `floor-allowed`, `pedantic-off` | 30 |
| `rust/deny` | `deny.toml` holds the floor (yanked, multiple versions, wildcards, unknown registries and git sources all `deny`; licences from the shared allow-list), then pinned `cargo deny --locked check` | `yanked-allowed`, `licence-outside-list`, `wildcard-dependency`, `unknown-git-source` | 20 |

`rust/bans` carries the 13 banned-API files that blocks, drover, swag and shed each keep today
(`unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `dbg_macro`, `indexing_slicing`, `as_conversions`,
`print_stdout`, `print_stderr`, `allow_attributes_without_reason`, `let_underscore_must_use`, `unsafe_code`). They
run in one scratch crate in one clippy call.

Stays the repo's: `rust-toolchain.toml`, `rustfmt.toml`, the crates `deny.toml` bans or skips, any pedantic lint
it allows, its tests (a local gate).

## swift (macOS; 400 s)

| Gate | Runs | Fixture cases that must fail | Budget |
|---|---|---|---|
| `swift/anti-slop` | the one anti-slop-swift (19 rules), a prebuilt tool, over `[swift] sources` | one per rule (19) | 30 |
| `swift/warnings` | each package builds with tests under `GATEKEEPER_STRICT=1` (its `Package.swift` treats all warnings as errors under that flag); each Xcode project builds with warnings as errors | `warning`, `strict-flag-missing` | 360 |
| `swift/tokens` | no raw colour or raw size in the `tokens` folders, but the allow-list and the shrink-only baseline | `raw-colour`, `raw-size`, `stale` | 10 |

The tool ships prebuilt with each gatekeeper release, so no run compiles swift-syntax. `swift/tokens` is a
built-in check and runs on any platform. A repo's SwiftLint rules stay a local gate in v1.

## ts (linux; 180 s)

| Gate | Runs | Fixture cases that must fail | Budget |
|---|---|---|---|
| `ts/tsc` | each project's tsconfig has `strict` on and no strict flag switched off; then `tsc --noEmit -p` | `strict-off`, `type-error` | 90 |
| `ts/oxlint` | pinned oxlint with the pack's config by reference: the bans and the 15 anti-slop rules | `as-cast`, `non-null`, `any`, `ts-ignore`, `ts-expect-error`, `inner-html`, `outer-html`, `insert-adjacent-html`, `eval`, `new-function`, `console-log`, `default-export`, `enum`, and one per anti-slop rule (15) | 30 |
| `ts/knip` | pinned knip: no unused file, export or dependency beyond the baseline | `unused-export`, `unused-dependency`, `stale` | 60 |

The ban cases are blocks' TS lint fixtures; its own lint script and the regex copy in cooee go. A repo's extra
oxlint or ESLint rules, its CSS token lint and its `check-*.mjs` guards stay local gates.

## supabase (linux; 10 s, and 300 s in CI for pgTAP)

| Gate | Runs | Fixture cases that must fail | Budget |
|---|---|---|---|
| `supabase/migrations` | static, no database: file names and timestamps, a new file is newer than the base's newest, a policy calls `(select auth.uid())` | `bad-name`, `older-than-base`, `bare-auth-uid` | 10 |
| `supabase/pgtap` | a local stack, a reset, then every file under `supabase/tests/` (found by glob; no per-file CI step to forget) | `failing-test`, `test-file-not-run` | 300 |

`supabase/pgtap` needs containers, so it is CI-only on a lane's host (the owner's decision). The apply and status
scripts are the union of today's two forks (transactional apply, the checkout-freshness guard, the ledger
warnings), run by `gatekeeper supabase …` and by `supabase-deploy.yml`, which runs `supabase/migrations` first.
Their own tests run in gatekeeper's CI. The storage-policy guards, the full replay and the baseline-equivalence
check stay local.

## Shared workflows

| Workflow | Its proof in gatekeeper's CI |
|---|---|
| `release.yml` | gatekeeper's own releases are cut by it. A dry run on every PR (`publish: false`) builds both targets, writes `SHA256SUMS`, signs it with a throwaway key and runs farrier's checker; then it changes one checksum line and the checker must refuse. The must-fail release folders themselves are farrier's fixtures |
| `testflight.yml` | actionlint, and an `upload: false` run in its first adopter before any other repo switches |
| `supabase-deploy.yml` | actionlint, and the scripts' tests against a mocked ledger API |

## The plan's own cases

Not gates, so they are test cases of the engine, one folder each under `crates/gatekeeper-engine/tests/`:

| What | Cases |
|---|---|
| kept files (G4) | `kept-on-pass`, `kept-on-fail`, `glob-matches-nothing` (reported, not a failure) |
| budget mode (G1) | `over-budget-hosted-warm` fails; `over-budget-self-hosted`, `over-budget-cold`, `over-budget-lane` pass as `slow` |
| affected projects (G10) | `leaf-changed`, `dependency-changed`, `file-outside-every-project` (widens), `ignored-file`, `no-base` (widens), `forced-full` |
| shards and legs (G10) | `shard-report-missing` fails `done`; `lane-plans-one-shard`; `after-leg-waits`; `after-leg-skipped-when-first-fails` |

## The engine's own budget

`gatekeeper check` on the `_base` fixture, every gate stubbed, takes under 200 ms: a hard assert in a perf test,
the way drover's and blocks' perf budgets are asserted.
