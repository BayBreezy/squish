// Fails when the app version differs between the three places that carry it,
// or (when given a tag such as v1.2.3) when it doesn't match the tag.
//
//   node scripts/check-version.mjs          # consistency only
//   node scripts/check-version.mjs v1.2.3   # also compare with a release tag
import { readFileSync } from "node:fs";

const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), "utf8");

const versions = {
  "package.json": JSON.parse(read("package.json")).version,
  "src-tauri/tauri.conf.json": JSON.parse(read("src-tauri/tauri.conf.json")).version,
  "src-tauri/Cargo.toml": /^version\s*=\s*"([^"]+)"/m.exec(read("src-tauri/Cargo.toml"))?.[1],
};

const tag = (process.argv[2] ?? "").replace(/^v/, "");
const unique = new Set(Object.values(versions));
let ok = unique.size === 1 && !unique.has(undefined);

for (const [file, version] of Object.entries(versions)) {
  console.log(`${version === undefined ? "missing" : version}  ${file}`);
}
if (!ok) {
  console.error("\nVersion mismatch: package.json, tauri.conf.json and Cargo.toml must agree.");
}
if (tag) {
  const [version] = unique;
  if (version !== tag) {
    console.error(`\nTag v${tag} does not match the app version ${version}.`);
    ok = false;
  }
}
process.exit(ok ? 0 : 1);
