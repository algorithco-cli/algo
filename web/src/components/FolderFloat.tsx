import Matter from "matter-js";
import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import type {
  CSSProperties,
  KeyboardEvent as ReactKeyboardEvent,
  PointerEvent as ReactPointerEvent,
} from "react";
import "./FolderFloat.css";

const { Bodies, Body, Composite, Engine } = Matter;

export interface FolderFloatItem {
  label: string;
  value: string;
}

export interface FolderFloatProps {
  items?: (string | FolderFloatItem)[];
  label?: string;
  sublabel?: string;
  trigger?: "hover" | "click";
  defaultOpen?: boolean;
  closeOnSelect?: boolean;
  physics?: boolean;
  drift?: number;
  onSelect?: (value: string, index: number) => void;
  onOpenChange?: (open: boolean) => void;
  folderColor?: string;
  frontColor?: string;
  paperColor?: string;
  itemColor?: string;
  itemTextColor?: string;
  labelColor?: string;
  width?: number;
  height?: number;
  radius?: number;
  spread?: number;
  lift?: number;
  tilt?: number;
  flapAngle?: number;
  restAngle?: number;
  openDuration?: number;
  stagger?: number;
  bounce?: number;
  className?: string;
}

interface PillSize {
  w: number;
  h: number;
}

interface PillPos {
  x: number;
  y: number;
  r: number;
}

interface FloatZone {
  left: number;
  right: number;
  top: number;
  bottom: number;
}

interface DragState {
  i: number;
  id: number;
  dx: number;
  dy: number;
  sx: number;
  sy: number;
  moved: boolean;
}

interface WorldState {
  engine: Matter.Engine | null;
  bodies: Matter.Body[];
  sizes: PillSize[];
  raf: number;
  last: number;
  t0: number;
  drag: DragState | null;
  zone: FloatZone | null;
  live: boolean;
}

interface LatestState {
  onSelect?: (value: string, index: number) => void;
  onOpenChange?: (open: boolean) => void;
  drift: number;
  reduce: boolean;
}

const DEFAULT_ITEMS: FolderFloatItem[] = [
  { label: "Try a warmer palette", value: "palette" },
  { label: "Tighten the spacing", value: "spacing" },
  { label: "Logo feels small", value: "logo" },
  { label: "Love the new hero", value: "hero" },
];
const PAD = 28;
const CHAR = 6.8;
const GAP = 12;
const ROW = 52;
const DRAG_MIN = 4;
const ZONE_PAD = 8;

const jitter = (i: number): number => {
  const x = Math.sin(i * 12.9898 + 4.1414) * 43758.5453;
  return x - Math.floor(x);
};

const layout = (
  list: FolderFloatItem[],
  spread: number,
  lift: number,
  tilt: number,
  sizes: (PillSize | null)[],
): PillPos[] => {
  const rows: { items: { i: number; pw: number }[]; width: number }[] = [];
  let row: { i: number; pw: number }[] = [];
  let width = 0;
  list.forEach((item, i) => {
    const pw = sizes[i]?.w ?? PAD + item.label.length * CHAR;
    if (row.length > 0 && width + GAP + pw > spread * 2) {
      rows.push({ items: row, width });
      row = [];
      width = 0;
    }
    row.push({ i, pw });
    width += (row.length > 1 ? GAP : 0) + pw;
  });
  if (row.length > 0) rows.push({ items: row, width });
  const pos: PillPos[] = new Array(list.length);
  rows.forEach((r, ri) => {
    let x = -r.width / 2;
    const shift = (ri % 2 === 0 ? -1 : 1) * Math.min(16, spread * 0.1);
    r.items.forEach(({ i, pw }) => {
      const j = jitter(i);
      pos[i] = {
        x: x + pw / 2 + shift + (j - 0.5) * 6,
        y: -lift - ri * ROW - j * 6,
        r: tilt * (j * 2 - 1),
      };
      x += pw + GAP;
    });
  });
  return pos;
};

