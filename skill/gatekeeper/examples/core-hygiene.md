# core/hygiene: what a failure looks like

Each case is one change on the pack's fixture repo. `gatekeeper selftest` runs every case and fails unless the gate fails on it printing the line shown, so these examples cannot go stale.

## list-missing

With the environment:

```
DROVER_PRIVATE_NAMES=
```

The gate fails and its log prints `list-missing`.

Fix: take the name out where the hit points; CI passes the private-names secret, a private repo turns this gate off with a reason. Rerun: `./.gatekeeper/run check --gate core/hygiene`

## name-in-commit

Commits with the message:

```
tidy the notes

Asked by quillmoor.
```

The gate fails and its log prints `private-name #1 at commit`.

Fix: take the name out where the hit points; CI passes the private-names secret, a private repo turns this gate off with a reason. Rerun: `./.gatekeeper/run check --gate core/hygiene`

## name-in-pr-body

With the PR body (`{head}` is the case's commit, `{base}` its parent):

```
Closes #1

For the quillmoor rollout.
```

The gate fails and its log prints `private-name #1 at PR body:3`.

Fix: take the name out where the hit points; CI passes the private-names secret, a private repo turns this gate off with a reason. Rerun: `./.gatekeeper/run check --gate core/hygiene`

## name-in-tree

Writes `notes.md`:

```
Ship it to the Quillmoor team.
```

The gate fails and its log prints `private-name #1 at notes.md:1`.

Fix: take the name out where the hit points; CI passes the private-names secret, a private repo turns this gate off with a reason. Rerun: `./.gatekeeper/run check --gate core/hygiene`

## shared-names

Writes `lines.md`:

```
Ship it to the Quillmoor team.
quillmoors and requillmoor2 are other words.
See quillmoor_notes and v2-quillmoor/x.
The north-pier build.
Meet at Pier Nine.
A quill, then quillmoor.
A quill alone.
équillmoor is another word.
Nothing here.
```

Writes `names`:

```
quillmoor
north-pier
pier nine
quill
```

The gate fails and its log prints `private-name #1 at lines.md:1
private-name #1 at lines.md:3
private-name #2 at lines.md:4
private-name #3 at lines.md:5
private-name #1 at lines.md:6
private-name #4 at lines.md:7`.

Fix: take the name out where the hit points; CI passes the private-names secret, a private repo turns this gate off with a reason. Rerun: `./.gatekeeper/run check --gate core/hygiene`
