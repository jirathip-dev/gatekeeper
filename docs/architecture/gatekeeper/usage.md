# gatekeeper: how the callers read (#1)

Read this first. These are the call sites after adoption; `adr.md` says why, `shapes.md` has the types. Every name
here is new and exists only once its issue lands. Commit shas and checksums are shortened with `…`; the real ones
are whole.

## A consumer's `ci.yml`: before and after

Before (swag today, 55 lines; the other repos carry the same steps plus their own):

```yaml
env:
  DROVER_BUILD: build-staging-9ea69d9
  DROVER_SHA256: 39a24079…
jobs:
  gates:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with: { fetch-depth: 0 }
      - uses: dtolnay/rust-toolchain@1.99.1
        with: { components: "rustfmt, clippy" }
      - uses: Swatinem/rust-cache@v2
      - uses: taiki-e/install-action@v2
        with: { tool: "just,cargo-deny" }
      - name: drover (pinned release build)
        run: |
          curl -sSfL -o "$RUNNER_TEMP/drover" "https://github.com/jirathip-dev/drover/releases/download/$DROVER_BUILD/…"
          echo "$DROVER_SHA256  $RUNNER_TEMP/drover" | sha256sum -c -
          # … chmod, export DROVER
      - run: just fmt
      - run: just clippy
      - run: just fixtures
      - run: just test
      - run: just deny
      - run: just arch          # with BASE=HEAD^1 on a pull request
      - run: just cap
      - name: actionlint
        run: |
          bash <(curl -sSfL https://raw.githubusercontent.com/rhysd/actionlint/v1.7.12/scripts/download-actionlint.bash) …
          just actionlint
```

After (the whole file):

```yaml
name: ci
on:
  push:
    branches: [main, staging]
  pull_request:

permissions:
  contents: read

jobs:
  gates:
    uses: jirathip-dev/gatekeeper/.github/workflows/gates.yml@3f9c2ab8…e8f9 # v1.0.0
    secrets:
      private-names: ${{ secrets.GATEKEEPER_PRIVATE_NAMES }}
```

The workflow is named by its commit; the tag is a comment for the reader. Each secret is passed by name;
`secrets: inherit` is refused. Branch protection needs one check: `gates / done`. Its name never changes, whatever
the packs.

## What the repo holds

```toml
# .gatekeeper/config.toml
packs = ["core", "rust"]
integration = "staging"

[pin]
version = "1.0.0"
x86_64-unknown-linux-gnu = "9f2c…"   # sha256 of the release asset
aarch64-apple-darwin = "41ab…"
```

`.gatekeeper/run` is the wrapper: about 30 lines of shell, the same bytes in every repo on one version. It reads
`[pin]`, looks in the host's cache for the binary with that sha256, downloads it from the release if it is not
there, checks the sha256 and runs it. A lane's host and a CI runner both start here, so neither has gatekeeper
installed.

The repo's own gates are one file each. This is swag's test step:

```toml
# .gatekeeper/gates/test.toml
about = "every scheme passes blocks' gate, every pack check_pack"
run = "cargo test --all-features --locked"
budget_secs = 60
paths = ["src/**", "tests/**", "Cargo.*", "schemes/**", "packs/**"]
```

Deleted from swag in the same PR: `clippy.toml`, `ci/lint-fixtures/`, `scripts/lint-fixtures.sh`,
`scripts/line-cap.sh`, `scripts/line-cap.test.sh`, and every justfile recipe but one:

```just
check:
    ./.gatekeeper/run check
```

## A lane: one command, the same as CI

```
$ ./.gatekeeper/run check
gatekeeper 1.0.0 · packs core rust · base origin/staging · 3 files changed
ok    core/pin          0.0s /   1s
ok    core/cap          0.4s /   5s   412 / cap 500
ok    core/arch         2.1s /  45s
ok    core/hygiene      0.3s /   5s
skip  core/pr-body                    no pull request yet
ok    core/actionlint   0.6s /   5s
ok    rust/fmt          0.8s /  10s
FAIL  rust/clippy       6.2s /  90s
      src/pack.rs:41:17: used `unwrap()` on an `Option` value (clippy::unwrap_used)
      fix: return the error; only tests may unwrap
      log: target/gatekeeper/rust-clippy.log   rerun: ./.gatekeeper/run check --gate rust/clippy
ok    rust/bans         4.9s /  30s   13 fixtures refused
ok    rust/deny         1.7s /  20s
ok    local/test        5.3s /  60s
1 failed: rust/clippy
```

