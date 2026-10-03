#!/usr/bin/env node

// Manual, cross-platform E2E for the real local Algorithco account stack.
// It never prints credentials or tokens and never writes to the account checkout.

import { spawn, spawnSync } from "node:child_process";
import { randomBytes, generateKeyPairSync } from "node:crypto";
import { createWriteStream, existsSync } from "node:fs";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import http from "node:http";
import https from "node:https";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import readline from "node:readline/promises";
import { fileURLToPath } from "node:url";

const guardRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const accountRoot = path.resolve(
  process.env.ALGORITHCO_ACCOUNT_REPO || path.join(guardRoot, "..", "algorithco-account"),
);
const composeFile = path.join(accountRoot, "infra", "docker-compose.yml");
const issuer = "https://auth.localhost:8443";
const backendUrl = "http://127.0.0.1:8080";
const webUrl = "http://127.0.0.1:3007";
const isWindows = process.platform === "win32";
const cargo = isWindows ? "cargo.exe" : "cargo";
const npm = isWindows ? "npm.cmd" : "npm";
const docker = isWindows ? "docker.exe" : "docker";
const git = isWindows ? "git.exe" : "git";
const exe = isWindows ? ".exe" : "";
const noBuild = process.argv.includes("--no-build");
const keepServices = process.argv.includes("--keep-services");
const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
let tempRoot;
let backend;
let web;
let composeArgs;
let accountWasRunning = false;
let originalAccountStatus = "";
let accountStatusChecked = false;

function fail(message) {
  throw new Error(message);
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: options.cwd || guardRoot,
    env: options.env || process.env,
    encoding: options.encoding === undefined ? "utf8" : options.encoding,
    stdio: options.stdio || "pipe",
    windowsHide: true,
  });
  if (result.error) fail(`${command} could not start: ${result.error.message}`);
  if (result.status !== 0 && !options.allowFailure) {
    const stderr = typeof result.stderr === "string" ? result.stderr.trim() : "";
    fail(`${command} exited ${result.status}${stderr ? `: ${stderr}` : ""}`);
  }
  return result;
}

function requireTool(command, versionArgs) {
  run(command, versionArgs);
}

function compose(extra, options = {}) {
  return run(docker, ["compose", ...composeArgs, ...extra], {
    cwd: accountRoot,
    ...options,
  });
}

function requestStatus(url, { ca, method = "GET", body } = {}) {
  return new Promise((resolve, reject) => {
    const target = new URL(url);
    const request = (target.protocol === "https:" ? https : http).request(
      target,
      {
        method,
        ...(ca ? { ca } : {}),
        headers: body
          ? { "content-type": "application/json", "content-length": Buffer.byteLength(body) }
          : {},
      },
      (response) => {
        response.resume();
        response.on("end", () => resolve(response.statusCode || 0));
      },
    );
    request.on("error", reject);
    if (body) request.write(body);
    request.end();
  });
}

async function waitFor(url, options = {}) {
  const deadline = Date.now() + 120_000;
  let lastError = "not ready";
  while (Date.now() < deadline) {
    try {
      const status = await requestStatus(url, options);
      if (status >= 200 && status < 500) return status;
      lastError = `HTTP ${status}`;
    } catch (error) {
      lastError = error instanceof Error ? error.message : String(error);
    }
    await new Promise((resolve) => setTimeout(resolve, 1_000));
  }
  fail(`${url} did not become ready: ${lastError}`);
}

async function confirm(message) {
  const answer = (await rl.question(`${message} [y/N] `)).trim().toLowerCase();
  if (answer !== "y" && answer !== "yes") fail("Manual E2E assertion was not confirmed");
}

function start(command, args, cwd, env, logPath) {
  const log = createWriteStream(logPath, { flags: "a" });
  const child = spawn(command, args, {
    cwd,
    env,
    stdio: ["ignore", log, log],
    windowsHide: true,
  });
  child.once("error", (error) => {
    process.stderr.write(`${command} failed to start: ${error.message}\n`);
  });
  child.log = log;
  return child;
}

function stopTree(child) {
  if (!child || child.exitCode !== null) return;
  if (isWindows) {
    run("taskkill.exe", ["/PID", String(child.pid), "/T", "/F"], { allowFailure: true });
  } else {
    child.kill("SIGTERM");
  }
  child.log?.end();
}

