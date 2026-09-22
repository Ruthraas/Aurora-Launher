import { useState } from "react";
import { Link, useParams } from "react-router-dom";
import { ChevronLeft } from "lucide-react";
import { PageHeader } from "@/components/page-header";
import { PlayButton } from "@/components/play-button";
import { cn } from "@/lib/utils";
import { useInstances } from "./use-instances";
import { useInstallEvents } from "./use-install-events";
import { useLaunchInstance } from "./use-instance-mutations";
import { loaderLabel } from "./loader-label";
import { OverviewTab } from "./OverviewTab";
import { ModsTab } from "./ModsTab";

type Tab = "overview" | "mods";

export function InstanceDetailPage() {
  useInstallEvents();
  const { id } = useParams<{ id: string }>();
  const { data: instances, isLoading } = useInstances();
  const launchInstance = useLaunchInstance();
  const [tab, setTab] = useState<Tab>("overview");

  const instance = instances?.find((i) => i.id === id);

  if (isLoading) return null;
  if (!instance) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3">
        <p className="text-sm text-muted-foreground">Instância não encontrada.</p>
        <Link to="/instances" className="text-sm text-primary hover:underline">
          Voltar pra Instâncias
        </Link>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <PageHeader
        title={
          <span className="flex items-center gap-2">
            <Link to="/instances" className="text-muted-foreground hover:text-foreground">
              <ChevronLeft size={18} />
            </Link>
            {instance.name}
          </span>
        }
        subtitle={`${instance.mcVersion} · ${loaderLabel(instance.loader)}`}
        actions={
          instance.status === "ready" || instance.status === "incomplete" ? (
            <PlayButton disabled={launchInstance.isPending} onClick={() => launchInstance.mutate(instance.id)}>
              {launchInstance.isPending ? "Abrindo…" : "Jogar"}
            </PlayButton>
          ) : undefined
        }
      />

      <div className="flex gap-5 border-b border-border px-6">
        {(["overview", "mods"] as Tab[]).map((value) => (
          <button
            key={value}
            type="button"
            onClick={() => setTab(value)}
            className={cn(
              "cursor-pointer border-b-2 py-2.5 text-sm font-medium transition-colors",
              tab === value ? "border-primary text-foreground" : "border-transparent text-muted-foreground hover:text-foreground",
            )}
          >
            {value === "overview" ? "Visão geral" : "Mods"}
          </button>
        ))}
      </div>

      <div className="scroll-thin flex-1 overflow-y-auto p-6">
        {tab === "overview" ? <OverviewTab instance={instance} /> : <ModsTab instance={instance} />}
      </div>
    </div>
  );
}
