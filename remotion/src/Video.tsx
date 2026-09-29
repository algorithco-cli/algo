import React from 'react';
import {AbsoluteFill, Sequence, interpolate, useCurrentFrame} from 'remotion';
import {TransitionSeries, linearTiming} from '@remotion/transitions';
import {fade} from '@remotion/transitions/fade';
import {C, F} from './theme';
import {SCENES, START, T, TOTAL} from './durations';
import {Background, Logo, Wordmark, clamp} from './ui';
import {Intro} from './scenes/Intro';
import {Problem} from './scenes/Problem';
import {What} from './scenes/What';
import {Pipeline} from './scenes/Pipeline';
import {Demo} from './scenes/Demo';
import {Usage} from './scenes/Usage';
import {Cta} from './scenes/Cta';

const BrandBug: React.FC = () => {
  const frame = useCurrentFrame();
  const o = interpolate(frame, [0, 14], [0, 1], clamp);
  return (
    <div style={{position: 'absolute', top: 44, left: 64, display: 'flex', alignItems: 'center', gap: 14, opacity: o}}>
      <Logo size={44} />
      <Wordmark size={26} />
    </div>
  );
};

const Progress: React.FC = () => {
  const frame = useCurrentFrame();
  return (
    <div
      style={{
        position: 'absolute',
        left: 0,
        bottom: 0,
        height: 5,
        width: `${(frame / (TOTAL - 1)) * 100}%`,
        background: `linear-gradient(90deg, ${C.brandLogo}, ${C.brandStrong})`,
      }}
    />
  );
};

const timing = linearTiming({durationInFrames: T});

export const GuardAd: React.FC = () => (
  <AbsoluteFill style={{background: C.bg, fontFamily: F.sans, color: C.text}}>
    <Background />
    <TransitionSeries>
      <TransitionSeries.Sequence durationInFrames={SCENES.intro}>
        <Intro />
      </TransitionSeries.Sequence>
      <TransitionSeries.Transition presentation={fade()} timing={timing} />
      <TransitionSeries.Sequence durationInFrames={SCENES.problem}>
        <Problem />
      </TransitionSeries.Sequence>
      <TransitionSeries.Transition presentation={fade()} timing={timing} />
      <TransitionSeries.Sequence durationInFrames={SCENES.what}>
        <What />
      </TransitionSeries.Sequence>
      <TransitionSeries.Transition presentation={fade()} timing={timing} />
      <TransitionSeries.Sequence durationInFrames={SCENES.pipeline}>
        <Pipeline />
      </TransitionSeries.Sequence>
      <TransitionSeries.Transition presentation={fade()} timing={timing} />
      <TransitionSeries.Sequence durationInFrames={SCENES.demo}>
        <Demo />
      </TransitionSeries.Sequence>
      <TransitionSeries.Transition presentation={fade()} timing={timing} />
      <TransitionSeries.Sequence durationInFrames={SCENES.usage}>
        <Usage />
      </TransitionSeries.Sequence>
      <TransitionSeries.Transition presentation={fade()} timing={timing} />
      <TransitionSeries.Sequence durationInFrames={SCENES.cta}>
        <Cta />
      </TransitionSeries.Sequence>
    </TransitionSeries>
    {/* persistent brand bug, hidden on the intro and CTA (which show the big logo) */}
    <Sequence from={START.problem} durationInFrames={START.cta - START.problem + T}>
      <BrandBug />
    </Sequence>
    <Progress />
  </AbsoluteFill>
);
