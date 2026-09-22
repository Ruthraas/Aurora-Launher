import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

interface WindowButtonProps {
  onClick: () => void;
  label: string;
  danger?: boolean;
  children: ReactNode;
}

export function WindowButton({ onClick, label, danger, children }: WindowButtonProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-label={label}
      className={cn(
        "flex h-7 w-9 items-center justify-center rounded text-white/70 transition-colors",
        danger ? "hover:bg-destructive hover:text-white" : "hover:bg-white/10 hover:text-white",
      )}
    >
      {children}
    </button>
  );
}
