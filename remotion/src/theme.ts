// Generated from the canonical root design-tokens.css by `npm run tokens`.
export {C} from './tokens.generated';
import {C} from './tokens.generated';

export const F = {
  sans: 'Inter, system-ui, -apple-system, "Segoe UI", sans-serif',
  mono: '"JetBrains Mono", ui-monospace, Menlo, Consolas, monospace',
} as const;

export type Verdict = 'allow' | 'ask' | 'deny';
export const VERDICT_COLOR: Record<Verdict, string> = {
  allow: C.allow,
  ask: C.ask,
  deny: C.deny,
};
