import { Breadcrumbs } from "../components/Breadcrumbs";
import { FaqList } from "../components/FaqList";
import { Section } from "../components/Section";
import { Seo } from "../components/Seo";
import { FAQS } from "../data/content";
import { useRevealOnMount } from "../hooks/useRevealOnMount";

export default function Faq(): JSX.Element {
  useRevealOnMount();
  return (
    <>
      <Seo path="/faq" />
      <div className="wrap" style={{ paddingTop: 24 }}>
        <Breadcrumbs trail={[{ label: "Home", to: "/" }, { label: "FAQ" }]} />
      </div>
      <Section
        id="faq"
        kicker="FAQ"
        title="Questions,"
        accent="answered"
        lede="Safety, privacy, speed, rollout — in one place."
      >
        <FaqList />
      </Section>
      <script type="application/ld+json">
        {JSON.stringify({
          "@context": "https://schema.org",
          "@type": "FAQPage",
          mainEntity: FAQS.map(([q, a]) => ({
            "@type": "Question",
            name: q,
            acceptedAnswer: { "@type": "Answer", text: a },
          })),
        })}
      </script>
    </>
  );
}
