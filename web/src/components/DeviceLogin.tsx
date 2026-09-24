import ArrowRight from "lucide-react/icons/arrow-right.mjs";
import Check from "lucide-react/icons/check.mjs";
import EyeOff from "lucide-react/icons/eye-off.mjs";
import Eye from "lucide-react/icons/eye.mjs";
import Lock from "lucide-react/icons/lock.mjs";
import Mail from "lucide-react/icons/mail.mjs";
import ShieldCheck from "lucide-react/icons/shield-check.mjs";
import User from "lucide-react/icons/user.mjs";
import * as React from "react";
import { Link } from "react-router-dom";
import { useMediaQuery } from "../hooks/useMediaQuery";
import {
  type GithubDeviceInit,
  type GoogleAuthUrl,
  backendOnline,
  emailLogin,
  emailSignup,
  githubDeviceInit,
  googleAuthUrl,
  googleCallback,
  pollGithubUntilLinked,
} from "../lib/auth";
import {
  BillingApiError,
  loadStoredToken,
  saveStoredToken,
} from "../lib/billing";
import PixelGuardDog from "./PixelGuardDog";
import { GithubIcon, GoogleIcon } from "./PlatformIcons";

type Method = "email" | "github" | "google";
type EmailMode = "login" | "signup";

function messageOf(err: unknown, fallback: string): string {
  if (err instanceof BillingApiError) {
    // 409 duplicate reads friendlier on a signup form.
    if (err.status === 409)
      return "An account with this email already exists — log in instead.";
    return err.message;
  }
  if (err instanceof Error) return `${fallback}: ${err.message}`;
  return fallback;
}

function providerMessage(
  err: unknown,
  envVar: string,
  fallback: string,
): string {
  if (
    err instanceof BillingApiError &&
    err.status === 503 &&
    /not configured/.test(err.message)
  ) {
    return `Provider login is not configured on this backend — set ${envVar} (and secret) and restart algo-backend.`;
  }
  return messageOf(err, fallback);
}

function expiryLabel(expiresIn: number): string {
  const mins = Math.max(1, Math.round(expiresIn / 60));
  return `expires in ${mins} min`;
}

/**
 * Sign-in card: email account (create + login, argon2id-hashed server-side)
 * or provider OAuth brokered by the backend. Every method mints the same
 * backend session token, stored for the Billing page.
 */
