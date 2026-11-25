# Code ratchets (#100)

## Summary

Decision: four ratchets from six new gates and the knip gate, which stays as it is. Dead code and complexity keep a list of named findings that only shrinks, as knip does today. Duplication and surface keep one number each, in the ceiling folders of the test health design (#99). A list or a number goes up only in a commit that carries the owner's decision. No new tool: the pinned oxlint has both complexity rules.
Slices: gatekeeper: a cap raise, dead code (with the gate that guards every list), duplication, complexity, surface, its own adoption and release 1.7.0. Then one adoption per repo, drover and shed first.
Open questions: (1) Dead code and complexity as named lists, not bare counts: swapping one dead item for a new one then fails. Agree? (2) May a surface count ever rise? Recommended: only with your decision in the commit, as a cap raise. (3) Approve one cap raise for gatekeeper, 11,700 to 12,700?

## Read in this order

| File | What |
|---|---|
| `usage.md` | how it reads: a repo's config, the adoption PR, a lane's failing runs, a raise with the owner's decision, a list check and a ceiling check in the engine |
| `adr.md` | the question, where the code stands, what is assumed of #99, eight options tables, the ten decisions against the owner's four, who writes which file, consequences, the slices, the open questions, the `arch.toml` lines |
| `shapes.md` | the two records, the config keys, the counted unit and list line of each check, how each tool's output is read, the `pub` scan's rules, the ceiling rules, what a change may do, the gate files, the pinned tools |
| `boundaries.md` | where the code goes beside `check/knip.rs` and `baseline.rs`, who may call whom, two statecharts, how each part is tested |

Read on gatekeeper staging 06e8d38 (1.5.0), drover staging 4cf52e4bf, shed faee4f3b and #99's ADR at
`origin/issue-99` 161323a, 2026-11-25.

## The design on one screen

```
 named findings ─▶ a list that only shrinks          a measure ─▶ a ceiling that only falls
 .gatekeeper/baseline/<gate>.txt (today's)           ci/ratchet/<count>.d/ (#99's folder)
 ───────────────────────────────────────────         ──────────────────────────────────────
 dead code                                           duplication
   ts/knip          as today                           core/duplication   jscpd: duplicated lines
   rust/dead-code   cargo-machete: unused deps                            clone >= 100 tokens and 10 lines
                    the scan: pub that no other      surface
                    crate names, that only tests       core/surface       names of the sets [surface] lists
                    name, a type nothing names                            (the docs gate's names list);
 complexity                                                               lines of the files it globs
   rust/complexity  clippy, forced through allows
                    > 60 lines, cognitive > 25
   ts/complexity    oxlint: > 60 lines, complexity > 20

 core/ratchet, against the base: no list grew, no ceiling file was edited or deleted, no higher one added,
 unless a commit carries  Ratchet: <record> <old> → <new> (#issue, q<N>)
```

| Owner decision | Where it lands |
|---|---|
| 1. four ratchets; a count only falls | the six gates and `ts/knip`; `core/ratchet` fails a change that moves a record up |
| 2. one ADR, #99's baselines, this order | the list and the ceiling folder, nothing new; slices R1 dead code, R2 duplication, R3 complexity, R4 surface |
| 3. knip, cargo-machete, jscpd; own `pub` scan; built-in complexity rules; own surface counts | `ts/knip`, `rust/dead-code`, `core/duplication`; clippy and the pinned oxlint; `[surface]` |
| 4. on with the release, at the current count | the bump to 1.7.0 runs `baseline <gate>` for each |

| | Rust | TypeScript | Swift |
|---|---|---|---|
| dead code | machete, the `pub` scan | knip | none |
| duplication | jscpd | jscpd | jscpd |
| complexity | clippy | oxlint | none |
| surface | by the repo's names list and globs, whatever the language | | |

## What it never does

- Store a baseline in a new format, or in a file every PR edits.
- Call code dead on a name match: the scan only asks for `pub(crate)`, and the compiler decides.
- Let an `allow` attribute, a jscpd ignore comment or a machete ignore list take something off a count.
- Let a repo loosen a threshold or the clone size.
- Read the GitHub API, or give a lane a different answer than CI.

## What the owner decides

Approve, revise or reject, and answer the three open questions in `adr.md`. Four effects worth a look, all
under "Consequences" there: drover starts with about 315 dead-code entries, nearly all of them live code that
is `pub` without need; tests count for duplication and complexity; two open PRs that both lower one ceiling
cost the second an update round; and from the bump on a hand-added line fails in every pack gate's list, docs
and knip included.
