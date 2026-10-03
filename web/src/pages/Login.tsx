import * as React from "react";
import { useNavigate } from "react-router-dom";
import * as THREE from "three";
import { DeviceLogin } from "../components/DeviceLogin";
import { DitherBackground, type RGB } from "../components/Dither";
import { Seo } from "../components/Seo";
import { useMediaQuery } from "../hooks/useMediaQuery";
import { useRevealOnMount } from "../hooks/useRevealOnMount";
import {
  accountAuthError,
  beginAccountLogin,
  handleAccountCallback,
} from "../lib/accountAuth";
import { isAccountMode } from "../lib/billing";

/** Variant 1 token hexes for the waves (dark-only site: read once). */
function useLoginWaveColors(): {
  backgroundColor: RGB | undefined;
  color: RGB | undefined;
} {
  const readToken = React.useCallback((name: string): RGB | undefined => {
    const raw = getComputedStyle(document.documentElement)
      .getPropertyValue(name)
      .trim();
    if (!/^#[0-9a-f]{6}$/i.test(raw)) return undefined;
    // THREE.Color decodes sRGB into the linear working space.
    return new THREE.Color(raw).toArray() as RGB;
  }, []);
  const [colors] = React.useState(() => ({
    backgroundColor: readToken("--ag-bg"),
    color: readToken("--ag-brand"),
  }));
  return colors;
}

function AccountLogin(): JSX.Element {
  const navigate = useNavigate();
  const started = React.useRef(false);
  const [error, setError] = React.useState("");

  const signIn = React.useCallback(async () => {
    setError("");
    try {
      await beginAccountLogin();
    } catch (err) {
      setError(accountAuthError(err));
      started.current = false;
    }
  }, []);

  React.useEffect(() => {
    if (started.current) return;
    started.current = true;
    const callback = new URL(window.location.href);
    if (
      callback.searchParams.has("code") ||
      callback.searchParams.has("error")
    ) {
      void handleAccountCallback(callback.href)
        .then(() => navigate("/dashboard", { replace: true }))
        .catch((err: unknown) => {
          window.history.replaceState({}, "", callback.pathname);
          setError(accountAuthError(err));
          started.current = false;
        });
      return;
    }
    void signIn();
  }, [navigate, signIn]);

  return (
    <div className="card" style={{ maxWidth: 620, margin: "0 auto" }}>
      <p className="eyebrow">Algorithco account</p>
      <h1>Sign in to Guard</h1>
      <p className="muted">
        Guard uses the central Algorithco account service. Authentication opens
        in this top-level window and returns here securely.
      </p>
      {error ? <p className="notice notice-error">{error}</p> : null}
      <button
        type="button"
        className="btn btn-primary"
        onClick={() => void signIn()}
      >
        {error ? "Try again" : "Continue to Algorithco"}
      </button>
    </div>
  );
}

export default function Login(): JSX.Element {
  useRevealOnMount();
  const { backgroundColor, color } = useLoginWaveColors();
  // Perf ladder (matchMedia, updates on resize/orientation change):
  // small screens render cheaper, tiny/reduced-motion screens freeze.
  const smallScreen = useMediaQuery("(max-width: 767px)");
  const tinyScreen = useMediaQuery("(max-width: 480px)");
  const reduceMotion = useMediaQuery("(prefers-reduced-motion: reduce)");

  return (
    <>
      <Seo path="/login" />
      {backgroundColor && color ? (
        <div className="login-page-bg" aria-hidden="true">
          <DitherBackground
            wrapperClassName="login-bg-canvas"
            backgroundColor={backgroundColor}
            waveColor={color}
            waveSpeed={0.06}
            waveFrequency={2.2}
            waveAmplitude={0.32}
            colorNum={5}
            pixelSize={3}
            fps={smallScreen ? 24 : 30}
            disableAnimation={reduceMotion || tinyScreen}
            mouseRadius={0.45}
          />
        </div>
      ) : null}
      <section id="login" className="section reveal">
        <div className="wrap">
          {isAccountMode() ? <AccountLogin /> : <DeviceLogin />}
        </div>
      </section>
    </>
  );
}
