import type { ReactNode } from "react";

interface PageHeaderProps {
  title: ReactNode;
  subtitle?: string;
  actions?: ReactNode;
}

export function PageHeader({ title, subtitle, actions }: PageHeaderProps) {
  return (
    <div className="flex items-center justify-between gap-4 border-b border-border px-6 py-4">
      <div>
        <h1 className="text-base font-semibold text-foreground">{title}</h1>
        {subtitle && <p className="mt-0.5 text-sm text-muted-foreground">{subtitle}</p>}
      </div>
      {actions && <div className="flex shrink-0 items-center gap-2">{actions}</div>}
    </div>
  );
}
