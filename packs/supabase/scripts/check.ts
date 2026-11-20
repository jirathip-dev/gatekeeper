import { readFileSync } from "node:fs";
import { basename, join } from "node:path";
import { parseArgs } from "node:util";
import { DIR, bareAuthUid, git, isMigration, local, parse } from "./migrations.ts";

const root = process.cwd();
const { base } = parseArgs({ options: { base: { type: "string" } } }).values;
const parsed = local(root);
const problems = parsed.flatMap((f) => ("error" in f ? [f.error] : []));
const files = parsed.filter(isMigration);
const seen = new Map<string, string>();
for (const f of files) {
  const other = seen.get(f.version);
  if (other !== undefined) {
    problems.push(`${DIR}/${f.file}: duplicate-version: ${f.version} is also ${other}`);
  }
  seen.set(f.version, f.file);
}
if (base) {
  const listed = git(root, ["ls-tree", "--name-only", base, `${DIR}/`]);
  const before = new Set(listed.split("\n").filter(Boolean).map((p) => basename(p)));
  const newest = [...before].map(parse).filter(isMigration).map((f) => f.version).sort().at(-1) ?? "";
  for (const f of files.filter((f) => !before.has(f.file))) {
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
process.stdout.write(`${parsed.length} migrations, ${problems.length} problems${base ? "" : " (no base: names only)"}\n`);
process.exit(problems.length ? 1 : 0);
