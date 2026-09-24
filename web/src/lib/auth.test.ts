import { afterEach, describe, expect, it, vi } from "vitest";
import {
  backendOnline,
  emailLogin,
  emailSignup,
  githubDeviceInit,
  githubPoll,
  googleAuthUrl,
  googleCallback,
  isNotConfigured,
  pollGithubUntilLinked,
} from "./auth";
import { BillingApiError } from "./billing";

function mockFetchOnce(body: unknown, status = 200): void {
  vi.stubGlobal(
    "fetch",
    vi.fn(
      async () =>
        ({
          ok: status >= 200 && status < 300,
          status,
          json: async () => body,
        }) as Response,
    ),
  );
}

afterEach(() => {
  vi.unstubAllGlobals();
  vi.unstubAllEnvs();
});

describe("email auth client", () => {
  const session = {
    provider: "email",
    email: "ada@example.com",
    name: "Ada",
    session_token: "sess_email_1",
    expires_in: 3600,
  };

  it("emailSignup posts email/password/name in dev (proxy path)", async () => {
    vi.stubEnv("VITE_BACKEND_URL", "");
    const seen: { url: string; body: unknown }[] = [];
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string, init: RequestInit) => {
        seen.push({ url, body: JSON.parse(init.body as string) });
        return {
          ok: true,
          status: 201,
          json: async () => session,
        } as Response;
      }),
    );
    const out = await emailSignup("ada@example.com", "password-01", "Ada");
    expect(out.session_token).toBe("sess_email_1");
    expect(seen).toHaveLength(1);
    expect(seen[0].url).toBe("/api/v1/auth/signup");
    expect(seen[0].body).toEqual({
      email: "ada@example.com",
      password: "password-01",
      name: "Ada",
    });
  });

  it("emailLogin strips /api on direct builds", async () => {
    vi.stubEnv("VITE_BACKEND_URL", "http://127.0.0.1:8080");
    const seen: string[] = [];
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string) => {
        seen.push(url);
        return {
          ok: true,
          status: 200,
          json: async () => session,
        } as Response;
      }),
    );
    const out = await emailLogin("ada@example.com", "password-01");
    expect(out.email).toBe("ada@example.com");
    expect(seen).toEqual(["http://127.0.0.1:8080/v1/auth/login"]);
  });

  it("blank name is omitted from the signup payload", async () => {
    vi.stubEnv("VITE_BACKEND_URL", "");
    const seen: { body: unknown }[] = [];
    vi.stubGlobal(
      "fetch",
      vi.fn(async (_url: string, init: RequestInit) => {
        seen.push({ body: JSON.parse(init.body as string) });
        return {
          ok: true,
          status: 201,
          json: async () => ({ ...session, name: null }),
        } as Response;
      }),
    );
    await emailSignup("ada@example.com", "password-01", "   ");
    expect(seen[0].body).toEqual({
      email: "ada@example.com",
      password: "password-01",
      name: undefined,
    });
  });

  it("401 login failure keeps the backend message", async () => {
    vi.stubEnv("VITE_BACKEND_URL", "");
    mockFetchOnce({ error: "invalid email or password" }, 401);
    const err = await emailLogin("a@b.c", "wrong").catch((e: unknown) => e);
    expect(err).toBeInstanceOf(BillingApiError);
    expect((err as BillingApiError).status).toBe(401);
    expect((err as BillingApiError).message).toBe("invalid email or password");
  });

  it("network failure surfaces status 0 with a backend hint", async () => {
    vi.stubEnv("VITE_BACKEND_URL", "");
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => {
        throw new TypeError("Failed to fetch");
      }),
    );
    const err = await emailSignup("a@b.c", "password-01").catch(
      (e: unknown) => e,
    );
    expect(err).toBeInstanceOf(BillingApiError);
    expect((err as BillingApiError).status).toBe(0);
    expect((err as BillingApiError).message).toContain("127.0.0.1:8080");
  });

  it("backendOnline is true on {status: ok}, false otherwise (never throws)", async () => {
    vi.stubEnv("VITE_BACKEND_URL", "");
    mockFetchOnce({ status: "ok", service: "algo-backend" });
    await expect(backendOnline()).resolves.toBe(true);
    mockFetchOnce({ status: "nope" });
    await expect(backendOnline()).resolves.toBe(false);
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => {
        throw new TypeError("down");
      }),
    );
    await expect(backendOnline()).resolves.toBe(false);
  });
});

