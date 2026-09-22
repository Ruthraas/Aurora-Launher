import type { ReactNode } from "react";
import { Play } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

interface PlayButtonProps {
  className?: string;
  disabled?: boolean;
  onClick?: () => void;
  children?: ReactNode;
}

/** Botão primário — accent chapado, sem gradiente (o único gradiente
 * permitido no design é o das barras de progresso, ver progress-bar.tsx). */
export function PlayButton({ className, disabled, onClick, children }: PlayButtonProps) {
  return (
    <Button
      onClick={onClick}
      disabled={disabled}
      className={cn("gap-1.5 bg-primary text-primary-foreground hover:bg-primary/90", className)}
    >
      <Play size={15} className="fill-current" />
      {children ?? "Jogar"}
    </Button>
  );
}
