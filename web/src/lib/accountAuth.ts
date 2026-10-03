import { BillingApiError, saveStoredToken } from "./billing";

const CLIENT_ID = "guard-web";
const SCOPES = "openid profile email offline_access";
const TRANSACTION_KEY = "guard-account-oidc-transaction";
const MAX_DISCOVERY_BYTES = 64 * 1024;
const MAX_JWKS_BYTES = 256 * 1024;
const MAX_TOKEN_BYTES = 16 * 1024;

export interface AccountWebConfig {
  issuer: string;
  redirectUri: string;
  billingUrl: string;
}

export interface OidcMetadata {
  issuer: string;
  authorization_endpoint: string;
  token_endpoint: string;
  jwks_uri: string;
  end_session_endpoint?: string;
}

export interface LoginTransaction {
  state: string;
  nonce: string;
  verifier: string;
  redirectUri: string;
  createdAt: number;
}

interface TokenResponse {
  access_token: string;
  id_token: string;
  refresh_token?: string;
  token_type: string;
  expires_in: number;
}

type OidcJwk = JsonWebKey & { kid?: string; alg?: string; use?: string };

export interface AccountSession {
  accessToken: string;
  expiresAt: number;
}

let refreshToken = "";
let idToken = "";

function env(name: string): string {
  const value = import.meta.env?.[name] as string | undefined;
  return typeof value === "string" ? value : "";
}

function trustedUrl(raw: string, name: string): URL {
  let url: URL;
  try {
    url = new URL(raw);
  } catch {
    throw new Error(`${name} must be a valid absolute URL`);
  }
  const loopback = url.hostname === "127.0.0.1" || url.hostname === "localhost";
  if (url.protocol !== "https:" && !(loopback && url.protocol === "http:")) {
    throw new Error(
      `${name} must use HTTPS except on a loopback development host`,
    );
  }
  return url;
}

export function accountWebConfig(): AccountWebConfig {
  const issuer = env("VITE_ACCOUNT_ISSUER");
  const redirectUri = env("VITE_ACCOUNT_REDIRECT_URI");
  const billingUrl = env("VITE_ACCOUNT_BILLING_URL");
  if (!issuer || !redirectUri || !billingUrl) {
    throw new Error(
      "VITE_ACCOUNT_ISSUER, VITE_ACCOUNT_REDIRECT_URI, and VITE_ACCOUNT_BILLING_URL are required in account mode",
    );
  }
  const issuerUrl = trustedUrl(issuer, "VITE_ACCOUNT_ISSUER");
  const redirect = trustedUrl(redirectUri, "VITE_ACCOUNT_REDIRECT_URI");
  const billing = trustedUrl(billingUrl, "VITE_ACCOUNT_BILLING_URL");
  if (issuerUrl.search || issuerUrl.hash) {
    throw new Error("VITE_ACCOUNT_ISSUER must not contain a query or fragment");
  }
  if (redirect.hash) {
    throw new Error("VITE_ACCOUNT_REDIRECT_URI must not contain a fragment");
  }
  return {
    issuer: issuerUrl.href.replace(/\/$/, ""),
    redirectUri: redirect.href,
    billingUrl: billing.href,
  };
}

function base64Url(bytes: Uint8Array): string {
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary)
    .replace(/\+/g, "-")
    .replace(/\//g, "_")
    .replace(/=+$/g, "");
}

function decodeBase64Url(value: string): Uint8Array {
  if (!/^[A-Za-z0-9_-]+$/.test(value)) throw new Error("invalid JWT encoding");
  const padded = value
    .replace(/-/g, "+")
    .replace(/_/g, "/")
    .padEnd(Math.ceil(value.length / 4) * 4, "=");
  const binary = atob(padded);
  return Uint8Array.from(binary, (char) => char.charCodeAt(0));
}

function randomValue(): string {
  const bytes = new Uint8Array(32);
  crypto.getRandomValues(bytes);
  return base64Url(bytes);
}

export async function pkceChallenge(verifier: string): Promise<string> {
  const digest = await crypto.subtle.digest(
    "SHA-256",
    new TextEncoder().encode(verifier),
  );
  return base64Url(new Uint8Array(digest));
}

async function boundedJson<T>(
  response: Response,
  maxBytes: number,
): Promise<T> {
  if (!response.ok)
    throw new Error(`account request failed (${response.status})`);
  const text = await response.text();
  if (new TextEncoder().encode(text).length > maxBytes) {
    throw new Error("account response exceeded the size limit");
  }
  try {
    return JSON.parse(text) as T;
  } catch {
    throw new Error("account returned malformed JSON");
  }
}

