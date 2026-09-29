import {staticFile} from 'remotion';
import {loadFont} from '@remotion/fonts';

// loadFont() blocks rendering until the font is ready.
loadFont({
  family: 'Inter',
  url: staticFile('fonts/inter-latin.woff2'),
  weight: '100 900',
});
loadFont({
  family: 'JetBrains Mono',
  url: staticFile('fonts/jetbrains-mono-latin.woff2'),
  weight: '100 800',
});
