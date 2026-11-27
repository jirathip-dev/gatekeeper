The code ratchets only fall: `rust/dead-code`, `rust/complexity`, `ts/complexity` and `ts/knip` keep a list in `.gatekeeper/baseline/<pack>-<gate>.txt`; `core/duplication` and `core/surface` keep a ceiling, one file per lowering in `ci/ratchet/<count>.d/<date>-<n>.toml`, the lowest wins.
`./.gatekeeper/run baseline <pack>/<gate>` drops a list's gone lines, or adds a lower ceiling file; a second run writes nothing.
`core/ratchet` fails a change that adds a list line, or edits, deletes or adds a ceiling file that is not lower, unless a commit carries `Ratchet: <pack/gate or count> <old> → <new> (#issue, q<N>)` with the owner's decision; a change that moves `[pin]` may add list lines.
`[rust.exported]` in `.gatekeeper/config.toml` names files whose `pub` items are used from outside the workspace, each `glob = "reason"`: `rust/dead-code` skips them.
`[duplication.ignore]` names more paths jscpd does not read, each `glob = "reason"`; fixtures, snapshots, vendored folders and the arch file's generated files are skipped already.
`[surface] names` lists sets of the `[docs.names]` list, each set's names one count; `[surface.lines]` maps a count's name to globs whose non-blank lines it counts. No `[surface]` table counts nothing.
No key changes a threshold, a clone's minimum size or a step: those are the packs'.
