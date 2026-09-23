import Plus from "lucide-react/icons/plus.mjs";
import * as React from "react";
import { FAQS } from "../data/content";

export const FaqList = React.memo(function FaqList(): JSX.Element {
  const [open, setOpen] = React.useState<number | null>(0);
  return (
    <div className="faq">
      {FAQS.map(([q, a], i) => {
        const isOpen = open === i;
        return (
          <div key={q} className={`faq-item reveal${isOpen ? " open" : ""}`}>
            <button
              type="button"
              className="faq-q"
              onClick={() => setOpen((o) => (o === i ? null : i))}
              aria-expanded={isOpen}
              aria-controls={`faq-a-${i}`}
            >
              <span>{q}</span>
              <span className={`faq-icon${isOpen ? " open" : ""}`} aria-hidden>
                <Plus size={17} />
              </span>
            </button>
            <section className="faq-a-wrap" id={`faq-a-${i}`} aria-label={q}>
              <p className="muted faq-a">{a}</p>
            </section>
          </div>
        );
      })}
    </div>
  );
});
