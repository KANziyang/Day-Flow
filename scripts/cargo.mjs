import { spawnSync } from "node:child_process";
import { env } from "./runtime.mjs";
const { status, error } = spawnSync("cargo", process.argv.slice(2), {
  env,
  stdio: "inherit",
});
if (error) console.error(error.message);
process.exit(status ?? 1);