describe("provider sign-in client", () => {
  it("githubDeviceInit + githubPoll hit the brokered routes", async () => {
    vi.stubEnv("VITE_BACKEND_URL", "");
    const seen: string[] = [];
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string) => {
        seen.push(url);
        if (url.endsWith("/github/device")) {
          return {
            ok: true,
            status: 200,
            json: async () => ({
              device_code: "gh_1",
              user_code: "WDKL-ABCD",
              verification_uri: "https://github.com/login/device",
              expires_in: 900,
              interval: 5,
            }),
          } as Response;
        }
        return {
          ok: true,
          status: 200,
          json: async () => ({
            pending: false,
            provider: "github",
            login: "octocat",
            session_token: "sess_1",
            expires_in: 3600,
          }),
        } as Response;
      }),
    );
    const init = await githubDeviceInit();
    expect(init.user_code).toBe("WDKL-ABCD");
    const linked = await pollGithubUntilLinked("gh_1", {
      sleep: async () => {},
    });
    expect(linked).toEqual({ session_token: "sess_1", login: "octocat" });
    expect(seen).toEqual([
      "/api/v1/auth/github/device",
      "/api/v1/auth/github/poll",
    ]);
  });

  it("unconfigured backend surfaces 503 + isNotConfigured", async () => {
    vi.stubEnv("VITE_BACKEND_URL", "");
    mockFetchOnce({ error: "github login not configured" }, 503);
    const err = await githubDeviceInit().catch((e: unknown) => e);
    expect(err).toBeInstanceOf(BillingApiError);
    expect((err as BillingApiError).status).toBe(503);
    expect(isNotConfigured(err)).toBe(true);
    expect(isNotConfigured(new Error("nope"))).toBe(false);
  });

  it("googleAuthUrl + googleCallback exchange code for a session", async () => {
    vi.stubEnv("VITE_BACKEND_URL", "http://127.0.0.1:8080");
    const seen: { url: string; body: unknown }[] = [];
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string, init: RequestInit) => {
        seen.push({ url, body: JSON.parse(init.body as string) });
        if (url.endsWith("/google/url")) {
          return {
            ok: true,
            status: 200,
            json: async () => ({
              auth_url: "https://accounts.google.com/o/oauth2/auth?...",
              state: "st_1",
              redirect_uri: "http://127.0.0.1:51004/oauth2redirect",
              expires_in: 600,
            }),
          } as Response;
        }
        return {
          ok: true,
          status: 200,
          json: async () => ({
            provider: "google",
            sub: "123",
            email: "a@b.c",
            name: "A B",
            session_token: "sess_2",
            expires_in: 3600,
          }),
        } as Response;
      }),
    );
    const url = await googleAuthUrl();
    expect(url.state).toBe("st_1");
    const session = await googleCallback("code_1", "st_1");
    expect(session.session_token).toBe("sess_2");
    expect(seen[0].url).toBe("http://127.0.0.1:8080/v1/auth/google/url");
    expect(seen[1].body).toEqual({ code: "code_1", state: "st_1" });
  });

  it("githubPoll posts the device_code", async () => {
    vi.stubEnv("VITE_BACKEND_URL", "");
    const seen: { url: string; body: unknown }[] = [];
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string, init: RequestInit) => {
        seen.push({ url, body: JSON.parse(init.body as string) });
        return {
          ok: true,
          status: 200,
          json: async () => ({ pending: true }),
        } as Response;
      }),
    );
    await expect(githubPoll("gh_9")).resolves.toEqual({ pending: true });
    expect(seen).toEqual([
      {
        url: "/api/v1/auth/github/poll",
        body: { device_code: "gh_9" },
      },
    ]);
  });
});