Every gate runs even after one fails, so one run shows every failure. Exit 1 on any `FAIL`. Full logs go to files,
never to the lane's context. On a lane's host a gate over its budget is marked `slow` and still passes.

`drover turn done` runs the same command: drover's default done check becomes `.gatekeeper/run check` wherever a
repo has that file. No fleet sets it by hand.

The first run of a version on a host prints one more line, `fetched gatekeeper 1.0.0 (sha256 ok)`. Two repos on
two pins share one host: the cache is keyed by the binary's sha256.

## Bumping the pin

```
$ ./.gatekeeper/run pin 1.1.0
release v1.1.0: signature ok, commit 8d2e41c0…
.gatekeeper/config.toml: 1.0.0 -> 1.1.0 (2 checksums)
.github/workflows/ci.yml: gates.yml@3f9c2ab8… # v1.0.0 -> @8d2e41c0… # v1.1.0
.gatekeeper/run: unchanged
$ ./.gatekeeper/run check     # the new gates run here first, then the same in this PR's CI
```

A bump is one small PR in the consumer. A stricter gate can turn a repo red only in that PR. `core/pin` fails a
`uses:` line whose sha is not the commit the pinned binary was built from, whose comment is not the pinned
version, or that carries `secrets: inherit`; it also fails an edited wrapper.

The first pin in a repo is pasted from its adoption issue (the wrapper, `[pin]`, the `uses:` line). `pin` then
checks all three against the signed release and rewrites them; after that only `pin` touches them.

## A gate nothing else has: add a file

```toml
# blocks: .gatekeeper/gates/perf.toml
about = "release perf budgets (crates/blocks/tests/perf.rs)"
run = "cargo test --release --locked -p blocks --test perf -- --ignored --test-threads=1"
budget_secs = 150
```

It runs in `.gatekeeper/run check` and in CI from that commit. No workflow edit, no justfile edit.

## Files a gate keeps: screenshots

```toml
# .gatekeeper/gates/ui.toml
about = "every screen renders at 1440 and 390 with no layout finding"
run = "just ui"
budget_secs = 120
paths = ["web/**"]

[[upload]]
name = "ui-390"
paths = ["target/ui/390/*.png"]
```

```
ok    local/ui         41.0s / 120s
      files: ui-390 (6) target/ui/390/
```

In CI the same six files are in the job's artifact, `gates-local`, under `files/ui-390/`, beside `report.json` and
the logs. They are kept whether the gate passed or failed.

## Old findings: a baseline that only shrinks

```
$ ./.gatekeeper/run baseline ts/oxlint
.gatekeeper/baseline/ts-oxlint.txt: 3369 findings in 417 files
```

A listed finding passes, a new one fails, and a fixed one must leave the list (`stale: shrink the baseline`).
After adoption the command only removes lines.

## A large monorepo (after G10)

```toml
# .gatekeeper/config.toml
packs = ["core", "ts", "supabase"]
integration = "staging"

[scope]
graphs = ["pnpm"]
ignore = ["docs/**", "**/*.md"]

[leg.db-replay]
needs = ["containers"]

[leg.db-probes]
needs = ["containers"]
after = "db-replay"
```

```toml
# .gatekeeper/gates/unit.toml
about = "unit tests of the projects this change can reach"
run = "pnpm vitest run --shard={shard} {projects}"
projects = ["apps/*", "packages/*"]
shards = { hosted = 4, self-hosted = 2 }
budget_secs = 300
```

```toml
# .gatekeeper/gates/db-replay.toml
about = "every migration replays on an empty database"
run = "node scripts/replay.mjs --snapshot target/db/snapshot.dump"
paths = ["supabase/**"]
leg = "db-replay"
budget_secs = 600

[[upload]]
name = "db-snapshot"
paths = ["target/db/snapshot.dump"]
```

- A pull request that touches one app runs `unit` over that app and every project that needs it. A change to the
  lockfile, a root config or `.gatekeeper/` reaches everything. A push to the integration branch, a schedule and
  `scope: full` (the caller sets it from a label, say) always run everything.
- In CI `unit` is four jobs, `1/4` to `4/4`. `done` fails when a planned shard has no report. On a lane's host
  it is one run, `1/1`, over the same tests.
- The gates of leg `db-probes` start after `db-replay` passed, with `db-snapshot` already in place. Both legs
  need containers, so a lane's host prints them as CI-only.

Until G10 this repo keeps its own workflow and adds a second, small one:

