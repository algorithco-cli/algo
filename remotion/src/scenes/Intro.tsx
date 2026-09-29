import React from 'react';
import {AbsoluteFill, spring, useCurrentFrame, useVideoConfig} from 'remotion';
import {C} from '../theme';
import {BlurWords, Logo, Pill, Wordmark, useFadeUp} from '../ui';

export const Intro: React.FC = () => {
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const s = spring({frame, fps, config: {damping: 14, stiffness: 110}});
  const sub = useFadeUp(66, 20, 16);
  const meta = useFadeUp(88, 20, 12);
  return (
    <AbsoluteFill
      style={{alignItems: 'center', justifyContent: 'center', flexDirection: 'column', gap: 40}}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 26,
          opacity: s,
          transform: `scale(${0.6 + 0.4 * s})`,
        }}
      >
        <Logo size={104} />
        <Wordmark size={48} />
      </div>
      <BlurWords
        lines={[{text: "Ship agents you don't have"}, {text: 'to babysit.', accent: true}]}
        fontSize={108}
        start={22}
        align="center"
      />
      <p
        style={{
          ...sub,
          margin: 0,
          maxWidth: 1250,
          textAlign: 'center',
          fontSize: 36,
          lineHeight: 1.45,
          color: C.muted,
        }}
      >
        Guard judges every shell command — hard-deny, ask, or allow — in under 3ms, and logs the
        reason.
      </p>
      <div style={{...meta, display: 'flex', gap: 16}}>
        <Pill color={C.brand}>~30s install</Pill>
        <Pill color={C.brand}>&lt;3ms local decisions</Pill>
        <Pill color={C.brand}>local-only by default</Pill>
      </div>
    </AbsoluteFill>
  );
};
