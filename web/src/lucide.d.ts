declare module "lucide-react/icons/*" {
  import type {
    ForwardRefExoticComponent,
    RefAttributes,
    SVGProps,
  } from "react";
  import type { LucideProps } from "lucide-react";
  const Icon: ForwardRefExoticComponent<
    Omit<LucideProps, "ref"> &
      SVGProps<SVGSVGElement> &
      RefAttributes<SVGSVGElement>
  >;
  export default Icon;
}