function makePrivateJwks() {
  const { privateKey } = generateKeyPairSync("ec", { namedCurve: "P-256" });
  const key = privateKey.export({ format: "jwk" });
  return {
    keys: [
      {
        ...key,
        kid: `guard-local-${randomBytes(8).toString("hex")}`,
        use: "sig",
        alg: "ES256",
      },
    ],
  };
}

async function cleanup() {
  stopTree(web);
  stopTree(backend);
  if (composeArgs && !keepServices && !accountWasRunning) {
    compose(["down"], { allowFailure: true, stdio: "inherit" });
  }
  if (accountStatusChecked) {
    const current = run(git, ["status", "--porcelain"], { cwd: accountRoot }).stdout;
    if (current !== originalAccountStatus) {
      process.stderr.write("WARNING: the account checkout status changed during E2E. Inspect it manually.\n");
    }
  }
  rl.close();
  if (tempRoot) await rm(tempRoot, { recursive: true, force: true });
}

async function main() {
  if (!existsSync(composeFile)) fail(`Account Compose file not found: ${composeFile}`);
  if (!process.env.GUARD_ACCOUNT_SERVICE_KEY?.startsWith("alg_sk_")) {
    fail(
      "Set GUARD_ACCOUNT_SERVICE_KEY to a read-only Guard service key provisioned in this local account database. It is read from the environment and never printed.",
    );
  }

  requireTool(git, ["--version"]);
  requireTool(docker, ["compose", "version"]);
  requireTool(cargo, ["--version"]);
  requireTool(npm, ["--version"]);

  originalAccountStatus = run(git, ["status", "--porcelain"], { cwd: accountRoot }).stdout;
  accountStatusChecked = true;
  if (originalAccountStatus !== "") fail("The account checkout must be clean before this read-only E2E");

  tempRoot = await mkdtemp(path.join(os.tmpdir(), "guard-account-e2e-"));
  const jwksPath = path.join(tempRoot, "oidc-jwks.json");
  const overridePath = path.join(tempRoot, "compose.override.yml");
  const caPath = path.join(tempRoot, "account-local-root.crt");
  const e2eHome = path.join(tempRoot, "home");
  await mkdir(e2eHome, { recursive: true });
  await writeFile(jwksPath, `${JSON.stringify(makePrivateJwks())}\n`, { mode: 0o600 });
  const dockerJwksPath = jwksPath.replaceAll("\\", "/");
  await writeFile(
    overridePath,
    [
      "services:",
      "  api:",
      "    environment:",
      "      ENTITLEMENTS_CLAIM_ENABLED: \"true\"",
      "      GUARD_WEB_REDIRECT_URIS: http://127.0.0.1:3007/auth/callback",
      "      GUARD_WEB_ORIGINS: http://127.0.0.1:3007",
      "secrets:",
      "  oidc-jwks:",
      `    file: ${JSON.stringify(dockerJwksPath)}`,
      "",
    ].join("\n"),
  );
  composeArgs = ["-f", "infra/docker-compose.yml", "-f", overridePath];
  accountWasRunning =
    run(docker, ["compose", "-f", "infra/docker-compose.yml", "ps", "-q"], {
      cwd: accountRoot,
    }).stdout.trim() !== "";

  process.stdout.write("Starting the account Compose stack from its own checkout...\n");
  compose(["up", "--build", "--wait"], { stdio: "inherit" });
  const cert = compose([
    "exec",
    "-T",
    "caddy",
    "cat",
    "/data/caddy/pki/authorities/local/root.crt",
  ], { encoding: null });
  await writeFile(caPath, cert.stdout, { mode: 0o600 });
  const ca = await readFile(caPath);
  await waitFor(`${issuer}/healthz`, { ca });

  process.stdout.write(
    `Account is ready. Trust this local Caddy root in the browser profile used for the manual check:\n${caPath}\n`,
  );
  await rl.question("Press Enter after the browser trusts the local root certificate. ");

  if (!noBuild) {
    run(cargo, ["build", "--manifest-path", "backend/Cargo.toml"], { stdio: "inherit" });
    run(cargo, ["build", "--manifest-path", "agent/Cargo.toml", "-p", "algocli-cli"], {
      stdio: "inherit",
    });
  }

  const backendBinary = path.join(guardRoot, "backend", "target", "debug", `algo-backend${exe}`);
  const algoBinary = path.join(guardRoot, "agent", "target", "debug", `algo${exe}`);
  if (!existsSync(backendBinary) || !existsSync(algoBinary)) {
    fail("Expected Guard binaries are missing; rerun without --no-build");
  }
  const sharedAccountEnv = {
    ALGO_ACCOUNT_ISSUER: issuer,
    ALGO_ACCOUNT_CA_CERT: caPath,
  };
  backend = start(
    backendBinary,
    [],
    guardRoot,
    {
      ...process.env,
      GUARD_AUTH_MODE: "account",
      GUARD_ACCOUNT_ISSUER: issuer,
      GUARD_ACCOUNT_AUDIENCES: "guard-web,guard-cli",
      GUARD_ACCOUNT_CA_CERT: caPath,
      ALGO_POLICY_SIGNING_SEED_HEX: randomBytes(32).toString("hex"),
    },
    path.join(tempRoot, "backend.log"),
  );
  web = start(
    npm,
    ["run", "dev", "--", "--host", "127.0.0.1", "--port", "3007", "--strictPort"],
    path.join(guardRoot, "web"),
    {
      ...process.env,
      VITE_GUARD_AUTH_MODE: "account",
      VITE_ACCOUNT_ISSUER: issuer,
      VITE_ACCOUNT_REDIRECT_URI: `${webUrl}/auth/callback`,
      VITE_ACCOUNT_BILLING_URL: `${issuer}/account`,
      VITE_BACKEND_URL: backendUrl,
    },
    path.join(tempRoot, "web.log"),
  );
  await waitFor(`${backendUrl}/health`);
  await waitFor(webUrl);
  const webhookStatus = await requestStatus(`${backendUrl}/v1/billing/webhook`, {
    method: "POST",
    body: "{}",
  });
  if (webhookStatus !== 404) {
    fail(`Deferred webhook route must remain absent in account mode; got HTTP ${webhookStatus}`);
  }

  process.stdout.write(
    `\nOpen ${webUrl}/login, sign in with Algorithco, then open /billing and click Refresh entitlement.\n`,
  );
  await confirm("Did web login succeed and did Billing show the effective Guard entitlement?");
  await confirm("Did web logout clear the Guard session and return through the account logout flow?");

  run(algoBinary, ["--home", e2eHome, "init", "--yes"], {
    env: { ...process.env, ...sharedAccountEnv },
    stdio: "inherit",
  });
  process.stdout.write("\nComplete the CLI device authorization in the browser when prompted.\n");
  run(algoBinary, ["--home", e2eHome, "login", "--flow", "device"], {
    env: { ...process.env, ...sharedAccountEnv },
    stdio: "inherit",
  });
  const marker = path.join(e2eHome, ".algo", "account-keyring.marker");
  if (!existsSync(marker)) fail("CLI login did not create the non-secret keyring marker");
  run(algoBinary, ["--home", e2eHome, "logout"], {
    env: { ...process.env, ...sharedAccountEnv },
    stdio: "inherit",
  });
  if (existsSync(marker)) fail("CLI logout left the account keyring marker behind");

  process.stdout.write("\nSign in once more to verify uninstall credential cleanup.\n");
  run(algoBinary, ["--home", e2eHome, "login", "--flow", "device"], {
    env: { ...process.env, ...sharedAccountEnv },
    stdio: "inherit",
  });
  if (!existsSync(marker)) fail("Second CLI login did not create the keyring marker");
  run(algoBinary, ["--home", e2eHome, "uninstall"], {
    env: { ...process.env, ...sharedAccountEnv },
    stdio: "inherit",
  });
  if (existsSync(marker)) fail("algo uninstall left the account keyring marker behind");

  const finalAccountStatus = run(git, ["status", "--porcelain"], { cwd: accountRoot }).stdout;
  if (finalAccountStatus !== originalAccountStatus) {
    fail("The account checkout changed; the E2E is required to leave it untouched");
  }
  process.stdout.write(
    "\nPASS: account/web/CLI login, entitlement display, deferred-webhook absence, logout, and uninstall cleanup were verified.\n",
  );
}

try {
  await main();
} catch (error) {
  process.stderr.write(`\nFAIL: ${error instanceof Error ? error.message : String(error)}\n`);
  process.exitCode = 1;
} finally {
  await cleanup();
}
