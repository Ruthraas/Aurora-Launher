import { useState } from "react";
import { Link } from "react-router-dom";
import { Boxes } from "lucide-react";
import { useTranslation } from "react-i18next";
import { PageHeader } from "@/components/page-header";
import { EmptyState } from "@/components/empty-state";
import { PlayButton } from "@/components/play-button";
import { ProgressBar } from "@/components/progress-bar";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";
import { useInstances } from "@/features/instances/use-instances";
import { useInstallEvents } from "@/features/instances/use-install-events";
import { useInstallProgressStore } from "@/features/instances/install-progress-store";
import { useLaunchInstance } from "@/features/instances/use-instance-mutations";
import { installStageLabels } from "@/features/instances/install-stage-labels";

const MAX_PREVIEW = 4;

export function HomePage() {
  useInstallEvents();
  const { t } = useTranslation();
  const { data: instances, isLoading } = useInstances();
  const launchInstance = useLaunchInstance();
  const progress = useInstallProgressStore((state) => state.progress);

  const [selectedId, setSelectedId] = useState<string | null>(null);

  const mostRecentlyPlayed = instances?.filter((i) => i.lastPlayed).sort((a, b) => (b.lastPlayed! > a.lastPlayed! ? 1 : -1))[0];
  const hero = (selectedId && instances?.find((i) => i.id === selectedId)) || mostRecentlyPlayed;
  const preview = instances?.slice(0, MAX_PREVIEW) ?? [];
  const activeInstall = Object.values(progress)[0];
  const installingInstance = activeInstall && instances?.find((i) => i.id === activeInstall.instanceId);

  return (
    <div className="flex h-full flex-col">
      <PageHeader title={t("nav.home")} />

      <div className="scroll-thin flex-1 overflow-y-auto p-6">
        {isLoading ? (
          <Skeleton className="h-28 w-full rounded-lg" />
        ) : !instances || instances.length === 0 ? (
          <EmptyState icon={Boxes} title={t("instances.emptyTitle")} description={t("instances.emptyDescription")} />
        ) : (
          <div className="flex flex-col gap-6">
            {hero && (
              <div className="flex items-center justify-between gap-4 rounded-xl border border-border bg-card p-5">
                <div>
                  <p className="text-xs font-medium tracking-wide text-muted-foreground uppercase">
                    {selectedId ? "Selecionada" : "Continuar jogando"}
                  </p>
                  <p className="mt-1 text-xl font-bold text-foreground">{hero.name}</p>
                  <p className="mt-0.5 text-xs text-muted-foreground">
                    {hero.mcVersion} · {hero.loader.type === "vanilla" ? "Vanilla" : hero.loader.type}
                  </p>
                </div>
                <PlayButton disabled={launchInstance.isPending} onClick={() => launchInstance.mutate(hero.id)} className="h-11 px-5">
                  {launchInstance.isPending ? "Abrindo…" : "Jogar"}
                </PlayButton>
              </div>
            )}

            <div>
              <div className="mb-2.5 flex items-center justify-between">
                <p className="text-xs font-medium tracking-wide text-muted-foreground uppercase">Instâncias</p>
                <Link to="/instances" className="text-xs text-primary hover:underline">
                  Ver todas
                </Link>
              </div>
              <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4">
                {preview.map((instance) => (
                  <button
                    key={instance.id}
                    type="button"
                    onClick={() => setSelectedId(instance.id)}
                    className={cn(
                      "cursor-pointer rounded-lg border p-3.5 text-left transition-colors",
                      instance.id === hero?.id
                        ? "border-primary/40 bg-primary/5"
                        : "border-border bg-card hover:border-primary/40",
                    )}
                  >
                    <p className="truncate text-sm font-semibold text-foreground">{instance.name}</p>
                    <p className="mt-0.5 truncate text-xs text-muted-foreground">
                      {instance.mcVersion} · {instance.loader.type === "vanilla" ? "Vanilla" : instance.loader.type}
                    </p>
                  </button>
                ))}
              </div>
            </div>

            {installingInstance && activeInstall && (
              <div className="rounded-xl border border-border bg-card p-4">
                <div className="mb-1.5 flex items-center justify-between text-sm">
                  <span className="text-foreground">
                    {installStageLabels[activeInstall.stage]} — {installingInstance.name}
                  </span>
                  {activeInstall.total > 0 && (
                    <span className="text-muted-foreground">
                      {Math.round((activeInstall.completed / activeInstall.total) * 100)}%
                    </span>
                  )}
                </div>
                <ProgressBar value={activeInstall.total > 0 ? (activeInstall.completed / activeInstall.total) * 100 : 0} />
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
