import * as React from "react";
import * as THREE from "three";
import { DeviceLogin } from "../components/DeviceLogin";
import { DitherBackground, type RGB } from "../components/Dither";
import { Seo } from "../components/Seo";
import { useMediaQuery } from "../hooks/useMediaQuery";
import { useRevealOnMount } from "../hooks/useRevealOnMount";

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
          <DeviceLogin />
        </div>
      </section>
    </>
  );
}
