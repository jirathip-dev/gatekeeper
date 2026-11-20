# gatekeeper: inventory of the duplicated files (#1)

Where each copy goes: a **pack** (gatekeeper holds the one copy), a **drover verb** (the concept is drover's), a
**workflow** (one of gatekeeper's four), or **stays local** (the repo's own, as a file in `.gatekeeper/gates/`).
Every consumer copy in the first three groups is deleted in that repo's adoption PR.

## Rust

| File today | Copies | Goes to |
|---|---|---|
| `clippy.toml` | 6 (5 identical, 1 reordered) | pack: `packs/rust/files/clippy.toml`, passed by reference; deleted everywhere |
| `[workspace.lints]` in `Cargo.toml` | every Rust repo | stays in `Cargo.toml`; `rust/bans` checks it against the pack's floor |
| `ci/lint-fixtures/rust/*.rs`, `ci/fixtures/banned/*.rs` | 4 | pack: `packs/rust/fixtures/bans/` |
| `scripts/lint-fixtures.sh` (82, 63, 26 lines), `scripts/banned.sh` | 4 | pack: the built-in fixture check behind `rust/bans` |
| `deny.toml` | 6 forks | stays local (targets, skips, banned crates are the repo's); `rust/deny` checks the floor |
| `rust-toolchain.toml`, `rustfmt.toml` | each repo | stays local |

## Core

| File today | Copies | Goes to |
|---|---|---|
| `scripts/line-cap.sh` and its test | 5 | drover verb: `drover cap-check` learns the counted globs and exclusions from `ci/cap.toml` (D1) |
| `ci/cap.toml`, `ci/cap.d/` | each repo | stays local: the cap is the repo's number |
| the `.drover/rules.md` 30-line `test` in each justfile | each repo | drover verb: `cap-check --rules` (it has this already) |
| `scripts/hygiene.sh` | 2 | drover verb: `drover hygiene check` (D2) |
| `ci/denylist.txt` | 2 | leaves the public repos: the names move to the fleet's `private_names` and the CI secret |
| `scripts/comment-rule.py`, `scripts/comments.py` and their tests | 2 | drover verb: arch-check's `[comments]` (it has this already); the open-issue check on `TODO(#N)` stays a local gate |
| `scripts/neutral-core.sh` and its test | 2 | stays local as data: a `[[ban]]` in the repo's `.drover/arch.toml`; the script goes |
| `scripts/dep-direction.sh` | 2 | stays local as data: `[[layer]]` entries in `.drover/arch.toml`; the script goes |
| `scripts/pr-body.sh` and its test | 1 | drover verb: `drover pr-body` (D3) |
| the pinned-drover download, toolchain, cargo-deny and actionlint steps in `ci.yml` | every repo | workflow `gates.yml` (provisioning) and the pinned tools the binary fetches |
| drover's reusable `arch-check.yml` | 1, called by cooee | deleted once cooee adopts: `core/arch` replaces it |
| `ci/gates.sh` and `ci/gates/*.sh` (cooee's own gate runner) | 1 | pack gates where one exists; the rest become `.gatekeeper/gates/*.toml`; the runner goes |
| the `check` recipe and the per-gate recipes in each justfile | every repo | one line: `check: ./.gatekeeper/run check` |

## Swift

| File today | Copies | Goes to |
|---|---|---|
| `tools/anti-slop-swift/` (19 shared rules; one copy has 5 more) | 2 | pack: `packs/swift/anti-slop/`; the generic extra rules join it, switched on per repo |
| the anti-slop wrapper and its validate scripts | 1 | pack: `swift/anti-slop` |
| `BLOCKS_STRICT`, `COOEE_STRICT`, `-warnings-as-errors` flags | 3 spellings | pack: `swift/warnings` and one flag, `GATEKEEPER_STRICT` |
| `ci/lint-fixtures/swift/{raw-colour,raw-size,stale,clean}.swift` and the Swift half of blocks' lint script | 1 | pack: `swift/tokens` and its fixtures |
| `ci/tokens/allowed.txt`, `ci/tokens/baseline.txt` | 1 | stays local, renamed: `.gatekeeper/tokens-allowed.txt`, `.gatekeeper/baseline/swift-tokens.txt` |
| `.swiftlint.yml` rules and their fixture test | 2 | stays local (a local gate) |
| the Swift format, coverage and nightly steps of an apple check script | 1 | stays local |

## TypeScript

| File today | Copies | Goes to |
|---|---|---|
| the hand-written TS ban lint (blocks) and its regex twin (cooee) | 2 | pack: `ts/oxlint` (the same bans as oxlint rules) |
| `web/lint-fixtures/*.ts` | 1 | pack: `packs/ts/fixtures/oxlint/` |
| the vendored anti-slop oxlint plugin, its runner, report and delta scripts | app repos | pack: `ts/oxlint`; the baseline becomes `.gatekeeper/baseline/ts-oxlint.txt` |
| the knip check script and its baseline | app repos | pack: `ts/knip`; `knip` config stays local |
| the strict-tsconfig check inside a ban script | 1 | pack: `ts/tsc` |
| `check-*.mjs` guards, CSS token and motion lint, React rules | app repos, blocks | stays local (local gates) |

## Supabase

| File today | Copies | Goes to |
|---|---|---|
| `scripts/apply-migrations.mjs`, `scripts/migration-status.mjs` and their tests | 2 forks | pack: `packs/supabase/scripts/`, one merged copy |
| the checkout-freshness and ledger-warning module | 1 | pack: inside the merged apply script |
| the static migration check (names, order, policy rule) | 1 | pack: `supabase/migrations` |
| the pgTAP workflow, one step a test file | 1 | pack: `supabase/pgtap` |
| `deploy-migrations.yml` | 2 forks | workflow: `supabase-deploy.yml` |
| new-migration helper, storage guards, chain replay, baseline equivalence | 1 | stays local |

## Release and TestFlight

| File today | Copies | Goes to |
|---|---|---|
| `scripts/release-sign.sh` (32 lines; only the identifier differs) | 3 | pack script, run by `gatekeeper release sign` |
| `release.yml` (about 120 lines each) | 3 | workflow: `release.yml`, which also signs `SHA256SUMS` and runs farrier's checker |
| the checks on a release's layout | none today | farrier: `farrier release verify`; gatekeeper only calls it |
| blocks' `release.yml` and `scripts/release.sh` (version tags, notes, no assets) | 1 | stays local: a different kind of release |
| `apple-testflight.yml` (167 and 137 lines) | 2 | workflow: `testflight.yml` |
| `scripts/asc-token.swift` (same code twice) | 2 | pack script of the workflow |
| a fastlane TestFlight workflow in one app repo | 1 | stays local until that repo's own adoption issue decides |
| a repo's self-hosted Mac runner setup action | 1 | stays local, named by `testflight.yml`'s `prepare` input |

## A monorepo's CI plumbing

| File today | Copies | Goes to |
|---|---|---|
| the scope script that picks affected projects, and its tests | 1 | engine: `scope.rs` and the graph readers (G10) |
| the shard matrix and the check that the shard reports cover every test | 1 | engine: `shards` on a gate; `done` fails a planned shard with no report (G10) |
| the aggregate job that turns many legs into one required check | 1 | workflow: `gates / done` |
| the database jobs that restore an earlier job's snapshot | 1 | stay local as gate files in legs; the hand-over is `[[upload]]` and `after` (G10) |
| per-job screenshot and report upload steps | several repos | a gate's `[[upload]]` (G4) |
| steps that build a tool from source on every run | 1 | issue #2: built once, fetched like any pinned tool |
| draft, label and tree-reuse rules of that workflow | 1 | stay the caller's: its `if:` and the `scope` and `packs` inputs |

## Scripts by destination, in short

- **drover verbs**: `cap-check` (extended), `arch-check` (as is), `lesson-check` (as is), `hygiene check` and
  `hygiene push` (new), `pr-body` (new).
- **gatekeeper**: the lint floor and fixtures, clippy config, the fixture runner, anti-slop for Swift and TS, the
  token lint, the tsconfig and deny floors, knip's ratchet, the migration check, the pgTAP runner, apply and
  status, release signing, the App Store Connect token, the four workflows, the wrapper, the selection of
  affected projects.
- **farrier**: what a release must look like, and the checker `release.yml` runs.
- **stays local**: caps and their history, `deny.toml`'s own entries, arch rules, tests, perf and soak budgets,
  app-specific guards, SwiftLint rules.
