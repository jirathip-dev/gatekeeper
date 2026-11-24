# The core pack

the line cap, the rules file, arch-check, actionlint, the dependency list

## core/actionlint

pinned actionlint on .github/workflows/

- fix: fix the workflow at the line actionlint names
- budget: 5s
- cached: yes
- rerun: `./.gatekeeper/run check --gate core/actionlint`
- what a failure looks like: [core-actionlint](../examples/core-actionlint.md)

## core/arch

the layers, bans, sizes, frozen and generated files and comments in .drover/arch.toml

- fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file
- budget: 45s
- cached: yes
- rerun: `./.gatekeeper/run check --gate core/arch`
- what a failure looks like: [core-arch](../examples/core-arch.md)

## core/cap

the counted lines under the cap in ci/cap.toml or a raise in ci/cap.d; a commit that edits the cap carries its Cap: trailer

- fix: delete code; a raise of the cap is its own issue, its commit carries `Cap: <old> → <new> (#issue, q<N>)`
- budget: 5s
- cached: no, cache = false
- rerun: `./.gatekeeper/run check --gate core/cap`
- what a failure looks like: [core-cap](../examples/core-cap.md)

## core/deps

every direct dependency of Cargo.toml, package.json and Package.swift (with their workspaces) is listed with a reason in .gatekeeper/deps.toml; a PR grows Cargo.lock, pnpm-lock.yaml and Package.resolved by at most the file's `growth` packages

1. Crypto, TLS, signatures and parsers of files from outside are never written in-house: use a proven library.
2. Small glue with no hostile input is written in-house.
3. Tools (linters, checkers) are used as they are, pinned and checksummed, never forked.
4. Build frameworks are not added where gatekeeper already does the job.
5. To add a dependency: ask the owner first; then list it in .gatekeeper/deps.toml with its one-line reason; if the lockfiles grow by more than the file's `growth`, add `Deps: #<issue> <the owner's answer>` to a commit message, the way a cap raise carries its `Cap:` line. Lowering `growth` is free; a raise counts only from the next PR.

- fix: ask the owner before you add a dependency; then list it with its reason, drop a stale entry, and for growth over the limit put `Deps: #<issue> <the owner's answer>` in a commit message
- budget: 5s
- cached: no, cache = false
- rerun: `./.gatekeeper/run check --gate core/deps`
- what a failure looks like: [core-deps](../examples/core-deps.md)

## core/docs

docs and agent text name real paths, links and names; agent text is generated

- fix: make the doc true: correct the path, link or name, or rerun the generator; a listed hit you fixed leaves with ./.gatekeeper/run baseline core/docs
- budget: 10s
- cached: no, cache = false
- rerun: `./.gatekeeper/run check --gate core/docs`
- what a failure looks like: [core-docs](../examples/core-docs.md)

## core/hygiene

the tree, the PR's commit messages and its body against the private-names list

- fix: take the name out where the hit points; CI passes the private-names secret, a private repo turns this gate off with a reason
- budget: 5s
- cached: no, cache = false
- rerun: `./.gatekeeper/run check --gate core/hygiene`
- what a failure looks like: [core-hygiene](../examples/core-hygiene.md)

## core/pin

every gatekeeper workflow call names the commit and version of the pinned release, passes each secret by name, and .gatekeeper/run is that release's wrapper

- fix: run ./.gatekeeper/run pin <version>, never edit the pin by hand; pass each secret by name, never secrets: inherit
- budget: 1s
- cached: yes
- rerun: `./.gatekeeper/run check --gate core/pin`
- what a failure looks like: [core-pin](../examples/core-pin.md)

## core/pr-body

the PR body names the head, has `## For humans`, has no "Generated with" line

- fix: edit the PR body: name the pushed head sha, add a `## For humans` section, drop the "Generated with" line
- budget: 2s
- cached: no, cache = false
- rerun: `./.gatekeeper/run check --gate core/pr-body`
- what a failure looks like: [core-pr-body](../examples/core-pr-body.md)

## core/rules

every active check in .drover/rules.md passes, each rule names its enforcer, and the file is at most 30 lines; writes checks.json for drover's lesson lane

- fix: fix what the named check reports, delete the rules a check now enforces, or retire the check in a lesson PR
- budget: 5s
- cached: yes
- rerun: `./.gatekeeper/run check --gate core/rules`
- what a failure looks like: [core-rules](../examples/core-rules.md)
