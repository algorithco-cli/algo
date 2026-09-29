import React from 'react';
import {
  AbsoluteFill,
  Easing,
  interpolate,
  spring,
  useCurrentFrame,
  useVideoConfig,
} from 'remotion';
import {C, F, VERDICT_COLOR, Verdict} from './theme';

export const clamp = {
  extrapolateLeft: 'clamp',
  extrapolateRight: 'clamp',
} as const;

/** Fade + rise, returns a style object. */
export const useFadeUp = (start: number, dur = 18, dist = 24): React.CSSProperties => {
  const frame = useCurrentFrame();
  const p = interpolate(frame, [start, start + dur], [0, 1], {
    ...clamp,
    easing: Easing.out(Easing.cubic),
  });
  return {opacity: p, transform: `translateY(${(1 - p) * dist}px)`};
};

/** How many characters of `text` are typed at `frame`. */
export const typed = (text: string, frame: number, start: number, cps = 40, fps = 30): string =>
  text.slice(0, Math.max(0, Math.min(text.length, Math.floor(((frame - start) / fps) * cps))));

export const typeDuration = (text: string, cps = 40, fps = 30): number =>
  Math.ceil((text.length / cps) * fps);

// ───────────────────────── Logo ─────────────────────────
export const Logo: React.FC<{size: number}> = ({size}) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 256 256"
    style={{filter: `drop-shadow(0 0 ${size * 0.25}px rgba(142,119,255,0.45))`}}
  >
    <rect width="256" height="256" rx="58" fill={C.bg} />
    <polyline
      points="68,70 130,128 68,186"
      fill="none"
      stroke={C.text}
      strokeWidth={34}
      strokeLinecap="round"
      strokeLinejoin="round"
    />
    <rect x="160" y="60" width="36" height="62" rx="18" fill={C.brandLogo} />
    <rect x="160" y="134" width="36" height="62" rx="18" fill={C.brandLogo} />
  </svg>
);

export const Wordmark: React.FC<{size: number}> = ({size}) => (
  <span
    style={{
      fontSize: size,
      fontWeight: 700,
      letterSpacing: '-0.02em',
      color: C.text,
    }}
  >
    algorithco <span style={{color: C.brandStrong}}>guard</span>
  </span>
);

// ───────────────────────── Background ─────────────────────────
export const Background: React.FC = () => {
  const frame = useCurrentFrame();
  const t = frame / 30;
  const x1 = 28 + 10 * Math.sin(t * 0.5);
  const y1 = 18 + 8 * Math.cos(t * 0.4);
  const x2 = 78 + 8 * Math.cos(t * 0.35);
  const y2 = 88 + 6 * Math.sin(t * 0.5);
  return (
    <AbsoluteFill style={{background: C.bg}}>
      <AbsoluteFill
        style={{
          background: `radial-gradient(1000px 700px at ${x1}% ${y1}%, rgba(142,119,255,0.26), transparent 62%), radial-gradient(900px 700px at ${x2}% ${y2}%, rgba(166,143,255,0.14), transparent 62%)`,
        }}
      />
      <AbsoluteFill
        style={{
          backgroundImage: `linear-gradient(${C.border}66 1px, transparent 1px), linear-gradient(90deg, ${C.border}66 1px, transparent 1px)`,
          backgroundSize: '64px 64px',
          backgroundPosition: `${-(frame * 0.25) % 64}px ${-(frame * 0.25) % 64}px`,
          WebkitMaskImage: 'radial-gradient(ellipse at center, black 20%, transparent 72%)',
          maskImage: 'radial-gradient(ellipse at center, black 20%, transparent 72%)',
          opacity: 0.55,
        }}
      />
    </AbsoluteFill>
  );
};

// ───────────────────────── Text ─────────────────────────
export const Kicker: React.FC<{children: React.ReactNode; delay?: number}> = ({
  children,
  delay = 0,
}) => {
  const s = useFadeUp(delay, 16, 10);
  return (
    <div
      style={{
        ...s,
        fontSize: 22,
        fontWeight: 600,
        letterSpacing: '0.18em',
        textTransform: 'uppercase',
        color: C.brand,
        marginBottom: 14,
      }}
    >
      {children}
    </div>
  );
};

type Line = {text: string; accent?: boolean; color?: string};

/** Word-by-word blur-in headline (mirrors the site's BlurText hero). */
export const BlurWords: React.FC<{
  lines: Line[];
  fontSize: number;
  start?: number;
  stagger?: number;
  align?: 'left' | 'center';
}> = ({lines, fontSize, start = 0, stagger = 4, align = 'left'}) => {
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  let idx = 0;
  return (
    <div
      style={{
        fontSize,
        fontWeight: 700,
        letterSpacing: '-0.03em',
        lineHeight: 1.08,
        textAlign: align,
      }}
    >
      {lines.map((line, li) => (
        <div key={li}>
          {line.text.split(' ').map((w, wi) => {
            const p = spring({
              frame: frame - start - idx++ * stagger,
              fps,
              config: {damping: 200},
            });
            return (
              <span
                key={wi}
                style={{
                  display: 'inline-block',
                  marginRight: '0.26em',
                  opacity: p,
                  filter: `blur(${(1 - p) * 14}px)`,
                  transform: `translateY(${-(1 - p) * 26}px)`,
                  color: line.color ?? (line.accent ? C.brandStrong : C.text),
                }}
              >
                {w}
              </span>
            );
          })}
        </div>
      ))}
    </div>
  );
};

