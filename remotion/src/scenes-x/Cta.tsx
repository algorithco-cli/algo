import React from 'react';
import {AbsoluteFill, spring, useCurrentFrame, useVideoConfig} from 'remotion';
import {CONFIG, INSTALL_CMD} from '../config';
import {C, F} from '../theme';
import {BlurWords, Cursor, Logo, Pill, Terminal, Wordmark, typeDuration, typed, useFadeUp} from '../ui';

export const XCta: React.FC = () => {
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const a = typed(INSTALL_CMD, frame, 30, 60);
  const aDone = 30 + typeDuration(INSTALL_CMD, 60);
  const b = typed('algo init', frame, aDone + 12, 16);
  const bDone = aDone + 12 + typeDuration('algo init', 16);
  const btn = spring({frame: frame - 96, fps, config: {damping: 12, stiffness: 120}});
  const pulse = 0.5 + 0.5 * Math.sin(frame / 8);
  return (
    <AbsoluteFill style={{padding: '84px 72px', justifyContent: 'center', gap: 34}}>
      <div style={{...useFadeUp(0, 14, 10), display: 'flex', alignItems: 'center', gap: 20}}>
        <Logo size={72} />
        <Wordmark size={40} />
      </div>
      <BlurWords lines={[{text: 'Try it in'}, {text: '30 seconds.', accent: true}]} fontSize={100} start={4} />
      <div style={useFadeUp(20, 16, 18)}>
        <Terminal title="~ — zsh" width="100%">
          <div style={{fontSize: 22, lineHeight: 1.7, wordBreak: 'break-all'}}>
            <div>
              <span style={{color: C.muted}}>$</span> {a}
              {a.length < INSTALL_CMD.length ? <Cursor height={22} /> : null}
            </div>
            {frame >= aDone + 4 ? (
              <div style={{fontSize: 24}}>
                <span style={{color: C.muted}}>$</span> {b}
                {b.length < 9 ? <Cursor height={24} /> : null}
              </div>
            ) : null}
            {frame >= bDone + 6 ? (
              <div style={{fontSize: 24, color: C.allow, wordBreak: 'normal'}}>✓ hooks installed — only with your consent</div>
            ) : null}
          </div>
        </Terminal>
      </div>
      <div style={{display: 'flex', alignItems: 'center', gap: 22, opacity: btn, transform: `scale(${0.8 + 0.2 * btn})`, transformOrigin: 'left center'}}>
        <div
          style={{
            padding: '22px 52px',
            borderRadius: 999,
            fontSize: 38,
            fontWeight: 700,
            color: C.text,
            background: `linear-gradient(135deg, ${C.brandLogo}, ${C.brand})`,
            boxShadow: `0 0 ${40 + pulse * 36}px ${C.brand}88`,
          }}
        >
          {CONFIG.ctaLabel} →
        </div>
        <span style={{fontSize: 24, color: C.muted, lineHeight: 1.35}}>
          Claude Code today
          <br />
          Codex · OpenCode planned
        </span>
      </div>
    </AbsoluteFill>
  );
};
