import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  type LoginTransaction,
  type OidcMetadata,
  accountLogout,
  createAuthorizationRequest,
  handleAccountCallback,
  pkceChallenge,
  storeLoginTransaction,
} from "./accountAuth";
import { loadStoredToken, saveStoredToken } from "./billing";

const issuer = "https://auth.algorithco.test";
const metadata: OidcMetadata = {
  issuer,
  authorization_endpoint: `${issuer}/auth`,
  token_endpoint: `${issuer}/token`,
  jwks_uri: `${issuer}/jwks`,
  end_session_endpoint: `${issuer}/session/end`,
};

class MemoryStorage implements Storage {
  readonly values = new Map<string, string>();
  get length(): number {
    return this.values.size;
  }
  clear(): void {
    this.values.clear();
  }
  getItem(key: string): string | null {
    return this.values.get(key) ?? null;
  }
  key(index: number): string | null {
    return [...this.values.keys()][index] ?? null;
  }
  removeItem(key: string): void {
    this.values.delete(key);
  }
  setItem(key: string, value: string): void {
    this.values.set(key, value);
  }
}

const response = (body: unknown, status = 200): Response =>
  new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });

function base64Url(value: Uint8Array | string): string {
  const bytes =
    typeof value === "string" ? new TextEncoder().encode(value) : value;
  return Buffer.from(bytes).toString("base64url");
}

async function signingKey(): Promise<{
  privateKey: CryptoKey;
  jwk: JsonWebKey & { kid: string; alg: string; use: string };
}> {
  const pair = (await crypto.subtle.generateKey(
    { name: "ECDSA", namedCurve: "P-256" },
    true,
    ["sign", "verify"],
  )) as CryptoKeyPair;
  return {
    privateKey: pair.privateKey,
    jwk: {
      ...(await crypto.subtle.exportKey("jwk", pair.publicKey)),
      kid: "test-key",
      alg: "ES256",
      use: "sig",
    },
  };
}

async function idToken(
  privateKey: CryptoKey,
  claims: Record<string, unknown>,
): Promise<string> {
  const header = base64Url(
    JSON.stringify({ alg: "ES256", kid: "test-key", typ: "JWT" }),
  );
  const payload = base64Url(JSON.stringify(claims));
  const signingInput = `${header}.${payload}`;
  const signature = await crypto.subtle.sign(
    { name: "ECDSA", hash: "SHA-256" },
    privateKey,
    new TextEncoder().encode(signingInput),
  );
  return `${signingInput}.${base64Url(new Uint8Array(signature))}`;
}

function transaction(
  overrides: Partial<LoginTransaction> = {},
): LoginTransaction {
  return {
    state: "expected-state",
    nonce: "expected-nonce",
    verifier: "test-verifier-with-more-than-forty-three-characters-123",
    redirectUri: "http://127.0.0.1:3007/auth/callback",
    createdAt: Date.now(),
    ...overrides,
  };
}

function accountFetcher(token: string, jwk: JsonWebKey): typeof fetch {
  return vi.fn(async (input: string | URL | Request) => {
    const url = String(input);
    if (url.includes(".well-known")) return response(metadata);
    if (url === metadata.token_endpoint) {
      return response({
        access_token: "obviously-fake-access-token",
        id_token: token,
        refresh_token: "obviously-fake-refresh-token",
        token_type: "Bearer",
        expires_in: 300,
      });
    }
    if (url === metadata.jwks_uri) return response({ keys: [jwk] });
    throw new Error(`unexpected test URL: ${url}`);
  }) as typeof fetch;
}

beforeEach(() => {
  vi.stubEnv("VITE_GUARD_AUTH_MODE", "account");
  vi.stubEnv("VITE_ACCOUNT_ISSUER", issuer);
  vi.stubEnv(
    "VITE_ACCOUNT_REDIRECT_URI",
    "http://127.0.0.1:3007/auth/callback",
  );
  vi.stubEnv("VITE_ACCOUNT_BILLING_URL", `${issuer}/account`);
  const localStorage = new MemoryStorage();
  const sessionStorage = new MemoryStorage();
  vi.stubGlobal("localStorage", localStorage);
  vi.stubGlobal("sessionStorage", sessionStorage);
  vi.stubGlobal("window", {
    localStorage,
    sessionStorage,
    dispatchEvent: vi.fn(),
    location: {
      href: "http://127.0.0.1:3007/auth/callback",
      assign: vi.fn(),
    },
  });
  saveStoredToken("");
});

afterEach(() => {
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
});