export async function discoverAccount(
  issuer: string,
  fetcher: typeof fetch = fetch,
): Promise<OidcMetadata> {
  const discoveryUrl = new URL(
    "/.well-known/openid-configuration",
    `${issuer}/`,
  );
  const metadata = await boundedJson<OidcMetadata>(
    await fetcher(discoveryUrl, { headers: { Accept: "application/json" } }),
    MAX_DISCOVERY_BYTES,
  );
  if (metadata.issuer !== issuer)
    throw new Error("account discovery issuer mismatch");
  for (const [name, raw] of [
    ["authorization_endpoint", metadata.authorization_endpoint],
    ["token_endpoint", metadata.token_endpoint],
    ["jwks_uri", metadata.jwks_uri],
  ] as const) {
    const endpoint = trustedUrl(raw, name);
    if (endpoint.origin !== new URL(issuer).origin) {
      throw new Error(`account ${name} origin mismatch`);
    }
  }
  if (metadata.end_session_endpoint) {
    const endSession = trustedUrl(
      metadata.end_session_endpoint,
      "end_session_endpoint",
    );
    if (endSession.origin !== new URL(issuer).origin) {
      throw new Error("account end_session_endpoint origin mismatch");
    }
  }
  return metadata;
}

export async function createAuthorizationRequest(
  config: AccountWebConfig,
  metadata: OidcMetadata,
  now = Date.now(),
): Promise<{ url: string; transaction: LoginTransaction }> {
  const transaction: LoginTransaction = {
    state: randomValue(),
    nonce: randomValue(),
    verifier: randomValue(),
    redirectUri: config.redirectUri,
    createdAt: now,
  };
  const authorization = new URL(metadata.authorization_endpoint);
  for (const [key, value] of Object.entries({
    client_id: CLIENT_ID,
    redirect_uri: config.redirectUri,
    response_type: "code",
    scope: SCOPES,
    state: transaction.state,
    nonce: transaction.nonce,
    code_challenge: await pkceChallenge(transaction.verifier),
    code_challenge_method: "S256",
  })) {
    authorization.searchParams.set(key, value);
  }
  return { url: authorization.href, transaction };
}

export function storeLoginTransaction(transaction: LoginTransaction): void {
  sessionStorage.setItem(TRANSACTION_KEY, JSON.stringify(transaction));
}

function takeTransaction(): LoginTransaction {
  const raw = sessionStorage.getItem(TRANSACTION_KEY);
  sessionStorage.removeItem(TRANSACTION_KEY);
  if (!raw) throw new Error("login transaction is missing or already used");
  let transaction: LoginTransaction;
  try {
    transaction = JSON.parse(raw) as LoginTransaction;
  } catch {
    throw new Error("login transaction is invalid");
  }
  if (
    typeof transaction.state !== "string" ||
    !transaction.state ||
    transaction.state.length > 256 ||
    typeof transaction.nonce !== "string" ||
    !transaction.nonce ||
    transaction.nonce.length > 256 ||
    typeof transaction.verifier !== "string" ||
    !/^[A-Za-z0-9._~-]{43,128}$/.test(transaction.verifier) ||
    typeof transaction.redirectUri !== "string" ||
    typeof transaction.createdAt !== "number" ||
    Date.now() - transaction.createdAt > 10 * 60 * 1000 ||
    transaction.createdAt - Date.now() > 5 * 1000
  ) {
    throw new Error("login transaction is invalid or expired");
  }
  return transaction;
}

function constantTimeEqual(left: string, right: string): boolean {
  const a = new TextEncoder().encode(left);
  const b = new TextEncoder().encode(right);
  let diff = a.length ^ b.length;
  const length = Math.max(a.length, b.length);
  for (let i = 0; i < length; i += 1) diff |= (a[i] ?? 0) ^ (b[i] ?? 0);
  return diff === 0;
}

export async function beginAccountLogin(
  navigate: (url: string) => void = (url) => window.location.assign(url),
): Promise<void> {
  const config = accountWebConfig();
  const metadata = await discoverAccount(config.issuer);
  const request = await createAuthorizationRequest(config, metadata);
  storeLoginTransaction(request.transaction);
  navigate(request.url);
}

function decodeJsonPart(part: string): Record<string, unknown> {
  const bytes = decodeBase64Url(part);
  try {
    return JSON.parse(new TextDecoder().decode(bytes)) as Record<
      string,
      unknown
    >;
  } catch {
    throw new Error("ID token JSON is malformed");
  }
}

