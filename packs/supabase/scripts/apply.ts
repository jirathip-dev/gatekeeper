import { readFileSync } from "node:fs";
import { join } from "node:path";
import { parseArgs } from "node:util";
import { DIR, LEDGER, compare, fail, git, ledgerApi, message } from "./migrations.ts";

const root = process.cwd();
const { target, branch } = parseArgs({
  options: { target: { type: "string" }, branch: { type: "string" } },
}).values;
if ((target !== "dev" && target !== "prod") || !branch) {
  fail("usage: apply.ts --target dev|prod --branch <deployed branch>");
}
if (git(root, ["status", "--porcelain", "--", DIR])) {
  fail(`supabase apply: ${DIR} has uncommitted changes; apply only what ${branch} holds`);
}
const head = git(root, ["rev-parse", "HEAD"]);
const tip = git(root, ["ls-remote", "origin", `refs/heads/${branch}`]).split(/\s/)[0] ?? "";
if (head !== tip) {
  fail(`supabase apply: this checkout is ${head.slice(0, 12)}, origin/${branch} is ${(tip || "missing").slice(0, 12)}; apply from a fresh checkout of ${branch}`);
}
try {
  const query = ledgerApi();
  await query(
    `create schema if not exists supabase_migrations; create table if not exists ${LEDGER} (version text primary key, statements text[], name text); alter table ${LEDGER} add column if not exists name text;`,
  );
  const { pending, warnings } = await compare(root, query);
  for (const w of warnings) {
    process.stdout.write(`${w}\n`);
  }
  for (const f of pending) {
    const sql = readFileSync(join(root, DIR, f.file), "utf8");
    let tag = "$gk$";
    while (sql.includes(tag)) {
      tag = `${tag.slice(0, -1)}_$`;
    }
    try {
      await query(
        `begin;\n${sql}\n;\ninsert into ${LEDGER} (version, name, statements) values ('${f.version}', '${f.name}', array[${tag}${sql}${tag}]);\ncommit;`,
      );
    } catch (e) {
      fail(`failed   ${f.file} (rolled back, not in the ledger): ${message(e)}`);
    }
    process.stdout.write(`applied  ${f.file}\n`);
  }
  process.stdout.write(`${target}: ${pending.length} applied\n`);
} catch (e) {
  fail(`supabase apply: ${message(e)}`);
}
