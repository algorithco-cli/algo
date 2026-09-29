import React from 'react';
import {Composition} from 'remotion';
import './fonts';
import {GuardAd} from './Video';
import {GuardX} from './VideoX';
import {Guard15} from './Video15';
import {FPS, TOTAL, X_TOTAL} from './durations';

export const Root: React.FC = () => (
  <>
    <Composition
      id="Guard15"
      component={Guard15}
      durationInFrames={450}
      fps={30}
      width={1920}
      height={1080}
    />
    <Composition
      id="GuardAd"
      component={GuardAd}
      durationInFrames={TOTAL}
      fps={FPS}
      width={1920}
      height={1080}
    />
    <Composition
      id="GuardX"
      component={GuardX}
      durationInFrames={X_TOTAL}
      fps={FPS}
      width={1080}
      height={1080}
    />
  </>
);
