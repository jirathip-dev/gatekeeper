With `[cache]` in `.gatekeeper/config.toml` a gate whose inputs already passed prints `cached` with the run that proved it.
A rerun of a cached pass that fails now prints `cached pass does not hold`: the gate reads a file it does not declare; add it to the gate's `paths` or set `cache = false`.
`--no-cache` runs and records nothing.
A `projects` gate keeps one record per project, over that project, every project it needs and every file outside all projects that `[scope] ignore` keeps: a change reruns the projects it reaches, and the row says how many ran and how many were cached.
A sharded CI job keeps one record for its whole set of projects and is cached only while that set and its files are unchanged.
