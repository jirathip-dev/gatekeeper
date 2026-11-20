import { readFileSync } from "node:fs";
import { basename, join } from "node:path";
import { parseArgs } from "node:util";
import { DIR, bareAuthUid, git, local, parse } from "./migrations.mjs";

const root = process.cwd();
const { base } = parseArgs({ options: { base: { type: "string" } } }).values;
const files = local(root);
const problems = files.filter((f) => f.error).map((f) => f.error);
const seen = new Map();
for (const f of files.filter((f) => f.version)) {
  if (seen.has(f.version)) {
    problems.push(`${DIR}/${f.file}: duplicate-version: ${f.version} is also ${seen.get(f.version)}`);
  }
  seen.set(f.version, f.file);
}
if (base) {
  const before = new Set(git(root, ["ls-tree", "--name-only", base, `${DIR}/`]).split("\n").filter(Boolean).map((p) => basename(p)));
  const newest = [...before].map(parse).map((f) => f.version ?? "").sort().at(-1) ?? "";
  for (const f of files.filter((f) => f.version && !before.has(f.file))) {
    if (f.version <= newest) {
      problems.push(`${DIR}/${f.file}: older-than-base: not newer than the base's newest migration ${newest}`);
    }
    if (bareAuthUid(readFileSync(join(root, DIR, f.file), "utf8"))) {
      problems.push(`${DIR}/${f.file}: bare-auth-uid: a policy calls auth.uid() bare; write (select auth.uid())`);
    }
  }
}
for (const p of problems) {
  process.stdout.write(`${p}\n`);
}
process.stdout.write(`${files.length} migrations, ${problems.length} problems${base ? "" : " (no base: names only)"}\n`);
process.exit(problems.length ? 1 : 0);
