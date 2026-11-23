// Lists English strings passed to t(), <Tr> (and PageHeader/Empty titles) that
// have no Spanish yet. Exits 1 when any are missing.
//   bun run i18n:check
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { ES } from "../src/lib/es";

function files(dir: string): string[] {
  return readdirSync(dir).flatMap((f) => {
    const p = join(dir, f);
    return statSync(p).isDirectory() ? files(p) : /\.tsx?$/.test(f) ? [p] : [];
  });
}

const patterns = [/\bt\(\s*"((?:[^"\\]|\\.)*)"/g, /<Tr>([^<{]+)<\/Tr>/g, /<(?:PageHeader|Empty)[^>]*?\b(?:title|subtitle)="([^"]*)"/gs];
const missing = new Map<string, string>();
for (const f of files("src")) {
  if (f.endsWith("es.ts") || f.endsWith("i18n.tsx")) continue;
  const src = readFileSync(f, "utf8");
  for (const re of patterns) {
    for (const m of src.matchAll(re)) {
      const key = JSON.parse(`"${m[1]}"`) as string;
      if (key && !(key in ES)) missing.set(key, f);
    }
  }
}
for (const [k, f] of missing) console.log(`${f}: ${k}`);
console.log(missing.size ? `\n${missing.size} strings have no Spanish yet.` : "Every string has Spanish.");
process.exit(missing.size ? 1 : 0);