export function FolderFloat({
  items = DEFAULT_ITEMS,
  label = "Design feedback",
  sublabel = "",
  trigger = "hover",
  defaultOpen = false,
  closeOnSelect = true,
  physics = true,
  drift = 0.5,
  onSelect,
  onOpenChange,
  folderColor = "#3f3f46",
  frontColor = "#52525b",
  paperColor = "#f5f5f5",
  itemColor = "#f5f5f5",
  itemTextColor = "#18181b",
  labelColor = "#f5f5f5",
  width = 200,
  height = 148,
  radius = 14,
  spread = 180,
  lift = 26,
  tilt = 8,
  flapAngle = 34,
  restAngle = 16,
  openDuration = 520,
  stagger = 45,
  bounce = 0.3,
  className = "",
}: FolderFloatProps): JSX.Element {
  const [open, setOpen] = useState(defaultOpen);
  const [popped, setPopped] = useState(-1);
  const [live, setLive] = useState(false);
  const [sizes, setSizes] = useState<(PillSize | null)[]>([]);
  const anchorRef = useRef<HTMLDivElement | null>(null);
  const pillRefs = useRef<(HTMLButtonElement | null)[]>([]);
  const world = useRef<WorldState>({
    engine: null,
    bodies: [],
    sizes: [],
    raf: 0,
    last: 0,
    t0: 0,
    drag: null,
    zone: null,
    live: false,
  });
  const latest = useRef<LatestState>({ drift, reduce: false });
  latest.current = {
    onSelect,
    onOpenChange,
    drift,
    reduce: latest.current.reduce,
  };
  const popTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const liveTimer = useRef<ReturnType<typeof setTimeout> | undefined>(
    undefined,
  );
  const list: FolderFloatItem[] = items.map((item) =>
    typeof item === "string" ? { label: item, value: item } : item,
  );
  const n = list.length;
  const sub = sublabel || `${n} ${n === 1 ? "note" : "notes"}`;
  const pos = layout(list, spread, lift, tilt, sizes);
  const posKey = pos
    .map((p) => `${p.x.toFixed(1)},${p.y.toFixed(1)}`)
    .join("|");

  const labelsKey = list.map((item) => item.label).join("|");
  // biome-ignore lint/correctness/useExhaustiveDependencies: measure intentionally keyed on count+labels only
  useLayoutEffect(() => {
    const measure = (): void => {
      const next: (PillSize | null)[] = pillRefs.current
        .slice(0, n)
        .map((el) => (el ? { w: el.offsetWidth, h: el.offsetHeight } : null));
      if (next.some((s) => s === null)) return;
      setSizes((prev) =>
        prev.length === next.length &&
        prev.every((s, i) => s?.w === next[i]?.w && s?.h === next[i]?.h)
          ? prev
          : next,
      );
    };
    measure();
    document.fonts?.ready.then(measure).catch(() => undefined);
  }, [n, labelsKey]);

  const stopPhysics = useCallback(() => {
    const w = world.current;
    clearTimeout(liveTimer.current);
    cancelAnimationFrame(w.raf);
    w.raf = 0;
    if (w.engine) {
      w.bodies.forEach((b, i) => {
        const el = pillRefs.current[i];
        if (!el) return;
        el.style.setProperty("--x", `${b.position.x.toFixed(1)}px`);
        el.style.setProperty(
          "--y",
          `${(b.position.y - (w.sizes[i]?.h ?? 0) / 2).toFixed(1)}px`,
        );
      });
      Composite.clear(w.engine.world, false, true);
      Engine.clear(w.engine);
      w.engine = null;
    }
    w.bodies = [];
    w.drag = null;
    w.live = false;
    setLive(false);
  }, []);

  // biome-ignore lint/correctness/useExhaustiveDependencies: restarts intentionally keyed on layout-affecting inputs only
  const startPhysics = useCallback(() => {
    const w = world.current;
    if (w.engine || n === 0) return;
    const els = pillRefs.current.slice(0, n);
    if (els.some((el) => el === null)) return;
    const engine = Engine.create({ gravity: { x: 0, y: 0 } });
    engine.enableSleeping = false;
    w.engine = engine;
    w.sizes = (els as HTMLButtonElement[]).map((el) => ({
      w: el.offsetWidth,
      h: el.offsetHeight,
    }));
    const ys = pos.map((p) => p.y);
    const zone: FloatZone = {
      left: -spread - ZONE_PAD,
      right: spread + ZONE_PAD,
      top: Math.min(...ys) - ZONE_PAD,
      bottom: -lift + Math.max(...w.sizes.map((s) => s.h)),
    };
    w.zone = zone;
    w.bodies = (els as HTMLButtonElement[]).map((el, i) => {
      const bw = w.sizes[i]?.w ?? 0;
      const bh = w.sizes[i]?.h ?? 0;
      const b = Bodies.rectangle(
        pos[i]?.x ?? 0,
        (pos[i]?.y ?? 0) + bh / 2,
        bw,
        bh,
        {
          chamfer: { radius: Math.min(bh / 2 - 1, 16) },
          restitution: 0.55,
          friction: 0,
          frictionAir: 0.08,
          inertia: Number.POSITIVE_INFINITY,
        },
      );
      b.plugin = { phase: jitter(i) * Math.PI * 2 };
      return b;
    });
    const T = 80;
    const walls = [
      Bodies.rectangle(
        (zone.left + zone.right) / 2,
        zone.top - T / 2,
        zone.right - zone.left + 2 * T,
        T,
        { isStatic: true },
      ),
      Bodies.rectangle(
        (zone.left + zone.right) / 2,
        zone.bottom + T / 2,
        zone.right - zone.left + 2 * T,
        T,
        { isStatic: true },
      ),
      Bodies.rectangle(
        zone.left - T / 2,
        (zone.top + zone.bottom) / 2,
        T,
        zone.bottom - zone.top + 2 * T,
        { isStatic: true },
      ),
      Bodies.rectangle(
        zone.right + T / 2,
        (zone.top + zone.bottom) / 2,
        T,
        zone.bottom - zone.top + 2 * T,
        { isStatic: true },
      ),
    ];
    Composite.add(engine.world, [...w.bodies, ...walls]);
    w.live = true;
    w.last = 0;
    w.t0 = performance.now();
    setLive(true);
    const tick = (now: number): void => {
      const s = world.current;
      if (!s.engine) return;
      const dt = s.last > 0 ? Math.min(32, now - s.last) : 16;
      s.last = now;
      const t = (now - s.t0) / 1000;
      const k = latest.current.drift * 0.00005 * Math.min(1, t / 2);
      s.bodies.forEach((b, i) => {
        if (s.drag && s.drag.i === i) return;
        const ph = typeof b.plugin.phase === "number" ? b.plugin.phase : 0;
        Body.applyForce(b, b.position, {
          x: Math.sin(t * 0.9 + ph) * k * b.mass,
          y: Math.cos(t * 1.3 + ph * 1.7) * k * b.mass,
        });
      });
      Engine.update(s.engine, dt);
      s.bodies.forEach((b, i) => {
        const el = pillRefs.current[i];
        if (!el) return;
        el.style.setProperty("--x", `${b.position.x.toFixed(1)}px`);
        el.style.setProperty(
          "--y",
          `${(b.position.y - (s.sizes[i]?.h ?? 0) / 2).toFixed(1)}px`,
        );
      });
      s.raf = requestAnimationFrame(tick);
    };
    w.raf = requestAnimationFrame(tick);
  }, [n, spread, lift, posKey]);

  const set = useCallback(
    (next: boolean) => {
      if (!next) stopPhysics();
      setOpen((prev) => {
        if (prev === next) return prev;
        latest.current.onOpenChange?.(next);
        return next;
      });
    },
    [stopPhysics],
  );

  useEffect(() => {
    clearTimeout(liveTimer.current);
    if (!open || !physics || latest.current.reduce) {
      if (!open || !physics) stopPhysics();
      return undefined;
    }
    liveTimer.current = setTimeout(
      startPhysics,
      openDuration + (n - 1) * stagger + 80,
    );
    return () => clearTimeout(liveTimer.current);
  }, [open, physics, openDuration, stagger, n, startPhysics, stopPhysics]);

  useEffect(() => {
    const mq = window.matchMedia("(prefers-reduced-motion: reduce)");
    const sync = (): void => {
      latest.current.reduce = mq.matches;
    };
    sync();
    mq.addEventListener("change", sync);
    return () => mq.removeEventListener("change", sync);
  }, []);

  useEffect(
    () => () => {
      clearTimeout(popTimer.current);
      stopPhysics();
    },
    [stopPhysics],
  );

  const pick = (item: FolderFloatItem, i: number): void => {
    latest.current.onSelect?.(item.value, i);
    clearTimeout(popTimer.current);
    setPopped(i);
    popTimer.current = setTimeout(() => setPopped(-1), 320);
    if (closeOnSelect) set(false);
  };

  const pointerAt = (
    e: ReactPointerEvent<HTMLButtonElement>,
  ): { x: number; y: number } => {
    const r = anchorRef.current?.getBoundingClientRect();
    return r ? { x: e.clientX - r.left, y: e.clientY - r.top } : { x: 0, y: 0 };
  };
  const down = (e: ReactPointerEvent<HTMLButtonElement>, i: number): void => {
    const w = world.current;
    if (!w.live || e.button !== 0) return;
    const b = w.bodies[i];
    if (!b) return;
    const p = pointerAt(e);
    w.drag = {
      i,
      id: e.pointerId,
      dx: b.position.x - p.x,
      dy: b.position.y - p.y,
      sx: e.clientX,
      sy: e.clientY,
      moved: false,
    };
    try {
      e.currentTarget.setPointerCapture(e.pointerId);
    } catch {
      /* pointer capture unsupported — click still works */
    }
  };
  const move = (e: ReactPointerEvent<HTMLButtonElement>, i: number): void => {
    const w = world.current;
    const d = w.drag;
    if (!d || d.i !== i || d.id !== e.pointerId) return;
    if (
      !d.moved &&
      Math.hypot(e.clientX - d.sx, e.clientY - d.sy) >= DRAG_MIN
    ) {
      d.moved = true;
      e.currentTarget.setAttribute("data-drag", "");
    }
    if (!d.moved) return;
    const b = w.bodies[i];
    const bw = w.sizes[i]?.w ?? 0;
    const bh = w.sizes[i]?.h ?? 0;
    const z = w.zone;
    if (!b || !z) return;
    const p = pointerAt(e);
    const x = Math.min(z.right - bw / 2, Math.max(z.left + bw / 2, p.x + d.dx));
    const y = Math.min(z.bottom - bh / 2, Math.max(z.top + bh / 2, p.y + d.dy));
    Body.setVelocity(b, {
      x: (x - b.position.x) * 0.6,
      y: (y - b.position.y) * 0.6,
    });
    Body.setPosition(b, { x, y });
  };
  const up = (
    e: ReactPointerEvent<HTMLButtonElement>,
    i: number,
    item: FolderFloatItem,
  ): void => {
    const w = world.current;
    const d = w.drag;
    if (!d || d.i !== i || d.id !== e.pointerId) return;
    w.drag = null;
    e.currentTarget.removeAttribute("data-drag");
    try {
      e.currentTarget.releasePointerCapture(e.pointerId);
    } catch {
      /* pointer capture unsupported — click still works */
    }
    if (!d.moved && e.type === "pointerup") pick(item, i);
  };

  const hover = trigger === "hover";

  const rootStyle = {
    "--ff-w": `${width}px`,
    "--ff-h": `${height}px`,
    "--ff-r": `${radius}px`,
    "--ff-back": folderColor,
    "--ff-front": frontColor,
    "--ff-paper": paperColor,
    "--ff-item": itemColor,
    "--ff-item-ink": itemTextColor,
    "--ff-label": labelColor,
    "--ff-spread": `${spread}px`,
    "--ff-lift": `${lift}px`,
    "--ff-angle": `${flapAngle}deg`,
    "--ff-rest": `${restAngle}deg`,
    "--ff-open": `${openDuration}ms`,
    "--ff-close": `${Math.round(openDuration * 0.6)}ms`,
    "--ff-stagger": `${stagger}ms`,
    "--ff-n": n,
    "--ff-spring": `cubic-bezier(0.34, ${(1 + bounce * 1.9).toFixed(2)}, 0.64, 1)`,
  } as CSSProperties;

  const onFolderKeyDown = (e: ReactKeyboardEvent<HTMLDivElement>): void => {
    if (e.key === "Escape" && open) {
      e.stopPropagation();
      set(false);
    }
  };

  return (
    <div
      className={`folder-float${className ? ` ${className}` : ""}`}
      data-open={open ? "" : undefined}
      data-live={live ? "" : undefined}
      data-physics={physics ? "" : undefined}
      data-trigger={trigger}
      onPointerEnter={hover ? () => set(true) : undefined}
      onPointerLeave={
        hover
          ? () => {
              if (!world.current.drag) set(false);
            }
          : undefined
      }
      onKeyDown={onFolderKeyDown}
      style={rootStyle}
    >
      <div ref={anchorRef} className="folder-float__items">
        {list.map((item, i) => {
          const p = pos[i];
          if (!p) return null;
          return (
            <button
              key={`${item.value}-${i}`}
              ref={(el) => {
                pillRefs.current[i] = el;
              }}
              type="button"
              className="folder-float__item"
              tabIndex={open ? 0 : -1}
              aria-hidden={!open}
              data-pop={popped === i ? "" : undefined}
              style={
                {
                  "--i": i,
                  "--x": `${p.x.toFixed(1)}px`,
                  "--y": `${p.y.toFixed(1)}px`,
                  "--r": `${p.r.toFixed(2)}deg`,
                } as CSSProperties
              }
              onPointerDown={(e) => down(e, i)}
              onPointerMove={(e) => move(e, i)}
              onPointerUp={(e) => up(e, i, item)}
              onPointerCancel={(e) => up(e, i, item)}
              onClick={(e) => {
                if (!world.current.live || e.detail === 0) pick(item, i);
              }}
            >
              <span className="folder-float__drift">{item.label}</span>
            </button>
          );
        })}
      </div>
      <div className="folder-float__folder">
        <span className="folder-float__back" aria-hidden="true" />
        <span className="folder-float__paper" aria-hidden="true" />
        <span className="folder-float__front" aria-hidden="true">
          <span className="folder-float__label">{label}</span>
          <span className="folder-float__sub">{sub}</span>
        </span>
        <button
          type="button"
          className="folder-float__trigger"
          aria-expanded={open}
          aria-label={`${label}, ${sub}`}
          onClick={() => set(!open)}
        />
      </div>
    </div>
  );
}
