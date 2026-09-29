import React from 'react';
import {interpolate, useCurrentFrame} from 'remotion';
import {INSTALL_CMD} from '../config';
import {C, F} from '../theme';
import {BlurWords, CheckIcon, Cursor, Kicker, SceneFrame, Terminal, clamp, typeDuration, typed, useFadeUp} from '../ui';

const STEPS = [
  {title: 'Install', body: 'One line. No account, no telemetry.'},
  {title: 'Run algo init', body: '~30s: detect agents → diff → consent → hooks.'},
  {title: 'Work as usual', body: 'Shadow mode logs what it would have blocked. Nothing is blocked yet.'},
  {title: 'Ask why — then enforce', body: 'algo why explains any decision. algo enforce on when you trust it.'},
];

const FIRST = 40;
const LEN = 74;

const Prompt: React.FC<{cmd: string; frame: number; cps?: number}> = ({cmd, frame, cps = 45}) => {
  const t = typed(cmd, frame, 4, cps);
  return (
    <div style={{marginBottom: 14, wordBreak: 'break-all'}}>
      <span style={{color: C.muted}}>$</span> {t}
      {t.length < cmd.length ? <Cursor height={28} /> : null}
    </div>
  );
};

const Line: React.FC<{at: number; frame: number; children: React.ReactNode}> = ({at, frame, children}) => (
  <div style={{opacity: interpolate(frame, [at, at + 8], [0, 1], clamp), transform: `translateX(${interpolate(frame, [at, at + 8], [-10, 0], clamp)}px)`}}>
    {children}
  </div>
);

const KV: React.FC<{k: string; v: React.ReactNode; color?: string}> = ({k, v, color = C.text}) => (
  <div style={{display: 'flex', gap: 24}}>
    <span style={{width: 150, color: C.muted}}>{k}</span>
    <span style={{color}}>{v}</span>
  </div>
);

const StepBody: React.FC<{idx: number; f: number}> = ({idx, f}) => {
  if (idx === 0) {
    const d = 4 + typeDuration(INSTALL_CMD, 45) + 6;
    return (
      <>
        <Prompt cmd={INSTALL_CMD} frame={f} />
        <Line at={d} frame={f}>
          <span style={{color: C.allow}}>✓</span> installed: algo · daemon · hook client
        </Line>
      </>
    );
  }
  if (idx === 1) {
    const lines = [
      'Detect agents',
      'Show diff · per-agent consent',
      'Backup + additive hook install',
      'Privacy prompt — local-only by default',
      'algo doctor',
    ];
    return (
      <>
        <Prompt cmd="algo init" frame={f} />
        {lines.map((l, i) => (
          <Line key={l} at={18 + i * 8} frame={f}>
            <span style={{color: C.allow}}>✓</span> {l}
          </Line>
        ))}
        <Line at={18 + lines.length * 8} frame={f}>
          <span style={{color: C.muted}}>done in ~30s</span>
        </Line>
      </>
    );
  }
  if (idx === 2) {
    return (
      <>
        <Prompt cmd="algo status" frame={f} />
        <Line at={16} frame={f}><KV k="mode" v="shadow — nothing is blocked" color={C.ask} /></Line>
        <Line at={24} frame={f}><KV k="allowed" v="128" color={C.allow} /></Line>
        <Line at={32} frame={f}><KV k="asked" v="6" color={C.ask} /></Line>
        <Line at={40} frame={f}><KV k="would-have" v="2 blocked" color={C.deny} /></Line>
      </>
    );
  }
  return (
    <>
      <Prompt cmd="algo why" frame={f} />
      <Line at={14} frame={f}><KV k="action" v="deny" color={C.deny} /></Line>
      <Line at={20} frame={f}><KV k="reason" v="DENY_CURL_PIPE_SH" /></Line>
      <Line at={26} frame={f}><KV k="confidence" v="1.00" /></Line>
      <Line at={32} frame={f}><KV k="source" v="rule" /></Line>
      <Line at={38} frame={f}><KV k="latency" v="~1ms" color={C.allow} /></Line>
      <div style={{marginTop: 14, opacity: interpolate(f, [46, 54], [0, 1], clamp)}}>
        <span style={{color: C.muted}}>$</span> algo enforce on
        <div><span style={{color: C.allow}}>✓</span> enforcing</div>
      </div>
    </>
  );
};

export const Usage: React.FC = () => {
  const frame = useCurrentFrame();
  const idx = Math.max(0, Math.min(3, Math.floor((frame - FIRST) / LEN)));
  const f = Math.max(0, frame - FIRST - idx * LEN);
  const swap = interpolate(f, [0, 8], [0, 1], clamp);
  return (
    <SceneFrame>
      <Kicker>How to use it</Kicker>
      <BlurWords
        lines={[{text: 'Four steps.'}, {text: 'About 30 seconds.', accent: true}]}
        fontSize={68}
        start={6}
      />
      <div style={{display: 'flex', gap: 56, marginTop: 40, flex: 1, minHeight: 0}}>
        <div style={{flex: 0.9, display: 'flex', flexDirection: 'column', gap: 18}}>
          {STEPS.map((s, i) => (
            <StepItem key={s.title} i={i} idx={idx} started={frame >= FIRST} {...s} />
          ))}
        </div>
        <div style={{flex: 1.1, ...useFadeUp(16, 18, 20)}}>
          <Terminal title="~ — zsh" width="100%" height={560}>
            <div key={idx} style={{opacity: frame < FIRST ? 0 : swap, fontSize: 27, lineHeight: 1.6}}>
              {frame >= FIRST ? <StepBody idx={idx} f={f} /> : null}
            </div>
          </Terminal>
        </div>
      </div>
    </SceneFrame>
  );
};

const StepItem: React.FC<{i: number; idx: number; started: boolean; title: string; body: string}> = ({
  i,
  idx,
  started,
  title,
  body,
}) => {
  const s = useFadeUp(14 + i * 6, 16, 16);
  const active = started && i === idx;
  const done = started && i < idx;
  return (
    <div
      style={{
        ...s,
        opacity: (s.opacity as number) * (active || !started ? 1 : done ? 0.75 : 0.4),
        display: 'flex',
        gap: 20,
        padding: '18px 22px',
        borderRadius: 18,
        background: active ? C.surface : 'transparent',
        border: `1px solid ${active ? C.brand : 'transparent'}`,
        boxShadow: active ? `0 0 44px ${C.brand}30` : 'none',
      }}
    >
      <div
        style={{
          flexShrink: 0,
          width: 46,
          height: 46,
          borderRadius: 99,
          display: 'grid',
          placeItems: 'center',
          fontFamily: F.mono,
          fontWeight: 700,
          fontSize: 22,
          color: active ? C.bg : C.muted,
          background: active ? C.brandStrong : done ? `${C.allow}22` : C.raised,
          border: `1px solid ${done ? C.allow : C.border}`,
        }}
      >
        {done ? <CheckIcon size={24} /> : i + 1}
      </div>
      <div>
        <div style={{fontSize: 32, fontWeight: 700, letterSpacing: '-0.02em'}}>{title}</div>
        <div style={{fontSize: 22, lineHeight: 1.45, color: C.muted, marginTop: 4}}>{body}</div>
      </div>
    </div>
  );
};
