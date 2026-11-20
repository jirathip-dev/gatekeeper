import { parseArgs } from "node:util";
import { compare, fail, ledgerApi, message } from "./migrations.ts";

parseArgs({ options: { integration: { type: "string" } } });

try {
  const { files, applied, pending, warnings } = await compare(process.cwd(), ledgerApi());
  for (const f of files) {
    process.stdout.write(`${applied.has(f.version) ? "applied" : "pending"}  ${f.file}\n`);
  }
  for (const w of warnings) {
    process.stdout.write(`${w}\n`);
  }
  process.stdout.write(`${files.length} migrations, ${pending.length} pending\n`);
} catch (e) {
  fail(`supabase status: ${message(e)}`);
}
