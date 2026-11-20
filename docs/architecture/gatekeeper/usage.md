# gatekeeper: how the callers read (#1)

Read this first. These are the call sites after adoption; `adr.md` says why, `shapes.md` has the types. Every name
here is new and exists only once its issue lands.

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
    uses: jirathip-dev/gatekeeper/.github/workflows/gates.yml@v1.0.0
    secrets:
      private-names: ${{ secrets.GATEKEEPER_PRIVATE_NAMES }}
```

Branch protection needs one check: `gates / done`. Its name never changes, whatever the packs.

## The repo's one config file

```toml
# .gatekeeper/config.toml
packs = ["core", "rust"]
integration = "staging"

[pin]
version = "1.0.0"
x86_64-unknown-linux-gnu = "9f2c…"   # sha256 of the release asset
aarch64-apple-darwin = "41ab…"
```

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
    gatekeeper check
```

## A lane: one command, the same as CI

```
$ gatekeeper check
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
      log: target/gatekeeper/rust-clippy.log   rerun: gatekeeper check --gate rust/clippy
ok    rust/bans         4.9s /  30s   13 fixtures refused
ok    rust/deny         1.7s /  20s
ok    local/test        5.3s /  60s
1 failed: rust/clippy
```

Every gate runs even after one fails, so one run shows every failure. Exit 1 on any `FAIL`. Full logs go to files,
never to the lane's context.

`drover turn done` runs the same command: drover's default done check becomes `gatekeeper check` wherever a repo
has `.gatekeeper/config.toml`. No fleet sets it by hand.

The host's `gatekeeper` reads `[pin]`. If it is another version, it downloads the pinned one once, checks its
sha256, keeps it in its cache and runs that. Two repos on two pins share one host.

## Bumping the pin

```
$ gatekeeper pin 1.1.0
.gatekeeper/config.toml: 1.0.0 -> 1.1.0 (2 checksums)
.github/workflows/ci.yml: gates.yml@v1.0.0 -> @v1.1.0
$ gatekeeper check            # the new gates run here first, then the same in this PR's CI
```

A bump is one small PR in the consumer. A stricter gate can turn a repo red only in that PR.

## A gate nothing else has: add a file

```toml
# blocks: .gatekeeper/gates/perf.toml
about = "release perf budgets (crates/blocks/tests/perf.rs)"
run = "cargo test --release --locked -p blocks --test perf -- --ignored --test-threads=1"
budget_secs = 150
```

It runs in `gatekeeper check` and in CI from that commit. No workflow edit, no justfile edit.

## Old findings: a baseline that only shrinks

```
$ gatekeeper baseline ts/oxlint
.gatekeeper/baseline/ts-oxlint.txt: 3369 findings in 417 files
```

A listed finding passes, a new one fails, and a fixed one must leave the list (`stale: shrink the baseline`).
After adoption the command only removes lines.

## Usage for lanes: the skill

```
$ gatekeeper skill --to <the lane's skills folder>
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
    uses: jirathip-dev/gatekeeper/.github/workflows/release.yml@v1.0.0
    with:
      bin: shed
      sha: ${{ github.event.workflow_run.head_sha }}
      branch: ${{ github.event.workflow_run.head_branch }}
      version-check: true
    secrets: inherit
```

The tags (`build-<sha7>`, `build-staging-<sha7>`) and asset names (`<bin>-<target>`, `SHA256SUMS`) stay as they
are, so each tool's self-update keeps working.

## TestFlight

```yaml
jobs:
  testflight:
    uses: jirathip-dev/gatekeeper/.github/workflows/testflight.yml@v1.0.0
    with:
      project: app/Beacon.xcodeproj
      scheme: Beacon
      platforms: '["ios"]'
      upload: ${{ github.ref == 'refs/heads/main' }}
    secrets: inherit
```

## Supabase deploy

```yaml
jobs:
  deploy:
    uses: jirathip-dev/gatekeeper/.github/workflows/supabase-deploy.yml@v1.0.0
    with:
      target: ${{ github.ref == 'refs/heads/main' && 'prod' || 'dev' }}
    secrets: inherit
```

By hand, with the same code: `gatekeeper supabase status` and `gatekeeper supabase apply --target dev`.

## gatekeeper gates itself

```yaml
# .github/workflows/ci.yml in gatekeeper
jobs:
  gates:
    uses: ./.github/workflows/gates.yml        # this commit's workflow, not a tag
    secrets:
      private-names: ${{ secrets.GATEKEEPER_PRIVATE_NAMES }}
```

```toml
# .gatekeeper/config.toml in gatekeeper
packs = ["core", "rust"]
integration = "staging"
pin = "checkout"          # build this commit's gatekeeper and gate the repo with it
```

Its local gates are `test` (cargo test) and `selftest` (`gatekeeper selftest`: every gate of every pack fails each
of its fixtures and passes its clean one).
