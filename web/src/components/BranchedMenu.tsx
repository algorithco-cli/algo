import {
  CursorPointer01Icon,
  Download04Icon,
  Layers01Icon,
  Notification03Icon,
  PaintBoardIcon,
  Rocket01Icon,
  Settings02Icon,
  TextFontIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { isValidElement, useLayoutEffect, useRef, useState } from "react";
import "./BranchedMenu.css";

const DEFAULT_ITEMS = [
  {
    label: "Getting started",
    children: [
      { value: "install", label: "Installation", icon: Download04Icon },
      { value: "quick", label: "Quick start", icon: Rocket01Icon },
      { value: "config", label: "Configuration", icon: Settings02Icon },
      { value: "theming", label: "Theming", icon: PaintBoardIcon },
    ],
  },
  {
    label: "Components",
    children: [
      { value: "buttons", label: "Buttons", icon: CursorPointer01Icon },
      { value: "typography", label: "Typography", icon: TextFontIcon },
      { value: "overlays", label: "Overlays", icon: Layers01Icon },
      { value: "toasts", label: "Toasts", icon: Notification03Icon },
    ],
  },
];

const PAD = 6;
const MARK = 16;

const renderIcon = (icon: unknown): JSX.Element | null => {
  if (!icon) return null;
  if (isValidElement(icon)) return icon as JSX.Element;
  return <HugeiconsIcon icon={icon as never} size={16} strokeWidth={1.8} />;
};

const toSet = (
  open: number | number[] | Set<number> | undefined,
): Set<number> =>
  new Set(
    Array.isArray(open)
      ? open
      : typeof open === "number" && open >= 0
        ? [open]
        : open instanceof Set
          ? [...open]
          : [],
  );

export type BranchedMenuLeaf = {
  value: string;
  label: string;
  href?: string;
  icon?: unknown;
};

export type BranchedMenuSection = {
  label: string;
  value?: string;
  children?: BranchedMenuLeaf[];
};

export default function BranchedMenu({
  items = DEFAULT_ITEMS as unknown as BranchedMenuSection[],
  defaultOpen = 0,
  defaultActive = "",
  active: controlledActive,
  open: controlledOpen,
  onSelect,
  onToggle,
  color = "var(--ag-text)",
  accentColor = "var(--ag-brand)",
  lineColor = "var(--ag-border)",
  width = 240,
  rowHeight = 36,
  indent = 40,
  trunk = 14,
  radius = 10,
  lineWidth = 1.5,
  fontSize = 14,
  drawDuration = 400,
  foldDuration = 300,
  className = "",
  ariaLabel = "Branched menu",
  staticHeads = false,
}: {
  items?: BranchedMenuSection[];
  defaultOpen?: number | number[];
  defaultActive?: string;
  active?: string;
  open?: number[] | Set<number>;
  onSelect?: (
    value: string,
    item: BranchedMenuLeaf | BranchedMenuSection,
  ) => void;
  onToggle?: (index: number, isOpen: boolean) => void;
  color?: string;
  accentColor?: string;
  lineColor?: string;
  width?: number;
  rowHeight?: number;
  indent?: number;
  trunk?: number;
  radius?: number;
  lineWidth?: number;
  fontSize?: number;
  drawDuration?: number;
  foldDuration?: number;
  className?: string;
  ariaLabel?: string;
  staticHeads?: boolean;
}): JSX.Element {
  const [innerOpen, setInnerOpen] = useState<Set<number>>(() =>
    toSet(defaultOpen),
  );
  const [innerActive, setInnerActive] = useState<string>(() => {
    if (defaultActive) return defaultActive;
    const first = (items as BranchedMenuSection[]).find(
      (it, i) => it.children && toSet(defaultOpen).has(i),
    );
    return (first?.children?.[0]?.value as string) ?? "";
  });

  const open = controlledOpen !== undefined ? toSet(controlledOpen) : innerOpen;
  const active =
    controlledActive !== undefined ? controlledActive : innerActive;

  const navRef = useRef<HTMLElement | null>(null);
  const heads = useRef<Array<HTMLElement | null>>([]);
  const markerRef = useRef<HTMLSpanElement | null>(null);
  const latest = useRef<{
    onSelect?: typeof onSelect;
    onToggle?: typeof onToggle;
  }>({});
  latest.current = { onSelect, onToggle };

  const activeSection = (items as BranchedMenuSection[]).findIndex((it) =>
    it.children?.some((kid) => kid.value === active),
  );
  const markerShown = activeSection >= 0 && open.has(activeSection);

  useLayoutEffect(() => {
    const place = (glide: boolean): void => {
      const m = markerRef.current;
      const el = heads.current[activeSection];
      if (!m) return;
      const on = Boolean(markerShown && el);
      if (!glide) m.style.transition = "none";
      if (on && el)
        m.style.top = `${el.offsetTop + (el.offsetHeight - MARK) / 2}px`;
      if (on) m.setAttribute("data-on", "");
      else m.removeAttribute("data-on");
      if (!glide) {
        void m.offsetHeight;
        m.style.transition = "";
      }
    };
    place(true);
    let first = true;
    const ro = new ResizeObserver(() => {
      if (first) {
        first = false;
        return;
      }
      place(false);
    });
    if (navRef.current) ro.observe(navRef.current);
    return () => ro.disconnect();
  }, [activeSection, markerShown, items, fontSize, rowHeight]);

  const select = (
    value: string,
    item: BranchedMenuLeaf | BranchedMenuSection,
  ): void => {
    if (controlledActive === undefined) setInnerActive(value);
    latest.current.onSelect?.(value, item);
  };
  const toggle = (i: number): void => {
    if (controlledOpen === undefined) {
      setInnerOpen((prev) => {
        const next = new Set(prev);
        const isOpen = !next.has(i);
        if (isOpen) next.add(i);
        else next.delete(i);
        latest.current.onToggle?.(i, isOpen);
        return next;
      });
    } else {
      latest.current.onToggle?.(i, !open.has(i));
    }
  };

  const r = Math.min(radius, rowHeight / 2 - 2);
  const endX = indent - 8;
  const rowY = (k: number): number => PAD + k * rowHeight + rowHeight / 2;
  const branch = (k: number): string =>
    `M ${trunk} ${rowY(k) - r} A ${r} ${r} 0 0 0 ${trunk + r} ${rowY(k)} H ${endX}`;
  const reach = (k: number): string =>
    `M ${trunk} 0 V ${rowY(k) - r} A ${r} ${r} 0 0 0 ${trunk + r} ${rowY(k)} H ${endX}`;
  const length = (k: number): number =>
    rowY(k) - r + (Math.PI * r) / 2 + (endX - trunk - r);

  return (
    <nav
      ref={navRef as never}
      className={`branched-menu${className ? ` ${className}` : ""}`}
      aria-label={ariaLabel}
      style={
        {
          "--bm-w": `${width}px`,
          "--bm-ink": color,
          "--bm-accent": accentColor,
          "--bm-line": lineColor,
          "--bm-font": `${fontSize}px`,
          "--bm-row": `${rowHeight}px`,
          "--bm-indent": `${indent}px`,
          "--bm-line-w": lineWidth,
          "--bm-draw": `${drawDuration}ms`,
          "--bm-fold": `${foldDuration}ms`,
        } as never
      }
    >
      <span
        ref={markerRef}
        className="branched-menu__marker"
        aria-hidden="true"
      />
      {(items as BranchedMenuSection[]).map((item, i) => {
        const kids = item.children;
        const isOpen = kids ? open.has(i) : false;
        const leafValue = item.value ?? item.label;
        const leafActive = !kids && leafValue === active;
        const bodyH = kids ? PAD * 2 + kids.length * rowHeight : 0;
        return (
          <div
            key={item.value ?? item.label}
            className="branched-menu__section"
            data-open={isOpen ? "" : undefined}
          >
            {staticHeads ? (
              <span
                ref={(el) => {
                  heads.current[i] = el;
                }}
                className="branched-menu__head branched-menu__head--static"
              >
                {item.label}
              </span>
            ) : (
              <button
                ref={(el) => {
                  heads.current[i] = el;
                }}
                type="button"
                className="branched-menu__head"
                aria-expanded={kids ? isOpen : undefined}
                aria-current={leafActive ? "true" : undefined}
                data-active={leafActive ? "" : undefined}
                onClick={() => (kids ? toggle(i) : select(leafValue, item))}
              >
                {item.label}
              </button>
            )}
            {kids ? (
              <div className="branched-menu__body">
                <div className="branched-menu__fold">
                  <div
                    className="branched-menu__tree"
                    style={{ height: bodyH }}
                  >
                    <svg
                      className="branched-menu__lines"
                      width={indent}
                      height={bodyH}
                      aria-hidden="true"
                    >
                      <path
                        className="branched-menu__base"
                        d={`M ${trunk} 0 V ${rowY(kids.length - 1) - r}`}
                      />
                      {kids.map((kid, k) => (
                        <path
                          key={kid.value}
                          className="branched-menu__base"
                          d={branch(k)}
                        />
                      ))}
                      {kids.map((kid, k) => (
                        <path
                          key={kid.value}
                          className="branched-menu__reach"
                          d={reach(k)}
                          style={{
                            strokeDasharray: length(k),
                            strokeDashoffset:
                              kid.value === active ? 0 : length(k),
                          }}
                        />
                      ))}
                    </svg>
                    {kids.map((kid) =>
                      kid.href ? (
                        <a
                          key={kid.value}
                          href={kid.href}
                          className="branched-menu__item"
                          aria-current={
                            kid.value === active ? "true" : undefined
                          }
                          data-active={kid.value === active ? "" : undefined}
                          tabIndex={isOpen ? 0 : -1}
                          onClick={() => select(kid.value, kid)}
                        >
                          {kid.icon ? (
                            <span
                              className="branched-menu__icon"
                              aria-hidden="true"
                            >
                              {renderIcon(kid.icon)}
                            </span>
                          ) : null}
                          <span className="branched-menu__label">
                            {kid.label}
                          </span>
                        </a>
                      ) : (
                        <button
                          key={kid.value}
                          type="button"
                          className="branched-menu__item"
                          aria-current={
                            kid.value === active ? "true" : undefined
                          }
                          data-active={kid.value === active ? "" : undefined}
                          tabIndex={isOpen ? 0 : -1}
                          onClick={() => select(kid.value, kid)}
                        >
                          {kid.icon ? (
                            <span
                              className="branched-menu__icon"
                              aria-hidden="true"
                            >
                              {renderIcon(kid.icon)}
                            </span>
                          ) : null}
                          <span className="branched-menu__label">
                            {kid.label}
                          </span>
                        </button>
                      ),
                    )}
                  </div>
                </div>
              </div>
            ) : null}
          </div>
        );
      })}
    </nav>
  );
}
