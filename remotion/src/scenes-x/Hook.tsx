import React from 'react';
import {AbsoluteFill, interpolate, useCurrentFrame} from 'remotion';
import {C, F} from '../theme';
import {BlurWords, Pill, Terminal, clamp, typed, useFadeUp} from '../ui';

export const XHook: React.FC = () => {
  const frame = useCurrentFrame();
  const CMD = 'rm -rf /';
  const t = typed(CMD, frame, 42, 10);
  const ran = frame >= 42 + 26;
  const shakeAt = 68;
  const k = frame - shakeAt;
  const shake = k >= 0 && k < 14 ? Math.sin(k * 2.6) * 10 * (1 - k / 14) : 0;
  const flash = interpolate(frame, [shakeAt, shakeAt + 4, shakeAt + 22], [0, 0.22, 0], clamp);
  const foot = useFadeUp(80, 14, 10);
  return (
    <AbsoluteFill style={{padding: '84px 72px', justifyContent: 'center', gap: 44}}>
      <AbsoluteFill style={{background: C.deny, opacity: flash}} />
      <BlurWords
        lines={[
          {text: 'Your AI agent can'},
          {text: 'run any command.'},
          {text: 'Without asking.', color: C.deny},
        ]}
        fontSize={92}
        start={4}
        stagger={5}
      />
      <div style={{transform: `translateX(${shake}px)`, ...useFadeUp(28, 16, 20)}}>
        <Terminal title="agent session · claude code" width="100%" glow={ran ? C.deny : undefined}>
          <div style={{fontSize: 34, display: 'flex', alignItems: 'center', gap: 14}}>
            <span style={{color: ran ? C.deny : C.muted}}>●</span>
            <span style={{color: C.muted}}>Bash(</span>
            <span style={{color: ran ? C.deny : C.text}}>{t}</span>
            <span style={{color: C.muted}}>)</span>
            {ran ? (
              <span style={{marginLeft: 'auto'}}>
                <Pill color={C.deny} size={22} mono>
                  ran
                </Pill>
              </span>
            ) : null}
          </div>
          <div style={{...foot, marginTop: 18, fontSize: 24, color: C.muted, fontFamily: F.sans}}>
            no checkpoint. no record.
          </div>
        </Terminal>
      </div>
    </AbsoluteFill>
  );
};
