A gate with `[select]` runs only the test files a change can reach, passed to its `run` line in place of `{files}`.
Its row prints `selected N of M test files in Xs:` with a count per rule (closure, always, readers, project-readers, widened), or `selected all M test files:` and why, then `list: target/gatekeeper/<pack>-<gate>.selection.json`.
Why did a test run, or not?
That file names each selected file with every rule that selected it: `closure` (vitest lists its imports reaching a changed file), `always`, `reads` (its code names a changed file's path or basename, or it walks that file's project or a directory above it), `names-project`, `widened` (a deleted file's project or import specifier).
Rerun a failing test with the same command: the same tree and base select the same files.
A selection that cannot be computed fails the gate with its `.select.log`; it never runs fewer tests.
A sharded `[select]` gate writes vitest's JSON report to `{report}`; each CI shard selects the same list and runs its slice, split by the times the last push to the integration branch recorded.
`ci done` fails a gate whose shards selected different lists (rerun all jobs), and a shard no file fell into prints `skip` and passes.
`./.gatekeeper/run replay --gate <pack>/<gate> --runs runs.jsonl --other '<cmd> {base}'` measures it against another selector: each line of `runs.jsonl` is `{"pr", "base", "head", "failed"}`, both select in a scratch worktree of the head, and it prints per run the counts, the files only one chose and each missed failure, then the totals, into `target/gatekeeper/replay/`; it exits 1 when gatekeeper missed a failure.
