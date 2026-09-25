/* web — auth client against the live backend (no mock).
 * Backend contract (backend/src/email.rs + main.rs email_*_handler):
 *   POST /v1/auth/signup {email, password, name?} → {provider, email, name?,
 *     session_token, expires_in} (201; 409 duplicate; 400 invalid)
 *   POST /v1/auth/login {email, password} → same session shape (200;
 *     401 shared for unknown email / wrong password — no oracle)
 * Provider sign-in (backend/src/github.rs + google.rs):
 *   POST /v1/auth/github/device {} → {device_code, user_code,
 *     verification_uri, expires_in, interval}
 *   POST /v1/auth/github/poll {device_code} → {pending, ...session_token?}
 *   POST /v1/auth/google/url {} → {auth_url, state, redirect_uri, expires_in}
 *   POST /v1/auth/google/callback {code, state} → {provider, sub, email?,
 *     name?, session_token, expires_in}
 *   GET  /health → {status: "ok", service: "algo-backend"}
 * All calls go through apiPath() from billing.ts, so dev hits same-origin
 * /api/* (vite proxy strips the prefix) and direct builds (VITE_BACKEND_URL
 * set to a bare origin) hit /v1/* + /health on that origin.
 */

import { BillingApiError, apiPath } from "./billing";

export interface EmailSession {
  provider: string;
  email: string;
  name: string | null;
  session_token: string;
  expires_in: number;
}

async function postJson<T>(path: string, body: unknown): Promise<T> {
  const url = apiPath(path);
  let res: Response;
  try {
    res = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
  } catch {
    throw new BillingApiError(
      0,
      "Backend unreachable (is algo-backend running on http://127.0.0.1:8080?).",
      null,
    );
  }
  if (!res.ok) {
    let message = `Request failed (${res.status})`;
    try {
      const errBody = (await res.json()) as { error?: unknown };
      if (typeof errBody.error === "string" && errBody.error.length > 0) {
        message = errBody.error;
      }
    } catch {
      /* non-JSON error body — keep the default message */
    }
    throw new BillingApiError(res.status, message, null);
  }
  try {
    return (await res.json()) as T;
  } catch {
    throw new BillingApiError(
      res.status,
      `Backend returned non-JSON (expected JSON from ${url}).`,
      null,
    );
  }
}

/** Create an account. 409 when the email is taken, 400 on invalid input. */
export function emailSignup(
  email: string,
  password: string,
  name?: string,
): Promise<EmailSession> {
  return postJson<EmailSession>("/api/v1/auth/signup", {
    email,
    password,
    name: name?.trim() ? name.trim() : undefined,
  });
}

/** Log in. 401 covers unknown email and wrong password alike (no oracle). */
export function emailLogin(
  email: string,
  password: string,
): Promise<EmailSession> {
  return postJson<EmailSession>("/api/v1/auth/login", { email, password });
}

/** Backend liveness for the status badge (never throws). */
export async function backendOnline(): Promise<boolean> {
  try {
    const res = await fetch(apiPath("/api/health"), {
      headers: { Accept: "application/json" },
    });
    if (!res.ok) return false;
    const body = (await res.json()) as { status?: unknown };
    return body?.status === "ok";
  } catch {
    return false;
  }
}

/* ---------- provider sign-in (GitHub device flow + Google OIDC) ----------
 * The server holds the provider client secrets and mints short-lived backend
 * session JWTs. Unconfigured backend → 503 {"error": "<provider> login not
 * configured"} (set ALGO_GITHUB_CLIENT_ID / ALGO_GOOGLE_CLIENT_ID + secret,
 * restart).
 */

export interface GithubDeviceInit {
  device_code: string;
  user_code: string;
  verification_uri: string;
  expires_in: number;
  interval: number;
}

export interface GithubPoll {
  pending: boolean;
  provider?: string;
  login?: string;
  session_token?: string;
  expires_in?: number;
  refresh_token?: string | null;
  provider_expires_in?: number | null;
}

export interface GoogleAuthUrl {
  auth_url: string;
  state: string;
  redirect_uri: string;
  expires_in: number;
}

export interface GoogleSession {
  provider: string;
  sub: string;
  email: string | null;
  name: string | null;
  session_token: string;
  expires_in: number;
  refresh_token?: string | null;
  provider_expires_in?: number | null;
}

/** True when the backend reports the provider is not configured (503). */
export function isNotConfigured(err: unknown): boolean {
  return (
    err instanceof BillingApiError &&
    err.status === 503 &&
    /not configured/.test(err.message)
  );
}

/** GitHub step 1: mint device_code + user_code (approve at verification_uri). */
export function githubDeviceInit(): Promise<GithubDeviceInit> {
  return postJson<GithubDeviceInit>("/api/v1/auth/github/device", {});
}

/** GitHub step 2: single poll — resolves even while pending. */
export function githubPoll(deviceCode: string): Promise<GithubPoll> {
  return postJson<GithubPoll>("/api/v1/auth/github/poll", {
    device_code: deviceCode,
  });
}

/** GitHub step 2→3: poll until authorized (backend maps denial/expiry to 4xx). */
export async function pollGithubUntilLinked(
  deviceCode: string,
  opts: {
    intervalMs?: number;
    timeoutMs?: number;
    sleep?: (ms: number) => Promise<void>;
  } = {},
): Promise<{ session_token: string; login: string | null }> {
  const intervalMs = Math.min(Math.max(opts.intervalMs ?? 5000, 2000), 10000);
  const timeoutMs = opts.timeoutMs ?? 300_000;
  const sleep =
    opts.sleep ?? ((ms: number) => new Promise((r) => setTimeout(r, ms)));
  const started = Date.now();
  for (;;) {
    const p = await githubPoll(deviceCode);
    if (!p.pending && p.session_token) {
      return { session_token: p.session_token, login: p.login ?? null };
    }
    if (Date.now() - started > timeoutMs) {
      throw new BillingApiError(
        0,
        "GitHub login timed out — get a fresh code.",
        null,
      );
    }
    await sleep(intervalMs);
  }
}

/** Google step 1: get the consent URL + one-shot state (PKCE/nonce server-side). */
export function googleAuthUrl(): Promise<GoogleAuthUrl> {
  return postJson<GoogleAuthUrl>("/api/v1/auth/google/url", {});
}

/**
 * Google step 2: exchange the code for a backend session. The server-pinned
 * redirect URI is a loopback address, so after consent copy `code` and
 * `state` from that URL back here.
 */
export function googleCallback(
  code: string,
  state: string,
): Promise<GoogleSession> {
  return postJson<GoogleSession>("/api/v1/auth/google/callback", {
    code,
    state,
  });
}