export async function verifyIdToken(
  token: string,
  metadata: OidcMetadata,
  nonce: string,
  fetcher: typeof fetch = fetch,
  nowSeconds = Math.floor(Date.now() / 1000),
): Promise<Record<string, unknown>> {
  if (!token || token.length > MAX_TOKEN_BYTES)
    throw new Error("ID token is invalid");
  const parts = token.split(".");
  if (parts.length !== 3) throw new Error("ID token is malformed");
  const header = decodeJsonPart(parts[0]);
  const claims = decodeJsonPart(parts[1]);
  if (header.alg !== "ES256" || typeof header.kid !== "string" || !header.kid) {
    throw new Error("ID token algorithm or key is invalid");
  }
  const jwks = await boundedJson<{ keys?: OidcJwk[] }>(
    await fetcher(metadata.jwks_uri, {
      headers: { Accept: "application/json" },
    }),
    MAX_JWKS_BYTES,
  );
  const candidates = (jwks.keys ?? []).filter(
    (key) =>
      key.kid === header.kid &&
      key.kty === "EC" &&
      key.crv === "P-256" &&
      (!key.alg || key.alg === "ES256") &&
      (!key.use || key.use === "sig"),
  );
  if (candidates.length !== 1)
    throw new Error("ID token key is unknown or ambiguous");
  const key = await crypto.subtle.importKey(
    "jwk",
    candidates[0],
    { name: "ECDSA", namedCurve: "P-256" },
    false,
    ["verify"],
  );
  const signature = Uint8Array.from(decodeBase64Url(parts[2])).buffer;
  const valid = await crypto.subtle.verify(
    { name: "ECDSA", hash: "SHA-256" },
    key,
    signature,
    new TextEncoder().encode(`${parts[0]}.${parts[1]}`),
  );
  if (!valid) throw new Error("ID token signature is invalid");
  const audience = claims.aud;
  const audienceOk =
    audience === CLIENT_ID ||
    (Array.isArray(audience) && audience.includes(CLIENT_ID));
  if (
    claims.iss !== metadata.issuer ||
    !audienceOk ||
    typeof claims.exp !== "number" ||
    claims.exp <= nowSeconds ||
    typeof claims.iat !== "number" ||
    claims.iat > nowSeconds + 5 ||
    typeof claims.sub !== "string" ||
    !claims.sub ||
    typeof claims.nonce !== "string" ||
    !constantTimeEqual(claims.nonce, nonce)
  ) {
    throw new Error("ID token claims are invalid");
  }
  if (
    Array.isArray(audience) &&
    audience.length > 1 &&
    claims.azp !== CLIENT_ID
  ) {
    throw new Error("ID token authorized party is invalid");
  }
  return claims;
}

export async function handleAccountCallback(
  callbackUrl = window.location.href,
  fetcher: typeof fetch = fetch,
): Promise<AccountSession> {
  const config = accountWebConfig();
  const callback = new URL(callbackUrl);
  const configuredRedirect = new URL(config.redirectUri);
  if (
    callback.origin !== configuredRedirect.origin ||
    callback.pathname !== configuredRedirect.pathname ||
    [...configuredRedirect.searchParams].some(
      ([name, value]) => callback.searchParams.get(name) !== value,
    )
  ) {
    throw new Error("Account callback redirect URI mismatch");
  }
  const oauthError = callback.searchParams.get("error");
  if (oauthError) {
    sessionStorage.removeItem(TRANSACTION_KEY);
    throw new Error(`Account sign-in failed: ${oauthError}`);
  }
  const code = callback.searchParams.get("code");
  const returnedState = callback.searchParams.get("state");
  if (!code || !returnedState)
    throw new Error("Account callback is missing code or state");
  const transaction = takeTransaction();
  if (
    transaction.redirectUri !== config.redirectUri ||
    !constantTimeEqual(returnedState, transaction.state)
  ) {
    throw new Error("Account callback state mismatch");
  }
  const metadata = await discoverAccount(config.issuer, fetcher);
  const body = new URLSearchParams({
    grant_type: "authorization_code",
    client_id: CLIENT_ID,
    code,
    redirect_uri: transaction.redirectUri,
    code_verifier: transaction.verifier,
  });
  const tokens = await boundedJson<TokenResponse>(
    await fetcher(metadata.token_endpoint, {
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      body,
    }),
    MAX_DISCOVERY_BYTES,
  );
  if (
    typeof tokens.token_type !== "string" ||
    tokens.token_type.toLowerCase() !== "bearer" ||
    typeof tokens.access_token !== "string" ||
    typeof tokens.id_token !== "string" ||
    typeof tokens.expires_in !== "number" ||
    tokens.expires_in <= 0
  ) {
    throw new Error("Account token response is invalid");
  }
  await verifyIdToken(tokens.id_token, metadata, transaction.nonce, fetcher);
  saveStoredToken(tokens.access_token);
  refreshToken =
    typeof tokens.refresh_token === "string" ? tokens.refresh_token : "";
  idToken = tokens.id_token;
  return {
    accessToken: tokens.access_token,
    expiresAt: Math.floor(Date.now() / 1000) + tokens.expires_in,
  };
}

export async function accountLogout(
  fetcher: typeof fetch = fetch,
  navigate: (url: string) => void = (url) => window.location.assign(url),
): Promise<void> {
  saveStoredToken("");
  refreshToken = "";
  idToken = "";
  sessionStorage.removeItem(TRANSACTION_KEY);
  const config = accountWebConfig();
  const metadata = await discoverAccount(config.issuer, fetcher);
  if (metadata.end_session_endpoint) navigate(metadata.end_session_endpoint);
}

export function accountBillingUrl(): string {
  return accountWebConfig().billingUrl;
}

export function accountAuthError(error: unknown): string {
  if (error instanceof BillingApiError || error instanceof Error)
    return error.message;
  return "Account sign-in failed";
}
