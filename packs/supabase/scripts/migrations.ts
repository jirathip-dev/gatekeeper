import { execFileSync } from "node:child_process";
import { readdirSync } from "node:fs";
import { join } from "node:path";

export const DIR = "supabase/migrations";
export const LEDGER = "supabase_migrations.schema_migrations";
const NAME = /^(\d{14})_([a-z0-9_]+)\.sql$/;

export type Migration = { file: string; version: string; name: string };
export type Parsed = Migration | { file: string; error: string };
export type Query = (sql: string) => Promise<unknown>;

export function git(root: string, args: string[]): string {
  return execFileSync("git", args, { cwd: root, encoding: "utf8" }).trim();
}

export function message(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

export function parse(file: string): Parsed {
  const m = NAME.exec(file);
  const version = m?.[1];
  const name = m?.[2];
  if (version === undefined || name === undefined) {
    return { file, error: `${DIR}/${file}: bad-name: not <14-digit UTC timestamp>_<lowercase_name>.sql` };
  }
  const iso = `${version.slice(0, 4)}-${version.slice(4, 6)}-${version.slice(6, 8)}T${version.slice(8, 10)}:${version.slice(10, 12)}:${version.slice(12)}Z`;
  const time = new Date(iso);
  if (Number.isNaN(time.getTime()) || time.toISOString().replace(/\D/g, "").slice(0, 14) !== version) {
    return { file, error: `${DIR}/${file}: bad-timestamp: ${version} is no real UTC time` };
  }
  return { file, version, name };
}

export function isMigration(f: Parsed): f is Migration {
  return !("error" in f);
}

export function local(root: string): Parsed[] {
  try {
    return readdirSync(join(root, DIR)).sort().map(parse);
  } catch {
    return [];
  }
}

export function bareAuthUid(sql: string): boolean {
  return sql
    .replace(/--[^\n]*/g, "")
    .split(";")
    .filter((st) => /^\s*(create|alter)\s+policy\b/i.test(st))
    .some((st) =>
      [...st.matchAll(/auth\.uid\s*\(\s*\)/gi)].some((m) => !/\(\s*select\s+$/i.test(st.slice(0, m.index))),
    );
}

export function ledgerApi(env: NodeJS.ProcessEnv = process.env): Query {
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
    }).catch((e: unknown) => {
      const cause = e instanceof Error && e.cause instanceof Error ? e.cause : e;
      throw new Error(`${url}: ${message(cause)}`);
    });
    const body = await res.text();
    if (!res.ok) {
      throw new Error(`ledger API ${res.status}: ${body}`);
    }
    const rows: unknown = JSON.parse(body);
    return rows;
  };
}

function versions(rows: unknown): string[] {
  if (!Array.isArray(rows)) {
    throw new Error(`the ledger API returned no row list: ${JSON.stringify(rows)}`);
  }
  return rows.map((row: unknown) => {
    const version: unknown = typeof row === "object" && row !== null && "version" in row ? row.version : undefined;
    if (typeof version !== "string") {
      throw new Error(`a ledger row has no version: ${JSON.stringify(row)}`);
    }
    return version;
  });
}

export async function compare(root: string, query: Query) {
  const parsed = local(root);
  const bad = parsed.flatMap((f) => ("error" in f ? [f.error] : []));
  if (bad.length) {
    throw new Error(bad.join("\n"));
  }
  const files = parsed.filter(isMigration);
  const rows = versions(await query(`select version from ${LEDGER} order by version`)).sort();
  const applied = new Set(rows);
  const newest = rows.at(-1) ?? "";
  const pending = files.filter((f) => !applied.has(f.version));
  const warnings = [
    ...rows
      .filter((v) => !files.some((f) => f.version === v))
      .map((v) => `warning: ${v} is in the ledger but has no file in ${DIR}`),
    ...pending
      .filter((f) => f.version < newest)
      .map((f) => `warning: ${f.file} is older than the ledger's newest ${newest} and not applied`),
  ];
  return { files, applied, pending, warnings };
}

export function fail(text: string): never {
  process.stderr.write(`${text}\n`);
  process.exit(1);
}
