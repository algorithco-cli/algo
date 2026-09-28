import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..", "..", "..");
const requiredAdrs = ["0013", "0014"];
for (const number of requiredAdrs) {
  const adrDir = resolve(root, "docs", "adr");
  const found = execFileSync(
    "git",
    ["ls-files", `docs/adr/${number}-*.md`],
    { cwd: root, encoding: "utf8" },
  ).trim();
  if (!found || !existsSync(resolve(root, found))) {
    throw new Error(`release blocked: merged ADR-${number} is required`);
  }
  const body = readFileSync(resolve(root, found), "utf8");
  if (!/^Status:\s*Accepted\s*$/imu.test(body)) {
    throw new Error(`release blocked: ADR-${number} must have Status: Accepted`);
  }
}

const tag = process.env.ALGO_PROTO_TAG || "v0.0.1-alpha";
execFileSync("git", ["rev-parse", "--verify", `${tag}^{commit}`], {
  cwd: root,
  stdio: "ignore",
});
console.log(`release prerequisites verified for ${tag}`);
