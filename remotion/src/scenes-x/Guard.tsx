import React from 'react';
import {AbsoluteFill, interpolate, spring, useCurrentFrame, useVideoConfig} from 'remotion';
import {C, F, VERDICT_COLOR, Verdict} from '../theme';
import {BlurWords, Card, Code, Logo, Pill, VerdictPill, Wordmark, clamp, useFadeUp} from '../ui';

const ROWS: Array<{v: Verdict; cmd: string; note: string}> = [
  {v: 'allow', cmd: 'ls -la', note: 'routine'},
  {v: 'ask', cmd: 'terraform apply', note: 'asks you'},
  {v: 'deny', cmd: 'rm -rf /', note: 'blocked'},
];

const Row: React.FC<{v: Verdict; cmd: string; note: string; start: number}> = ({v, cmd, note, start}) => {
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const s = spring({frame: frame - start, fps, config: {damping: 15, stiffness: 130}});
  const isDeny = v === 'deny';
  const k = frame - (start + 10);
  const shake = isDeny && k >= 0 && k < 12 ? Math.sin(k * 2.8) * 8 * (1 - k / 12) : 0;
  return (
    <div style={{opacity: s, transform: `translateX(${(1 - s) * 90 + shake}px)`}}>
      <Card glow={isDeny && frame > start + 8 ? C.deny : undefined} style={{padding: '22px 28px', display: 'flex', alignItems: 'center', gap: 22}}>
        <VerdictPill verdict={v} size={26} />
        <Code size={30}>{cmd}</Code>
        <span style={{flex: 1}} />
        <span style={{fontFamily: F.sans, fontSize: 24, color: VERDICT_COLOR[v]}}>{note}</span>
      </Card>
    </div>
  );
};

export const XGuard: React.FC = () => {
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const logo = spring({frame, fps, config: {damping: 13, stiffness: 110}});
  const foot = useFadeUp(96, 16, 12);
  return (
    <AbsoluteFill style={{padding: '84px 72px', justifyContent: 'center', gap: 34}}>
      <div style={{display: 'flex', alignItems: 'center', gap: 24, opacity: logo, transform: `scale(${0.6 + 0.4 * logo})`, transformOrigin: 'left center'}}>
        <Logo size={96} />
        <Wordmark size={46} />
      </div>
      <BlurWords
        lines={[{text: 'A checkpoint for'}, {text: 'every command.', accent: true}]}
        fontSize={84}
        start={12}
      />
      <div style={{display: 'flex', flexDirection: 'column', gap: 18, marginTop: 6}}>
        {ROWS.map((r, i) => (
          <Row key={r.cmd} {...r} start={44 + i * 18} />
        ))}
      </div>
      <div style={{...foot, display: 'flex', gap: 14}}>
        <Pill color={C.brand} size={24}>
          judged in &lt;3ms
        </Pill>
        <Pill color={C.brand} size={24}>
          reason always logged
        </Pill>
      </div>
    </AbsoluteFill>
  );
};
