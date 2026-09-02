import { readFile } from "node:fs/promises";

const packageJson = JSON.parse(await readFile(new URL("../package.json", import.meta.url), "utf8"));
const cargoToml = await readFile(new URL("../Cargo.toml", import.meta.url), "utf8");
const packageSection = cargoToml.match(/\[package\]([\s\S]*?)(?:\n\[|$)/)?.[1] ?? "";
const cargoVersion = packageSection.match(/^version\s*=\s*"([^"]+)"/m)?.[1];

if (!cargoVersion) throw new Error("Cargo.toml package version was not found");
if (packageJson.version !== cargoVersion) {
  throw new Error(`version mismatch: package.json ${packageJson.version}, Cargo.toml ${cargoVersion}`);
}

console.log(`version ${cargoVersion}`);