```yaml
# .github/workflows/gates.yml, beside the repo's own ci.yml
jobs:
  gates:
    uses: jirathip-dev/gatekeeper/.github/workflows/gates.yml@3f9c2ab8…e8f9 # v1.0.0
    secrets:
      private-names: ${{ secrets.GATEKEEPER_PRIVATE_NAMES }}
```

Both `gates / done` and its own aggregate check are required, and its `ci.yml` is not frozen yet.

## Usage for lanes: the skill

```
$ ./.gatekeeper/run skill --to <the lane's skills folder>
wrote gatekeeper/SKILL.md, gatekeeper/packs/core.md, gatekeeper/packs/rust.md, gatekeeper/examples/ (4)
```

The skill is generated from the gate files of the pinned version: each gate's `about`, its `fix` line and how to
rerun it. A lane reads that, not gatekeeper's source.

## Release

```yaml
# .github/workflows/release.yml in shed (today 118 lines plus a 32-line signing script)
name: release
on:
  workflow_run:
    workflows: [ci]
    types: [completed]
    branches: [main, staging]

jobs:
  release:
    if: github.event.workflow_run.conclusion == 'success' && github.event.workflow_run.event == 'push'
    uses: jirathip-dev/gatekeeper/.github/workflows/release.yml@3f9c2ab8…e8f9 # v1.0.0
    with:
      bin: shed
      sha: ${{ github.event.workflow_run.head_sha }}
      branch: ${{ github.event.workflow_run.head_branch }}
      version-check: true
    secrets:
      MACOS_CERT_P12: ${{ secrets.MACOS_CERT_P12 }}
      MACOS_CERT_PASSWORD: ${{ secrets.MACOS_CERT_PASSWORD }}
      ASC_API_KEY_P8: ${{ secrets.ASC_API_KEY_P8 }}
      ASC_API_KEY_ID: ${{ secrets.ASC_API_KEY_ID }}
      ASC_API_ISSUER_ID: ${{ secrets.ASC_API_ISSUER_ID }}
      RELEASE_SIGNING_KEY: ${{ secrets.RELEASE_SIGNING_KEY }}
```

The tags (`build-<sha7>`, `build-staging-<sha7>`) and asset names (`<bin>-<target>`, `SHA256SUMS`) stay as they
are, so each tool's self-update keeps working. New: `SHA256SUMS.minisig`, and farrier's checker passes before the
release exists. The repo adds `release.pub`, the public half of the signing key.

## TestFlight

```yaml
jobs:
  testflight:
    uses: jirathip-dev/gatekeeper/.github/workflows/testflight.yml@3f9c2ab8…e8f9 # v1.0.0
    with:
      project: app/Beacon.xcodeproj
      scheme: Beacon
      platforms: '["ios"]'
      upload: ${{ github.ref == 'refs/heads/main' }}
    secrets:
      ASC_KEY_P8: ${{ secrets.ASC_KEY_P8 }}
      ASC_KEY_ID: ${{ secrets.ASC_KEY_ID }}
      ASC_ISSUER_ID: ${{ secrets.ASC_ISSUER_ID }}
```

## Supabase deploy

```yaml
jobs:
  deploy:
    uses: jirathip-dev/gatekeeper/.github/workflows/supabase-deploy.yml@3f9c2ab8…e8f9 # v1.0.0
    with:
      target: ${{ github.ref == 'refs/heads/main' && 'prod' || 'dev' }}
    secrets:
      SUPABASE_ACCESS_TOKEN: ${{ secrets.SUPABASE_ACCESS_TOKEN }}
```

By hand, with the same code: `./.gatekeeper/run supabase status` and `./.gatekeeper/run supabase apply --target dev`.

## gatekeeper gates itself

```yaml
# .github/workflows/ci.yml in gatekeeper
jobs:
  gates:
    uses: ./.github/workflows/gates.yml        # this commit's workflow, not a release
    secrets:
      private-names: ${{ secrets.GATEKEEPER_PRIVATE_NAMES }}
  release-dry:
    uses: ./.github/workflows/release.yml      # builds, lists, signs with a throwaway key, checks; publishes nothing
    with:
      bin: gatekeeper
      publish: false
```

```toml
# .gatekeeper/config.toml in gatekeeper
packs = ["core", "rust"]
integration = "staging"
pin = "checkout"          # the wrapper builds this commit's gatekeeper and gates the repo with it
```

Its local gates are `test` (cargo test) and `selftest` (`gatekeeper selftest`: every gate of every pack fails each
of its fixtures and passes its clean one).
