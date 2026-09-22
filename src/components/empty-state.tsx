import type { LucideIcon } from "lucide-react";
import { cn } from "@/lib/utils";

interface EmptyStateProps {
  icon: LucideIcon;
  title: string;
  description?: string;
  className?: string;
}

export function EmptyState({ icon: Icon, title, description, className }: EmptyStateProps) {
  return (
    <div className={cn("flex h-full flex-col items-center justify-center gap-3 p-8 text-center", className)}>
      <div className="flex h-11 w-11 items-center justify-center rounded-lg border border-border bg-card">
        <Icon className="text-muted-foreground" size={20} strokeWidth={1.5} />
      </div>
      <div>
        <p className="text-sm font-medium text-foreground">{title}</p>
        {description && <p className="mt-1 max-w-sm text-sm text-muted-foreground">{description}</p>}
      </div>
    </div>
  );
}
