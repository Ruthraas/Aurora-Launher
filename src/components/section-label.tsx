import type { ReactNode } from "react";

interface SectionLabelProps {
  children: ReactNode;
  action?: ReactNode;
}

export function SectionLabel({ children, action }: SectionLabelProps) {
  return (
    <div className="flex items-center justify-between">
      <p className="text-xs font-medium tracking-wide text-muted-foreground uppercase">{children}</p>
      {action}
    </div>
  );
}
