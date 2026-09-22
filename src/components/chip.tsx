import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

interface ChipProps {
  active?: boolean;
  onClick?: () => void;
  children: ReactNode;
}

export function Chip({ active, onClick, children }: ChipProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "shrink-0 cursor-pointer rounded-full border px-3 py-1 text-xs font-medium whitespace-nowrap transition-colors",
        active
          ? "border-transparent bg-foreground text-background"
          : "border-border text-muted-foreground hover:text-foreground",
      )}
    >
      {children}
    </button>
  );
}
