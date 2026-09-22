import type { ReactNode } from "react";
import { Link } from "react-router-dom";
import { ChevronLeft } from "lucide-react";
import type { Instance } from "@/lib/tauri/commands/instances";
import { loaderLabel } from "./loader-label";

/** Cabeçalho da tela de instância no estilo do app oficial do
 *  Modrinth: ícone grande, nome, badges de loader/versão/RAM soltos
 *  embaixo (em vez do "subtitle" cru de uma linha só que tinha
 *  antes). */
export function InstanceHeader({ instance }: { instance: Instance }) {
  return (
    <div className="flex items-center gap-3">
      <Link to="/instances" className="shrink-0 text-muted-foreground hover:text-foreground">
        <ChevronLeft size={18} />
      </Link>

      <div className="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl bg-surface-2 text-lg font-semibold text-foreground">
        {instance.name.charAt(0).toUpperCase()}
      </div>

      <div className="min-w-0">
        <p className="truncate text-base font-semibold text-foreground">{instance.name}</p>
        <div className="mt-0.5 flex flex-wrap items-center gap-1.5">
          <Badge>{instance.mcVersion}</Badge>
          <Badge>{loaderLabel(instance.loader)}</Badge>
          <Badge>{Math.round(instance.ramMaxMb / 1024)} GB</Badge>
        </div>
      </div>
    </div>
  );
}

function Badge({ children }: { children: ReactNode }) {
  return (
    <span className="rounded-full border border-border bg-surface-2 px-2 py-0.5 text-[11px] font-medium text-muted-foreground">
      {children}
    </span>
  );
}