export function DeviceLogin(): JSX.Element {
  const [online, setOnline] = React.useState<boolean | null>(null);
  const [method, setMethod] = React.useState<Method>("email");
  const [emailMode, setEmailMode] = React.useState<EmailMode>("login");
  const [linked, setLinked] = React.useState(() => loadStoredToken() !== "");
  const [ghInit, setGhInit] = React.useState<GithubDeviceInit | null>(null);
  const [ggUrl, setGgUrl] = React.useState<GoogleAuthUrl | null>(null);
  const [token, setToken] = React.useState<string>(() => loadStoredToken());
  const [email, setEmail] = React.useState("");
  const [password, setPassword] = React.useState("");
  const [name, setName] = React.useState("");
  const [ggCode, setGgCode] = React.useState("");
  const [ggState, setGgState] = React.useState("");
  const [busy, setBusy] = React.useState(false);
  const [waiting, setWaiting] = React.useState(false);
  const [showPassword, setShowPassword] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const [copied, setCopied] = React.useState<string | null>(null);
  const runId = React.useRef(0);
  // Decorative mascot: desktop only (>=1280px). Never rendered elsewhere,
  // so the sprite loads/renders nothing and reserves no space off-desktop.
  const showMascot = useMediaQuery("(min-width: 1280px)");

  React.useEffect(() => {
    let cancelled = false;
    backendOnline().then((ok) => {
      if (!cancelled) setOnline(ok);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const finishLink = React.useCallback((id: number, accessToken: string) => {
    if (runId.current !== id) return;
    saveStoredToken(accessToken);
    setToken(accessToken);
    setLinked(true);
    setBusy(false);
    setWaiting(false);
  }, []);

  const fail = React.useCallback((id: number, message: string) => {
    if (runId.current !== id) return;
    setError(message);
    setBusy(false);
    setWaiting(false);
  }, []);

  const switchMethod = React.useCallback((next: Method) => {
    runId.current += 1; // cancel any in-flight poll
    setMethod(next);
    setError(null);
    setBusy(false);
    setWaiting(false);
    setCopied(null);
  }, []);

  const switchEmailMode = React.useCallback((next: EmailMode) => {
    setEmailMode(next);
    setError(null);
  }, []);

  const cancelPoll = React.useCallback(() => {
    runId.current += 1;
    setBusy(false);
    setWaiting(false);
  }, []);

  const copyText = React.useCallback(async (key: string, text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(key);
      window.setTimeout(() => {
        setCopied((c) => (c === key ? null : c));
      }, 2000);
    } catch {
      setError("Could not copy — select the value manually.");
    }
  }, []);

  const submitEmail = React.useCallback(async () => {
    const cleanEmail = email.trim();
    if (!cleanEmail || !password) {
      setError("Enter your email and password.");
      return;
    }
    if (password.length < 8) {
      setError("Password must be at least 8 characters.");
      return;
    }
    const id = ++runId.current;
    setBusy(true);
    setError(null);
    try {
      const session =
        emailMode === "signup"
          ? await emailSignup(cleanEmail, password, name)
          : await emailLogin(cleanEmail, password);
      finishLink(id, session.session_token);
    } catch (err) {
      fail(
        id,
        messageOf(
          err,
          emailMode === "signup" ? "Could not create account" : "Login failed",
        ),
      );
    }
  }, [email, emailMode, fail, finishLink, name, password]);

  const startGithub = React.useCallback(async () => {
    const id = ++runId.current;
    setBusy(true);
    setWaiting(false);
    setError(null);
    try {
      const fresh = await githubDeviceInit();
      if (runId.current !== id) return;
      setGhInit(fresh);
      setWaiting(true);
      setBusy(false);
      const intervalMs = Math.min(Math.max(fresh.interval * 1000, 2000), 10000);
      const linkedToken = await pollGithubUntilLinked(fresh.device_code, {
        intervalMs,
      });
      finishLink(id, linkedToken.session_token);
    } catch (err) {
      fail(
        id,
        providerMessage(err, "ALGO_GITHUB_CLIENT_ID", "GitHub login failed"),
      );
    }
  }, [fail, finishLink]);

  const startGoogle = React.useCallback(async () => {
    const id = ++runId.current;
    setBusy(true);
    setError(null);
    try {
      const fresh = await googleAuthUrl();
      if (runId.current !== id) return;
      setGgUrl(fresh);
      setGgState(fresh.state);
      setBusy(false);
    } catch (err) {
      fail(
        id,
        providerMessage(err, "ALGO_GOOGLE_CLIENT_ID", "Google login failed"),
      );
    }
  }, [fail]);

  const finishGoogle = React.useCallback(async () => {
    const code = ggCode.trim();
    const st = ggState.trim();
    if (!code || !st) {
      setError("Paste both code and state from the redirect URL.");
      return;
    }
    const id = ++runId.current;
    setBusy(true);
    setWaiting(true);
    setError(null);
    try {
      const session = await googleCallback(code, st);
      finishLink(id, session.session_token);
    } catch (err) {
      fail(
        id,
        providerMessage(err, "ALGO_GOOGLE_CLIENT_ID", "Google login failed"),
      );
    }
  }, [fail, finishLink, ggCode, ggState]);

  const logout = React.useCallback(() => {
    runId.current += 1;
    saveStoredToken("");
    setToken("");
    setEmail("");
    setPassword("");
    setName("");
    setGgCode("");
    setGgState("");
    setGhInit(null);
    setGgUrl(null);
    setError(null);
    setCopied(null);
    setBusy(false);
    setWaiting(false);
    setShowPassword(false);
    setLinked(false);
  }, []);

  if (linked) {
    return (
      <div className="auth-stage">
        <div className="card login-card auth-center">
          <span className="auth-success-icon" aria-hidden="true">
            <ShieldCheck size={36} />
          </span>
          <h2 className="auth-title">You&apos;re in</h2>
          <p className="muted auth-lede">
            Token saved for the <Link to="/billing">Billing page</Link>. Team
            cloud is Phase 3 (planned); today everything still runs local-first.
          </p>
          <p className="auth-actions">
            <button
              type="button"
              className="btn"
              onClick={() => void copyText("token", token)}
            >
              {copied === "token" ? "Copied!" : "Copy token"}
            </button>
            <button type="button" className="btn" onClick={logout}>
              Log out
            </button>
          </p>
        </div>
        <div className="auth-mascot" aria-hidden="true">
          {showMascot ? <PixelGuardDog size={320} alert /> : null}
        </div>
      </div>
    );
  }

  const title =
    method === "email"
      ? emailMode === "signup"
        ? "Create account"
        : "Log in"
      : method === "github"
        ? "Continue with GitHub"
        : "Continue with Google";
  const pwLongEnough = password.length >= 8;

  return (
    <div className="auth-stage">
      <div className="card login-card auth-center">
        <p className="login-hint auth-status">
          {online === null ? (
            <span className="muted">Checking backend…</span>
          ) : online ? (
            <span className="badge badge-allow">backend online</span>
          ) : (
            <span className="badge badge-deny">
              backend offline — start it on http://127.0.0.1:8080
            </span>
          )}
        </p>
        <h2 className="auth-title">{title}</h2>
        <p className="muted auth-lede">
          {method === "email"
            ? emailMode === "signup"
              ? "Create your account and pick up right where you left off."
              : "Log in to your account and seamlessly pick up where you left off."
            : "Approve in your browser — the backend mints the session."}
        </p>

        {method === "email" ? (
          <form
            className="auth-form"
            onSubmit={(e) => {
              e.preventDefault();
              void submitEmail();
            }}
          >
            <label className="field-pill" htmlFor="login-email">
              <Mail size={16} aria-hidden="true" />
              <input
                id="login-email"
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder="Enter your email address"
                autoComplete="email"
                disabled={busy}
              />
            </label>
            <label className="field-pill" htmlFor="login-password">
              <Lock size={16} aria-hidden="true" />
              <input
                id="login-password"
                type={showPassword ? "text" : "password"}
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder="Enter your password"
                autoComplete={
                  emailMode === "signup" ? "new-password" : "current-password"
                }
                disabled={busy}
              />
              <button
                type="button"
                className="pw-toggle"
                onClick={() => setShowPassword((s) => !s)}
                aria-pressed={showPassword}
                aria-label={showPassword ? "Hide password" : "Show password"}
                disabled={busy}
              >
                {showPassword ? (
                  <EyeOff size={16} aria-hidden="true" />
                ) : (
                  <Eye size={16} aria-hidden="true" />
                )}
              </button>
            </label>
            {emailMode === "signup" ? (
              <label className="field-pill" htmlFor="login-name">
                <User size={16} aria-hidden="true" />
                <input
                  id="login-name"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="Display name (optional)"
                  autoComplete="nickname"
                  disabled={busy}
                />
              </label>
            ) : null}
            {emailMode === "signup" && password.length > 0 ? (
              <output
                className={`login-hint pw-hint${pwLongEnough ? " ok" : ""}`}
              >
                {pwLongEnough ? (
                  <>
                    <Check size={14} aria-hidden="true" /> Good length
                  </>
                ) : (
                  <>At least 8 characters ({password.length}/8)</>
                )}
              </output>
            ) : null}
            <button
              type="submit"
              className="btn btn-primary btn-block auth-submit"
              disabled={busy || online === false}
            >
              {busy ? (
                "Contacting backend…"
              ) : emailMode === "signup" ? (
                <>
                  Create account <ArrowRight size={16} aria-hidden="true" />
                </>
              ) : (
                <>
                  Log in <ArrowRight size={16} aria-hidden="true" />
                </>
              )}
            </button>
          </form>
        ) : null}

        {method === "github" ? (
          <div className="auth-fade">
            <p className="muted auth-lede">
              The backend brokers GitHub&apos;s device flow and mints the
              session — secrets never touch the browser.
            </p>
            {ghInit ? (
              <>
                <p className="user-code">
                  <code>{ghInit.user_code}</code>
                  <button
                    type="button"
                    className="btn"
                    onClick={() => void copyText("gh", ghInit.user_code)}
                  >
                    {copied === "gh" ? "Copied!" : "Copy"}
                  </button>
                </p>
                <p className="muted login-hint">
                  Enter it at{" "}
                  <a
                    href={ghInit.verification_uri}
                    target="_blank"
                    rel="noreferrer"
                  >
                    {ghInit.verification_uri}
                  </a>{" "}
                  · {expiryLabel(ghInit.expires_in)}
                  {waiting ? " · polling…" : ""}
                </p>
                {waiting ? (
                  <p className="login-hint">
                    <button type="button" className="btn" onClick={cancelPoll}>
                      Cancel
                    </button>
                  </p>
                ) : null}
              </>
            ) : (
              <button
                type="button"
                className="btn btn-primary btn-block auth-submit"
                onClick={() => void startGithub()}
                disabled={busy || online === false}
              >
                {busy ? "Contacting GitHub…" : "Continue with GitHub"}
              </button>
            )}
            <button
              type="button"
              className="login-link"
              onClick={() => switchMethod("email")}
            >
              ← Use email instead
            </button>
          </div>
        ) : null}

        {method === "google" ? (
          <div className="auth-fade">
            <p className="muted auth-lede">
              Consent happens at Google; the backend verifies the ID token
              (JWKS, aud/iss/exp, nonce) and mints the session.
            </p>
            {ggUrl ? (
              <>
                <a
                  className="btn btn-block"
                  href={ggUrl.auth_url}
                  target="_blank"
                  rel="noreferrer"
                >
                  Open Google sign-in
                </a>
                <p className="muted login-hint">
                  After consent the browser lands on a loopback address — copy{" "}
                  <code>code</code> and <code>state</code> from that URL back
                  here ({expiryLabel(ggUrl.expires_in)}).
                </p>
                <label className="field-pill" htmlFor="gg-code">
                  <input
                    id="gg-code"
                    value={ggCode}
                    onChange={(e) => setGgCode(e.target.value)}
                    placeholder="code from the redirect URL"
                    spellCheck={false}
                    autoComplete="off"
                    disabled={busy}
                  />
                </label>
                <label className="field-pill" htmlFor="gg-state">
                  <input
                    id="gg-state"
                    value={ggState}
                    onChange={(e) => setGgState(e.target.value)}
                    placeholder="state from the redirect URL"
                    spellCheck={false}
                    autoComplete="off"
                    disabled={busy}
                  />
                </label>
                <button
                  type="button"
                  className="btn btn-primary btn-block auth-submit"
                  onClick={() => void finishGoogle()}
                  disabled={busy}
                >
                  {busy ? "Verifying…" : "Finish Google sign-in"}
                </button>
              </>
            ) : (
              <button
                type="button"
                className="btn btn-primary btn-block auth-submit"
                onClick={() => void startGoogle()}
                disabled={busy || online === false}
              >
                {busy ? "Contacting Google…" : "Continue with Google"}
              </button>
            )}
            <button
              type="button"
              className="login-link"
              onClick={() => switchMethod("email")}
            >
              ← Use email instead
            </button>
          </div>
        ) : null}

        {method === "email" ? (
          <>
            <div className="auth-divider" aria-hidden="true">
              <span>or continue with</span>
            </div>
            <div className="provider-row">
              <button
                type="button"
                className="btn provider-btn"
                onClick={() => switchMethod("github")}
                disabled={busy}
              >
                <GithubIcon /> GitHub
              </button>
              <button
                type="button"
                className="btn provider-btn"
                onClick={() => switchMethod("google")}
                disabled={busy}
              >
                <GoogleIcon /> Google
              </button>
            </div>
            <p className="muted auth-foot">
              {emailMode === "signup" ? (
                <>
                  Already have an account?{" "}
                  <button
                    type="button"
                    className="login-link"
                    onClick={() => switchEmailMode("login")}
                  >
                    Log in
                  </button>
                </>
              ) : (
                <>
                  Didn&apos;t have an account?{" "}
                  <button
                    type="button"
                    className="login-link"
                    onClick={() => switchEmailMode("signup")}
                  >
                    Sign up
                  </button>
                </>
              )}
            </p>
          </>
        ) : null}

        {error ? (
          <div className="auth-error" role="alert">
            {error}
          </div>
        ) : null}
      </div>
      <div className="auth-mascot" aria-hidden="true">
        {showMascot ? <PixelGuardDog size={320} /> : null}
      </div>
    </div>
  );
}
