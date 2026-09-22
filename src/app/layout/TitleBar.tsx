import { Minus, Square, X } from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { AuroraLogo } from "@/components/aurora-logo";
import { WindowButton } from "./WindowButton";

export function TitleBar() {
  return (
    <header
      data-tauri-drag-region
      className="flex h-9 shrink-0 items-center justify-between border-b border-border bg-surface pl-3 select-none"
    >
      <div data-tauri-drag-region className="flex items-center gap-2 text-foreground">
        <AuroraLogo size={15} />
        <span className="text-sm font-semibold">Aurora Launcher</span>
      </div>

      <div className="flex items-center pr-1">
        <WindowButton label="Minimizar" onClick={() => void getCurrentWindow().minimize()}>
          <Minus size={14} />
        </WindowButton>
        <WindowButton label="Maximizar" onClick={() => void getCurrentWindow().toggleMaximize()}>
          <Square size={11} />
        </WindowButton>
        <WindowButton label="Fechar" danger onClick={() => void getCurrentWindow().close()}>
          <X size={14} />
        </WindowButton>
      </div>
    </header>
  );
}
