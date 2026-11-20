import { execFileSync } from "node:child_process";
import { readdirSync } from "node:fs";
import { join } from "node:path";

export const DIR = "supabase/migrations";
export const LEDGER = "supabase_migrations.schema_migrations";
const NAME = /^(\d{14})_([a-z0-9_]+)\.sql$/;

export function git(root, args) {
  return execFileSync("git", args, { cwd: root, encoding: "utf8" }).trim();
}

export function parse(file) {
  const m = NAME.exec(file);
  if (!m) {
    return { file, error: `${DIR}/${file}: bad-name: not <14-digit UTC timestamp>_<lowercase_name>.sql` };
  }
  const [, version, name] = m;
  const iso = `${version.slice(0, 4)}-${version.slice(4, 6)}-${version.slice(6, 8)}T${version.slice(8, 10)}:${version.slice(10, 12)}:${version.slice(12)}Z`;
  const time = new Date(iso);
  if (Number.isNaN(time.getTime()) || time.toISOString().replace(/\D/g, "").slice(0, 14) !== version) {
    return { file, error: `${DIR}/${file}: bad-timestamp: ${version} is no real UTC time` };
  }
  return { file, version, name };
}

export function local(root) {
  let names = [];
  try {
    names = readdirSync(join(root, DIR));
  } catch {
    return [];
  }
  return names.sort().map(parse);
}

export function bareAuthUid(sql) {
  return sql
    .replace(/--[^\n]*/g, "")
    .split(";")
    .filter((st) => /^\s*(create|alter)\s+policy\b/i.test(st))
    .some((st) =>
      [...st.matchAll(/auth\.uid\s*\(\s*\)/gi)].some((m) => !/\(\s*select\s+$/i.test(st.slice(0, m.index))),
    );
}

export function ledgerApi(env = process.env) {
  const { SUPABASE_ACCESS_TOKEN: token, SUPABASE_PROJECT_REF: ref } = env;
  if (!token || !ref) {
    throw new Error("set SUPABASE_ACCESS_TOKEN and SUPABASE_PROJECT_REF");
  }
  const url = `${env.SUPABASE_API_URL ?? "https://api.supabase.com"}/v1/projects/${ref}/database/query`;
  return async (query) => {
    const res = await fetch(url, {
      method: "POST",
      headers: { authorization: `Bearer ${token}`, "content-type": "application/json" },
      body: JSON.stringify({ query }),
    }).catch((e) => {
      throw new Error(`${url}: ${e.cause?.message ?? e.message}`);
    });
    const body = await res.text();
    if (!res.ok) {
      throw new Error(`ledger API ${res.status}: ${body}`);
    }
    return JSON.parse(body);
  };
}

export async function compare(root, query) {
  const files = local(root);
  const bad = files.filter((f) => f.error);
  if (bad.length) {
    throw new Error(bad.map((f) => f.error).join("\n"));
  }
  const rows = await query(`select version from ${LEDGER} order by version`);
  const applied = new Set(rows.map((r) => r.version));
  const newest = rows.at(-1)?.version ?? "";
  const pending = files.filter((f) => !applied.has(f.version));
  const warnings = [
    ...rows
      .filter((r) => !files.some((f) => f.version === r.version))
      .map((r) => `warning: ${r.version} is in the ledger but has no file in ${DIR}`),
    ...pending
      .filter((f) => f.version < newest)
      .map((f) => `warning: ${f.file} is older than the ledger's newest ${newest} and not applied`),
  ];
  return { files, applied, pending, warnings };
}

export function fail(message) {
  process.stderr.write(`${message}\n`);
  process.exit(1);
}
