import React from 'react';
import {useCurrentFrame} from 'remotion';
import {C, F} from '../theme';
import {BlurWords, Card, Code, Kicker, Logo, Pill, SceneFrame, VerdictPill, useFadeUp} from '../ui';

const AGENTS: Array<{name: string; now: boolean}> = [
  {name: 'Claude Code', now: true},
  {name: 'Codex CLI', now: false},
  {name: 'OpenCode', now: false},
];

const SHELL = [
  {v: 'allow' as const, cmd: 'ls -la'},
  {v: 'ask' as const, cmd: 'terraform apply'},
  {v: 'deny' as const, cmd: 'rm -rf /'},
];

const PILLARS = [
  {t: 'Safer', b: 'Hard-deny before it runs. Rules outrank models.'},
  {t: 'Quieter', b: 'Shadow-first. Quiet unless it needs you.'},
  {t: 'Auditable', b: 'algo why: action, reason, confidence, source, latency.'},
  {t: 'Integrations', b: 'Hooks, plugins, MCP. Claude Code now; Codex and OpenCode planned.'},
];

const Arrow: React.FC<{label: string; delay: number}> = ({label, delay}) => {
  const frame = useCurrentFrame();
  const s = useFadeUp(delay, 14, 0);
  const W = 170;
  const x = ((frame * 3.2 + delay * 7) % W) - 8;
  return (
    <div style={{...s, width: W, display: 'flex', flexDirection: 'column', alignItems: 'center'}}>
      <div style={{position: 'relative', height: 4, width: '100%', background: `${C.brand}55`, borderRadius: 4}}>
        <div
          style={{
            position: 'absolute',
            top: -6,
            left: x,
            width: 16,
            height: 16,
            borderRadius: 99,
            background: C.brandStrong,
            boxShadow: `0 0 18px ${C.brand}`,
          }}
        />
      </div>
      <div style={{marginTop: 18, fontSize: 18, color: C.muted, fontFamily: F.mono, whiteSpace: 'nowrap'}}>{label}</div>
    </div>
  );
};

export const What: React.FC = () => {
  return (
    <SceneFrame>
      <Kicker>What it is</Kicker>
      <BlurWords
        lines={[
          {text: 'An intelligent control layer'},
          {text: 'for CLI coding agents.', accent: true},
        ]}
        fontSize={68}
        start={6}
      />
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          gap: 26,
          marginTop: 44,
        }}
      >
        {/* agents */}
        <div style={useFadeUp(30, 18, 20)}>
          <Card style={{width: 400, padding: 28}}>
            <div style={{fontSize: 20, color: C.muted, letterSpacing: '0.1em', textTransform: 'uppercase', marginBottom: 16}}>
              CLI coding agents
            </div>
            {AGENTS.map((a) => (
              <div
                key={a.name}
                style={{display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '10px 0'}}
              >
                <span style={{fontSize: 30, fontWeight: 600}}>{a.name}</span>
                <Pill color={a.now ? C.allow : C.ask} size={18}>
                  {a.now ? 'now' : 'planned'}
                </Pill>
              </div>
            ))}
          </Card>
        </div>

        <Arrow label="hooks · plugins · MCP" delay={44} />

        {/* guard */}
        <div style={useFadeUp(56, 18, 20)}>
          <Card glow={C.brand} style={{width: 400, padding: 28, textAlign: 'center'}}>
            <div style={{display: 'flex', justifyContent: 'center'}}>
              <Logo size={92} />
            </div>
            <div style={{fontSize: 34, fontWeight: 700, marginTop: 14, letterSpacing: '-0.02em'}}>
              algorithco <span style={{color: C.brandStrong}}>guard</span>
            </div>
            <div style={{fontFamily: F.mono, fontSize: 20, color: C.muted, marginTop: 6}}>
              &lt;3ms local decisions
            </div>
            <div style={{display: 'flex', gap: 10, justifyContent: 'center', marginTop: 18}}>
              <VerdictPill verdict="allow" size={20} />
              <VerdictPill verdict="ask" size={20} />
              <VerdictPill verdict="deny" size={20} />
            </div>
          </Card>
        </div>

        <Arrow label="allow · ask · deny" delay={72} />

        {/* shell */}
        <div style={useFadeUp(84, 18, 20)}>
          <Card style={{width: 400, padding: 28}}>
            <div style={{fontSize: 20, color: C.muted, letterSpacing: '0.1em', textTransform: 'uppercase', marginBottom: 16}}>
              Your shell
            </div>
            {SHELL.map((s, i) => (
              <ShellRow key={s.cmd} {...s} start={96 + i * 14} />
            ))}
          </Card>
        </div>
      </div>

      <div style={{display: 'flex', gap: 24, marginTop: 44}}>
        {PILLARS.map((p, i) => (
          <PillarCard key={p.t} {...p} start={120 + i * 10} />
        ))}
      </div>
    </SceneFrame>
  );
};

const ShellRow: React.FC<{v: 'allow' | 'ask' | 'deny'; cmd: string; start: number}> = ({
  v,
  cmd,
  start,
}) => (
  <div style={{...useFadeUp(start, 12, 10), display: 'flex', alignItems: 'center', gap: 14, padding: '9px 0'}}>
    <VerdictPill verdict={v} size={18} />
    <Code size={22}>{cmd}</Code>
  </div>
);

const PillarCard: React.FC<{t: string; b: string; start: number}> = ({t, b, start}) => (
  <div style={{...useFadeUp(start, 16, 16), flex: 1}}>
    <Card style={{padding: '22px 24px', height: '100%'}}>
      <div style={{fontSize: 28, fontWeight: 700, color: C.brandStrong, marginBottom: 8}}>{t}</div>
      <div style={{fontSize: 21, lineHeight: 1.45, color: C.muted}}>{b}</div>
    </Card>
  </div>
);