describe("account SPA authorization", () => {
  it("derives the RFC 7636 S256 challenge", async () => {
    await expect(
      pkceChallenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
    ).resolves.toBe("E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
  });

  it("creates a code request with PKCE, state, nonce, and exact redirect URI", async () => {
    const request = await createAuthorizationRequest(
      {
        issuer,
        redirectUri: "http://127.0.0.1:3007/auth/callback",
        billingUrl: `${issuer}/account`,
      },
      metadata,
    );
    const url = new URL(request.url);
    expect(url.searchParams.get("client_id")).toBe("guard-web");
    expect(url.searchParams.get("response_type")).toBe("code");
    expect(url.searchParams.get("code_challenge_method")).toBe("S256");
    expect(url.searchParams.get("redirect_uri")).toBe(
      request.transaction.redirectUri,
    );
    expect(url.searchParams.get("state")).toBe(request.transaction.state);
    expect(url.searchParams.get("nonce")).toBe(request.transaction.nonce);
    expect(url.searchParams.get("scope")).toBe(
      "openid profile email offline_access",
    );
  });

  it("rejects a callback state mismatch before exchanging the code", async () => {
    storeLoginTransaction(transaction());
    const fetcher = vi.fn() as unknown as typeof fetch;
    await expect(
      handleAccountCallback(
        "http://127.0.0.1:3007/auth/callback?code=test-code&state=attacker-state",
        fetcher,
      ),
    ).rejects.toThrow("state mismatch");
    expect(fetcher).not.toHaveBeenCalled();
  });

  it("rejects a signed ID token with the wrong nonce", async () => {
    const key = await signingKey();
    const now = Math.floor(Date.now() / 1000);
    const token = await idToken(key.privateKey, {
      iss: issuer,
      aud: "guard-web",
      sub: "test-subject",
      nonce: "wrong-nonce",
      iat: now,
      exp: now + 300,
    });
    storeLoginTransaction(transaction());
    await expect(
      handleAccountCallback(
        "http://127.0.0.1:3007/auth/callback?code=test-code&state=expected-state",
        accountFetcher(token, key.jwk),
      ),
    ).rejects.toThrow("claims are invalid");
    expect(loadStoredToken()).toBe("");
  });

  it("rejects an expired signed ID token", async () => {
    const key = await signingKey();
    const now = Math.floor(Date.now() / 1000);
    const token = await idToken(key.privateKey, {
      iss: issuer,
      aud: "guard-web",
      sub: "test-subject",
      nonce: "expected-nonce",
      iat: now - 600,
      exp: now - 1,
    });
    storeLoginTransaction(transaction());
    await expect(
      handleAccountCallback(
        "http://127.0.0.1:3007/auth/callback?code=test-code&state=expected-state",
        accountFetcher(token, key.jwk),
      ),
    ).rejects.toThrow("claims are invalid");
  });

  it("handles provider callback errors and consumes the transaction", async () => {
    storeLoginTransaction(transaction());
    await expect(
      handleAccountCallback(
        "http://127.0.0.1:3007/auth/callback?error=access_denied",
      ),
    ).rejects.toThrow("access_denied");
    await expect(
      handleAccountCallback(
        "http://127.0.0.1:3007/auth/callback?code=test-code&state=expected-state",
        vi.fn() as unknown as typeof fetch,
      ),
    ).rejects.toThrow("missing or already used");
  });

  it("stores a valid access token only in process memory", async () => {
    localStorage.setItem("algo-billing-token", "legacy-token-residue");
    const key = await signingKey();
    const now = Math.floor(Date.now() / 1000);
    const token = await idToken(key.privateKey, {
      iss: issuer,
      aud: "guard-web",
      sub: "test-subject",
      nonce: "expected-nonce",
      iat: now,
      exp: now + 300,
    });
    storeLoginTransaction(transaction());
    await handleAccountCallback(
      "http://127.0.0.1:3007/auth/callback?code=test-code&state=expected-state",
      accountFetcher(token, key.jwk),
    );
    expect(loadStoredToken()).toBe("obviously-fake-access-token");
    expect(localStorage.length).toBe(0);
    expect(sessionStorage.getItem("guard-account-oidc-transaction")).toBeNull();
  });

  it("clears local credentials before navigating to discovered logout", async () => {
    saveStoredToken("obviously-fake-access-token");
    storeLoginTransaction(transaction());
    const navigate = vi.fn();
    const fetcher = vi.fn(async () =>
      response(metadata),
    ) as unknown as typeof fetch;
    await accountLogout(fetcher, navigate);
    expect(loadStoredToken()).toBe("");
    expect(sessionStorage.getItem("guard-account-oidc-transaction")).toBeNull();
    expect(navigate).toHaveBeenCalledWith(metadata.end_session_endpoint);
  });
});
