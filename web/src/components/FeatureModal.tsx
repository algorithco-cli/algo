import {
  ArrowLeft01Icon,
  ArrowRight01Icon,
  Cancel01Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useEffect, useRef } from "react";
import type { KeyboardEvent as ReactKeyboardEvent, ReactNode } from "react";
import { createPortal } from "react-dom";
import { Link } from "react-router-dom";
import "./FeatureModal.css";

export interface FeatureModalLink {
  to: string;
  label: string;
}

interface FeatureModalProps {
  kicker: string;
  icon: ReactNode;
  title: string;
  body: ReactNode;
  link?: FeatureModalLink;
  steps: readonly string[];
  index: number;
  onStep: (index: number) => void;
  onClose: () => void;
}

const FOCUSABLE =
  'a[href], button:not([disabled]), [tabindex]:not([tabindex="-1"])';

export function FeatureModal({
  kicker,
  icon,
  title,
  body,
  link,
  steps,
  index,
  onStep,
  onClose,
}: FeatureModalProps): JSX.Element {
  const cardRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const card = cardRef.current;
    const prevFocus = document.activeElement as HTMLElement | null;
    card
      ?.querySelector<HTMLButtonElement>("[data-autofocus]")
      ?.focus({ preventScroll: true });
    const onKey = (e: KeyboardEvent): void => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onKey);
    const prev = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    return () => {
      document.removeEventListener("keydown", onKey);
      document.body.style.overflow = prev;
      prevFocus?.focus({ preventScroll: true });
    };
  }, [onClose]);

  const trapTab = (e: ReactKeyboardEvent): void => {
    if (e.key !== "Tab") return;
    const card = cardRef.current;
    if (!card) return;
    const items = [...card.querySelectorAll<HTMLElement>(FOCUSABLE)].filter(
      (el) => el.getAttribute("aria-hidden") !== "true",
    );
    const first = items[0];
    const last = items[items.length - 1];
    if (!first || !last) return;
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first.focus();
    }
  };

  // Portaled to document.body: ancestors with `contain: layout paint`
  // (e.g. .reveal sections) would otherwise hijack position:fixed.
  return createPortal(
    <dialog
      open
      className="feature-modal"
      aria-labelledby="feature-modal-title"
      onKeyDown={trapTab}
    >
      <button
        type="button"
        className="feature-modal__overlay"
        aria-label="Close dialog"
        onClick={onClose}
        tabIndex={-1}
      />
      <div ref={cardRef} className="feature-modal__card">
        <div className="feature-modal__head">
          <span className="feature-modal__icon" aria-hidden="true">
            {icon}
          </span>
          <span className="feature-modal__headtext">
            <span className="feature-modal__kicker">{kicker}</span>
            <span className="feature-modal__step">
              {index + 1} / {steps.length}
            </span>
          </span>
          <button
            type="button"
            className="feature-modal__x"
            aria-label="Close dialog"
            onClick={onClose}
            data-autofocus
          >
            <HugeiconsIcon icon={Cancel01Icon} size={18} strokeWidth={1.8} />
          </button>
        </div>

        <div className="feature-modal__body">
          <h3 id="feature-modal-title">{title}</h3>
          <div className="feature-modal__text">{body}</div>
          <div className="feature-modal__dots">
            {steps.map((key, i) => (
              <button
                key={key}
                type="button"
                className="feature-modal__dot"
                data-active={i === index ? "" : undefined}
                aria-label={`Show pillar ${i + 1} of ${steps.length}`}
                aria-current={i === index ? "true" : undefined}
                onClick={() => onStep(i)}
                tabIndex={i === index ? -1 : 0}
              />
            ))}
          </div>
        </div>

        <div className="feature-modal__foot">
          <span className="feature-modal__stepbtns">
            <button
              type="button"
              className="feature-modal__stepbtn"
              aria-label="Previous pillar"
              disabled={index === 0}
              onClick={() => onStep(index - 1)}
            >
              <HugeiconsIcon
                icon={ArrowLeft01Icon}
                size={17}
                strokeWidth={1.8}
              />
            </button>
            <button
              type="button"
              className="feature-modal__stepbtn"
              aria-label="Next pillar"
              disabled={index === steps.length - 1}
              onClick={() => onStep(index + 1)}
            >
              <HugeiconsIcon
                icon={ArrowRight01Icon}
                size={17}
                strokeWidth={1.8}
              />
            </button>
          </span>
          {link ? (
            <Link
              to={link.to}
              className="btn btn-primary feature-modal__primary"
              onClick={onClose}
            >
              {link.label} →
            </Link>
          ) : (
            <button
              type="button"
              className="btn btn-primary feature-modal__primary"
              onClick={onClose}
            >
              Got it
            </button>
          )}
        </div>
      </div>
    </dialog>,
    document.body,
  );
}
