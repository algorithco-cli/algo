import { execFileSync } from "node:child_process";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..", "..", "..");

const tag = process.env.ALGO_PROTO_TAG || "v0.0.1-alpha";
execFileSync("git", ["rev-parse", "--verify", `${tag}^{commit}`], {
  cwd: root,
  stdio: "ignore",
});
console.log(`release prerequisites verified for ${tag}`);
