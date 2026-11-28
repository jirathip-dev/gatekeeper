# core/surface: what a failure looks like

Each case is one change on the pack's fixture repo. `gatekeeper selftest` runs every case and fails unless the gate fails on it printing the line shown, so these examples cannot go stale.

## line-added

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[docs.names]
run = "sh names.sh"

[surface.lines]
agent = ["skill/**/*.md"]
```

Writes `ci/ratchet/surface-agent.d/2026-11-27-3.toml`:

```
ceiling = 3
```

Writes `skill/tool/SKILL.md`:

```
# Tool

Run it.

Then check it.
Then ship it.
```

The gate fails and its log prints `surface-agent: 4 lines, over the ceiling 3`.

Fix: remove the new name or line, or one old one; a count a whole step under its ceiling turns the ratchet: ./.gatekeeper/run baseline core/surface adds a lower file. A raise needs the owner's decision in a `Ratchet: surface-<count> <old> → <new> (#issue, q<N>)` trailer. Rerun: `./.gatekeeper/run check --gate core/surface`

## name-added

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[docs.names]
run = "sh names.sh"

[surface]
names = ["flag"]
```

Writes `ci/ratchet/surface-flag.d/2026-11-27-2.toml`:

```
ceiling = 2
```

Writes `names.sh`:

```
printf 'flag\t--dry-run\nflag\t--json\nflag\t--force\nverb\tspawn\n'
```

The gate fails and its log prints `surface-flag: 3 names, over the ceiling 2`.

Fix: remove the new name or line, or one old one; a count a whole step under its ceiling turns the ratchet: ./.gatekeeper/run baseline core/surface adds a lower file. A raise needs the owner's decision in a `Ratchet: surface-<count> <old> → <new> (#issue, q<N>)` trailer. Rerun: `./.gatekeeper/run check --gate core/surface`

## no-table

The gate passes.

## one-added-one-removed

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[docs.names]
run = "sh names.sh"

[surface]
names = ["flag"]
```

Writes `ci/ratchet/surface-flag.d/2026-11-27-2.toml`:

```
ceiling = 2
```

Writes `names.sh`:

```
printf 'flag\t--dry-run\nflag\t--force\n'
```

The gate passes.

## removed-and-not-lowered

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[docs.names]
run = "sh names.sh"

[surface]
names = ["flag"]
```

Writes `ci/ratchet/surface-flag.d/2026-11-27-3.toml`:

```
ceiling = 3
```

Writes `names.sh`:

```
printf 'flag\t--dry-run\nflag\t--json\n'
```

The gate fails and its log prints `surface-flag: 2 names, a step under the ceiling 3: ./.gatekeeper/run baseline core/surface`.

Fix: remove the new name or line, or one old one; a count a whole step under its ceiling turns the ratchet: ./.gatekeeper/run baseline core/surface adds a lower file. A raise needs the owner's decision in a `Ratchet: surface-<count> <old> → <new> (#issue, q<N>)` trailer. Rerun: `./.gatekeeper/run check --gate core/surface`

## set-not-in-the-list

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[docs.names]
run = "sh names.sh"

[surface]
names = ["refusal"]
```

Writes `names.sh`:

```
printf 'flag\t--dry-run\n'
```

The gate fails and its log prints `surface-refusal: the names list prints no `refusal` name`.

Fix: remove the new name or line, or one old one; a count a whole step under its ceiling turns the ratchet: ./.gatekeeper/run baseline core/surface adds a lower file. A raise needs the owner's decision in a `Ratchet: surface-<count> <old> → <new> (#issue, q<N>)` trailer. Rerun: `./.gatekeeper/run check --gate core/surface`
