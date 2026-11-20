# The core pack

the line cap, the rules file, arch-check, actionlint

## core/actionlint

pinned actionlint on .github/workflows/

- fix: fix the workflow at the line actionlint names
- budget: 5s
- rerun: `./.gatekeeper/run check --gate core/actionlint`
- what a failure looks like: [core-actionlint](../examples/core-actionlint.md)

## core/arch

drover arch-check: the layers, bans, sizes, frozen and generated files and comments in .drover/arch.toml

- fix: follow the rule the finding names; rerun the generator, never hand-edit a generated file
- budget: 45s
- rerun: `./.gatekeeper/run check --gate core/arch`
- what a failure looks like: [core-arch](../examples/core-arch.md)

## core/cap

drover cap-check: the counted lines under the cap in ci/cap.toml, and .drover/rules.md at most 30 lines

- fix: delete code, or the rules a check now enforces; a raise of the cap is its own issue
- budget: 5s
- rerun: `./.gatekeeper/run check --gate core/cap`
- what a failure looks like: [core-cap](../examples/core-cap.md)

## core/pin

every gatekeeper workflow call names the commit and version of the pinned release, passes each secret by name, and .gatekeeper/run is that release's wrapper

- fix: run ./.gatekeeper/run pin <version>, never edit the pin by hand; pass each secret by name, never secrets: inherit
- budget: 1s
- rerun: `./.gatekeeper/run check --gate core/pin`
- what a failure looks like: [core-pin](../examples/core-pin.md)

## core/rules

drover lesson-check: every active check in .drover/rules.md passes

- fix: fix what the named check reports, or retire the check in a lesson PR
- budget: 5s
- rerun: `./.gatekeeper/run check --gate core/rules`
- what a failure looks like: [core-rules](../examples/core-rules.md)