export const Code: React.FC<{children: React.ReactNode; size?: number; color?: string}> = ({
  children,
  size = 26,
  color = C.text,
}) => (
  <code
    style={{
      fontFamily: F.mono,
      fontSize: size,
      color,
      background: C.surface,
      border: `1px solid ${C.border}`,
      borderRadius: 8,
      padding: '2px 10px',
    }}
  >
    {children}
  </code>
);

export const Pill: React.FC<{
  color: string;
  children: React.ReactNode;
  size?: number;
  mono?: boolean;
  solid?: boolean;
}> = ({color, children, size = 22, mono = false, solid = false}) => (
  <span
    style={{
      display: 'inline-flex',
      alignItems: 'center',
      gap: 8,
      fontFamily: mono ? F.mono : F.sans,
      fontSize: size,
      fontWeight: 600,
      color: solid ? C.bg : color,
      background: solid ? color : `${color}22`,
      border: `1px solid ${color}66`,
      borderRadius: 999,
      padding: `${size * 0.18}px ${size * 0.6}px`,
      whiteSpace: 'nowrap',
    }}
  >
    {children}
  </span>
);

export const VerdictPill: React.FC<{verdict: Verdict; size?: number}> = ({verdict, size = 24}) => (
  <Pill color={VERDICT_COLOR[verdict]} size={size} mono>
    <span
      style={{
        width: size * 0.4,
        height: size * 0.4,
        borderRadius: 99,
        background: VERDICT_COLOR[verdict],
      }}
    />
    {verdict}
  </Pill>
);

// ───────────────────────── Cards & terminal ─────────────────────────
export const Card: React.FC<{
  children: React.ReactNode;
  style?: React.CSSProperties;
  glow?: string;
}> = ({children, style, glow}) => (
  <div
    style={{
      background: C.surface,
      border: `1px solid ${glow ? `${glow}88` : C.border}`,
      borderRadius: 20,
      boxShadow: glow ? `0 0 60px ${glow}33` : '0 20px 60px rgba(0,0,0,0.35)',
      ...style,
    }}
  >
    {children}
  </div>
);

export const Cursor: React.FC<{color?: string; height?: number}> = ({
  color = C.brandStrong,
  height = 28,
}) => {
  const frame = useCurrentFrame();
  return (
    <span
      style={{
        display: 'inline-block',
        width: 14,
        height,
        marginLeft: 4,
        verticalAlign: 'text-bottom',
        background: color,
        opacity: Math.floor(frame / 15) % 2 === 0 ? 1 : 0,
      }}
    />
  );
};

export const Terminal: React.FC<{
  title: string;
  badge?: string;
  width?: number | string;
  height?: number | string;
  status?: [string, string];
  glow?: string;
  children: React.ReactNode;
  style?: React.CSSProperties;
}> = ({title, badge, width, height, status, glow, children, style}) => (
  <div
    style={{
      width,
      height,
      display: 'flex',
      flexDirection: 'column',
      background: C.raised,
      border: `1px solid ${glow ? `${glow}99` : C.border}`,
      borderRadius: 20,
      overflow: 'hidden',
      boxShadow: glow
        ? `0 0 80px ${glow}40, 0 30px 80px rgba(0,0,0,0.5)`
        : '0 30px 80px rgba(0,0,0,0.5)',
      ...style,
    }}
  >
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 16,
        padding: '16px 24px',
        background: C.surface,
        borderBottom: `1px solid ${C.border}`,
      }}
    >
      <span style={{display: 'flex', gap: 9}}>
        {[C.deny, C.ask, C.allow].map((c) => (
          <i key={c} style={{width: 14, height: 14, borderRadius: 99, background: c}} />
        ))}
      </span>
      <span style={{fontFamily: F.mono, fontSize: 20, color: C.muted}}>{title}</span>
      <span style={{flex: 1}} />
      {badge ? (
        <span style={{fontFamily: F.mono, fontSize: 20, color: C.brandStrong}}>⚡ {badge}</span>
      ) : null}
    </div>
    <div
      style={{
        flex: 1,
        padding: '28px 32px',
        fontFamily: F.mono,
        fontSize: 28,
        lineHeight: 1.55,
        color: C.text,
        minHeight: 0,
      }}
    >
      {children}
    </div>
    {status ? (
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          padding: '12px 24px',
          background: C.surface,
          borderTop: `1px solid ${C.border}`,
          fontFamily: F.mono,
          fontSize: 18,
          color: C.muted,
        }}
      >
        <span>
          <span style={{color: C.allow}}>●</span> {status[0]}
        </span>
        <span>{status[1]}</span>
      </div>
    ) : null}
  </div>
);

export const XIcon: React.FC<{size?: number; color?: string}> = ({size = 28, color = C.deny}) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={color} strokeWidth={2.6} strokeLinecap="round">
    <path d="M6 6l12 12M18 6L6 18" />
  </svg>
);

export const CheckIcon: React.FC<{size?: number; color?: string}> = ({size = 28, color = C.allow}) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={color} strokeWidth={2.8} strokeLinecap="round" strokeLinejoin="round">
    <path d="M5 12.5l4.5 4.5L19 7.5" />
  </svg>
);

export const SceneFrame: React.FC<{children: React.ReactNode; style?: React.CSSProperties}> = ({
  children,
  style,
}) => (
  <AbsoluteFill
    style={{
      padding: '132px 120px 70px',
      display: 'flex',
      flexDirection: 'column',
      ...style,
    }}
  >
    {children}
  </AbsoluteFill>
);
