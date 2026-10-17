import { existsSync } from "node:fs";
import { resolve, delimiter } from "node:path";
export const env = { ...process.env };
const cargoHome = resolve(".tools/cargo");
if (existsSync(cargoHome)) {
  env.CARGO_HOME = cargoHome;
  env.RUSTUP_HOME = resolve(".tools/rustup");
  const pathKey = Object.keys(env).find((key) => key.toLowerCase() === 'path') ?? 'Path';
  env[pathKey] = resolve(cargoHome, "bin") + delimiter + (env[pathKey] ?? '');
}
