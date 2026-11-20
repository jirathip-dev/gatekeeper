import { execFile } from "node:child_process";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { type Server, createServer } from "node:http";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { after, before, test } from "node:test";
import assert from "node:assert/strict";
import { promisify } from "node:util";

type Sent = { query: string; auth: string; url: string };

const run = promisify(execFile);
const here = import.meta.dirname;
const ledger: { versions: string[]; queries: Sent[] } = { versions: [], queries: [] };
let api = "";
let server: Server | undefined;

before(async () => {
  const listening = createServer((req, res) => {
    let body = "";
    req.on("data", (c: Buffer) => (body += c.toString()));
    req.on("end", () => {
      const parsed: unknown = JSON.parse(body);
      const query = typeof parsed === "object" && parsed !== null && "query" in parsed ? String(parsed.query) : "";
      ledger.queries.push({ query, auth: req.headers.authorization ?? "", url: req.url ?? "" });
      const reply = (status: number, json: unknown) => res.writeHead(status).end(JSON.stringify(json));
      if (req.url?.startsWith("/bad") === true) {
        reply(200, { result: "no rows here" });
        return;
      }
      if (query.includes("boom")) {
        reply(400, { message: "syntax error at boom" });
        return;
      }
      if (query.startsWith("select version")) {
        reply(200, ledger.versions.toSorted().map((version) => ({ version })));
        return;
      }
      const added = /insert into supabase_migrations\.schema_migrations \(version, name, statements\) values \('(\d+)'/.exec(query)?.[1];
      if (added !== undefined) {
        ledger.versions.push(added);
      }
      reply(201, []);
    });
  });
  await new Promise<void>((ok) => listening.listen(0, "127.0.0.1", ok));
  const address = listening.address();
  api = `http://127.0.0.1:${typeof address === "object" && address !== null ? address.port : 0}`;
  server = listening;
});

after(() => server?.close());

function sh(cwd: string, ...args: string[]) {
  return run("git", ["-c", "user.name=t", "-c", "user.email=t@localhost", "-c", "commit.gpgsign=false", ...args], { cwd });
}

async function repo(files: Record<string, string>): Promise<string> {
  const dir = mkdtempSync(join(tmpdir(), "gk-ledger-"));
  const work = join(dir, "work");
  mkdirSync(join(work, "supabase/migrations"), { recursive: true });
  await sh(dir, "init", "-q", "--bare", "origin.git");
  await sh(work, "init", "-q");
  for (const [name, sql] of Object.entries(files)) {
    writeFileSync(join(work, "supabase/migrations", name), sql);
  }
  await sh(work, "add", "-A");
  await sh(work, "commit", "-q", "-m", "base");
  await sh(work, "remote", "add", "origin", join(dir, "origin.git"));
  await sh(work, "push", "-q", "origin", "HEAD:refs/heads/staging");
  return work;
}

async function script(cwd: string, name: string, args: string[] = [], env: Record<string, string> = {}) {
  ledger.queries = [];
  const vars = { ...process.env, SUPABASE_ACCESS_TOKEN: "tok", SUPABASE_PROJECT_REF: "ref", SUPABASE_API_URL: api, ...env };
  try {
    const { stdout } = await run("node", [join(here, name), ...args], { cwd, env: vars });
    return { code: 0, out: stdout };
  } catch (e) {
    const failed = typeof e === "object" && e !== null ? e : {};
    const code = "code" in failed && typeof failed.code === "number" ? failed.code : -1;
    const stdout = "stdout" in failed ? String(failed.stdout) : "";
    const stderr = "stderr" in failed ? String(failed.stderr) : "";
    return { code, out: `${stdout}${stderr}` };
  }
}

const apply = (cwd: string) => script(cwd, "apply.ts", ["--target", "dev", "--branch", "staging"]);
const A = "20260101000000_a.sql";
const B = "20260102000000_b.sql";
const C = "20260103000000_c.sql";

test("status lists applied and pending, and warns on ledger drift", async () => {
  const work = await repo({ [A]: "select 1;", [C]: "select 3;" });
  ledger.versions = ["20260101000000", "20260102000000"];
  const { code, out } = await script(work, "status.ts");
  assert.equal(code, 0, out);
  assert.match(out, /applied {2}20260101000000_a\.sql/);
  assert.match(out, /pending {2}20260103000000_c\.sql/);
  assert.match(out, /warning: 20260102000000 is in the ledger but has no file/);
  assert.equal(ledger.queries[0]?.auth, "Bearer tok");
  assert.equal(ledger.queries[0]?.url, "/v1/projects/ref/database/query");
});

test("status warns on a pending file older than the ledger's newest", async () => {
  const work = await repo({ [A]: "select 1;", [B]: "select 2;" });
  ledger.versions = ["20260102000000"];
  const { out } = await script(work, "status.ts");
  assert.match(out, /warning: 20260101000000_a\.sql is older than the ledger's newest 20260102000000 and not applied/);
});

test("apply runs each pending file with its ledger row in one transaction, then is a no-op", async () => {
  const work = await repo({ [A]: "select 1;", [B]: "create table t ($gk$);" });
  ledger.versions = ["20260101000000"];
  const first = await apply(work);
  assert.equal(first.code, 0, first.out);
  assert.match(first.out, /applied {2}20260102000000_b\.sql\ndev: 1 applied/);
  const sent = ledger.queries.find((q) => q.query.includes("create table t"))?.query ?? "";
  assert.match(sent, /^begin;\ncreate table t/);
  assert.match(sent, /array\[\$gk_\$create table t \(\$gk\$\);\$gk_\$\]\);\ncommit;$/);
  assert.deepEqual(ledger.versions, ["20260101000000", "20260102000000"]);
  const again = await apply(work);
  assert.equal(again.code, 0, again.out);
  assert.match(again.out, /dev: 0 applied/);
});

test("a failing file stops apply and stays out of the ledger", async () => {
  const work = await repo({ [A]: "boom;", [B]: "select 2;" });
  ledger.versions = [];
  const { code, out } = await apply(work);
  assert.equal(code, 1);
  assert.match(out, /failed {3}20260101000000_a\.sql \(rolled back, not in the ledger\): ledger API 400/);
  assert.deepEqual(ledger.versions, []);
  assert.ok(!ledger.queries.some((q) => q.query.includes("select 2")));
});

test("apply refuses a checkout behind its branch and sends nothing", async () => {
  const work = await repo({ [A]: "select 1;" });
  await sh(work, "commit", "-q", "--allow-empty", "-m", "ahead");
  await sh(work, "push", "-q", "origin", "HEAD:refs/heads/staging");
  await sh(work, "reset", "-q", "--hard", "HEAD~1");
  const { code, out } = await apply(work);
  assert.equal(code, 1);
  assert.match(out, /apply from a fresh checkout of staging/);
  assert.equal(ledger.queries.length, 0);
});

test("apply refuses uncommitted migrations", async () => {
  const work = await repo({ [A]: "select 1;" });
  writeFileSync(join(work, "supabase/migrations", B), "select 2;");
  const { code, out } = await apply(work);
  assert.equal(code, 1);
  assert.match(out, /supabase\/migrations has uncommitted changes/);
  assert.equal(ledger.queries.length, 0);
});

test("apply refuses a bad file name before touching the ledger rows", async () => {
  const work = await repo({ [A]: "select 1;", "init.sql": "select 2;" });
  ledger.versions = [];
  const { code, out } = await apply(work);
  assert.equal(code, 1);
  assert.match(out, /init\.sql: bad-name/);
  assert.deepEqual(ledger.versions, []);
});

test("a ledger reply without versions is refused", async () => {
  const work = await repo({ [A]: "select 1;" });
  const { code, out } = await script(work, "status.ts", [], { SUPABASE_API_URL: `${api}/bad` });
  assert.equal(code, 1, out);
  assert.match(out, /supabase status: the ledger API returned no row list: \{"result":"no rows here"\}/);
});

test("both scripts refuse without a token", async () => {
  const work = await repo({ [A]: "select 1;" });
  for (const name of ["status.ts", "apply.ts"]) {
    const args = name === "apply.ts" ? ["--target", "prod", "--branch", "staging"] : [];
    const { code, out } = await script(work, name, args, { SUPABASE_ACCESS_TOKEN: "" });
    assert.equal(code, 1);
    assert.match(out, /set SUPABASE_ACCESS_TOKEN and SUPABASE_PROJECT_REF/);
  }
});
