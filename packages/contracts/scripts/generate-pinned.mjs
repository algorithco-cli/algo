import { execFileSync } from "node:child_process";
import { cpSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const tag = process.env.ALGO_PROTO_TAG || "v0.0.1-alpha";
const packageRoot = resolve(import.meta.dirname, "..");
const repositoryRoot = resolve(packageRoot, "..", "..");
const checkout = mkdtempSync(join(tmpdir(), "algo-contracts-proto-"));
const protoRoot = join(checkout, "proto");

function git(args, options = {}) {
  return execFileSync("git", args, {
    cwd: repositoryRoot,
    maxBuffer: 16 * 1024 * 1024,
    ...options,
  });
}

try {
  const resolved = git(["rev-parse", "--verify", `${tag}^{commit}`], {
    encoding: "utf8",
  }).trim();
  const files = git(["ls-tree", "-r", "--name-only", tag, "proto"], {
    encoding: "utf8",
  })
    .split(/\r?\n/u)
    .filter(Boolean);
  if (!files.includes("proto/buf.yaml")) {
    throw new Error(`${tag} does not contain proto/buf.yaml`);
  }

  for (const file of files) {
    const destination = join(checkout, file);
    mkdirSync(resolve(destination, ".."), { recursive: true });
    writeFileSync(destination, git(["show", `${tag}:${file}`]));
  }

  rmSync(join(packageRoot, "src", "gen"), { recursive: true, force: true });
  rmSync(join(packageRoot, "gen"), { recursive: true, force: true });
  execFileSync(
    process.platform === "win32" ? "buf.exe" : "buf",
    [
      "generate",
      protoRoot,
      "--template",
      join(repositoryRoot, "proto", "buf.gen.yaml"),
    ],
    { cwd: packageRoot, stdio: "inherit" },
  );
  mkdirSync(join(packageRoot, "src", "gen"), { recursive: true });
  cpSync(
    join(packageRoot, "gen", "ts", "algorithco_guard"),
    join(packageRoot, "src", "gen", "algorithco_guard"),
    { recursive: true },
  );
  rmSync(join(packageRoot, "gen"), { recursive: true, force: true });
  console.log(`generated contracts from ${tag} (${resolved.slice(0, 12)})`);
} finally {
  rmSync(checkout, { recursive: true, force: true });
}
