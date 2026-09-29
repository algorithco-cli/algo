import React from 'react';
import {AbsoluteFill, Sequence, interpolate, useCurrentFrame} from 'remotion';
import {TransitionSeries, linearTiming} from '@remotion/transitions';
import {fade} from '@remotion/transitions/fade';
import {C, F} from './theme';
import {X_SCENES, X_START, X_T, X_TOTAL} from './durations';
import {Background, Logo, clamp} from './ui';
import {XHook} from './scenes-x/Hook';
import {XGuard} from './scenes-x/Guard';
import {XHow} from './scenes-x/How';
import {XTrust} from './scenes-x/Trust';
import {XCta} from './scenes-x/Cta';

const Progress: React.FC = () => {
  const frame = useCurrentFrame();
  return (
    <div
      style={{
        position: 'absolute',
        left: 0,
        bottom: 0,
        height: 6,
        width: `${(frame / (X_TOTAL - 1)) * 100}%`,
        background: `linear-gradient(90deg, ${C.brandLogo}, ${C.brandStrong})`,
      }}
    />
  );
};

const Bug: React.FC = () => {
  const frame = useCurrentFrame();
  const o = interpolate(frame, [0, 12], [0, 1], clamp);
  return (
    <div style={{position: 'absolute', top: 34, right: 44, opacity: o * 0.9}}>
      <Logo size={40} />
    </div>
  );
};

const timing = linearTiming({durationInFrames: X_T});

export const GuardX: React.FC = () => (
  <AbsoluteFill style={{background: C.bg, fontFamily: F.sans, color: C.text}}>
    <Background />
    <TransitionSeries>
      <TransitionSeries.Sequence durationInFrames={X_SCENES.hook}>
        <XHook />
      </TransitionSeries.Sequence>
      <TransitionSeries.Transition presentation={fade()} timing={timing} />
      <TransitionSeries.Sequence durationInFrames={X_SCENES.guard}>
        <XGuard />
      </TransitionSeries.Sequence>
      <TransitionSeries.Transition presentation={fade()} timing={timing} />
      <TransitionSeries.Sequence durationInFrames={X_SCENES.how}>
        <XHow />
      </TransitionSeries.Sequence>
      <TransitionSeries.Transition presentation={fade()} timing={timing} />
      <TransitionSeries.Sequence durationInFrames={X_SCENES.trust}>
        <XTrust />
      </TransitionSeries.Sequence>
      <TransitionSeries.Transition presentation={fade()} timing={timing} />
      <TransitionSeries.Sequence durationInFrames={X_SCENES.cta}>
        <XCta />
      </TransitionSeries.Sequence>
    </TransitionSeries>
    <Sequence from={X_START.how} durationInFrames={X_START.cta - X_START.how + X_T}>
      <Bug />
    </Sequence>
    <Progress />
  </AbsoluteFill>
);
