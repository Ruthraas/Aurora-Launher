import { Link } from "react-router-dom";
import { RotateCw, Settings, Trash2, TriangleAlert } from "lucide-react";
import { Button } from "@/components/ui/button";
import { PlayButton } from "@/components/play-button";
import { ProgressBar } from "@/components/progress-bar";
import type { Instance } from "@/lib/tauri/commands/instances";
import { useInstallProgressStore } from "./install-progress-store";
import { useDeleteInstance, useLaunchInstance, useRetryInstanceInstall } from "./use-instance-mutations";
import { installStageLabels } from "./install-stage-labels";
import { loaderLabel } from "./loader-label";

export function InstanceCard({ instance }: { instance: Instance }) {
  const progress = useInstallProgressStore((state) => state.progress[instance.id]);
  const deleteInstance = useDeleteInstance();
  const launchInstance = useLaunchInstance();
  const retryInstall = useRetryInstanceInstall();

  return (
    <div className="rounded-lg border border-border bg-card p-4">
      <div className="flex items-start justify-between gap-2">
        <Link to={`/instances/${instance.id}`} className="min-w-0 hover:opacity-80">
          <p className="truncate text-sm font-semibold text-foreground">{instance.name}</p>
          <p className="text-meta text-xs">
            {instance.mcVersion} · {loaderLabel(instance.loader)}
          </p>
        </Link>
        <div className="flex shrink-0 gap-0.5">
          <Button variant="ghost" size="icon-sm" asChild>
            <Link to={`/instances/${instance.id}`}>
              <Settings size={14} />
            </Link>
          </Button>
          <Button
            variant="ghost"
            size="icon-sm"
            disabled={deleteInstance.isPending}
            onClick={() => deleteInstance.mutate(instance.id)}
          >
            <Trash2 size={14} />
          </Button>
        </div>
      </div>

      {instance.status === "installing" && (
        <div className="mt-3 space-y-1.5">
          <div className="flex items-center justify-between text-xs text-muted-foreground">
            <span>{progress ? installStageLabels[progress.stage] : "Preparando…"}</span>
            {progress && progress.total > 0 && (
              <span>
                {progress.completed}/{progress.total}
              </span>
            )}
          </div>
          <ProgressBar value={progress && progress.total > 0 ? (progress.completed / progress.total) * 100 : 0} />
        </div>
      )}

      {(instance.status === "ready" || instance.status === "incomplete") && (
        <div className="mt-3 flex flex-col gap-1.5">
          {instance.status === "incomplete" && (
            <p className="flex items-center gap-1.5 text-xs text-warning">
              <TriangleAlert size={12} />
              {instance.missingManualDownloads.length} mod(s) pra baixar na mão —{" "}
              <Link to={`/instances/${instance.id}`} className="underline">
                ver detalhes
              </Link>
            </p>
          )}
          <div className="flex gap-1.5">
            <PlayButton
              className="flex-1"
              disabled={launchInstance.isPending}
              onClick={() => launchInstance.mutate(instance.id)}
            >
              {launchInstance.isPending ? "Abrindo…" : "Jogar"}
            </PlayButton>
            <Button
              variant="outline"
              size="icon"
              title="Reinstalar (repara arquivos faltando ou corrompidos)"
              disabled={retryInstall.isPending}
              onClick={() => retryInstall.mutate(instance.id)}
            >
              <RotateCw size={14} />
            </Button>
          </div>
        </div>
      )}

      {instance.status === "error" && (
        <div className="mt-3 space-y-2">
          <p className="line-clamp-2 text-xs text-destructive">{instance.errorMessage ?? "Falha na instalação."}</p>
          <Button
            variant="outline"
            size="sm"
            className="w-full gap-1.5"
            disabled={retryInstall.isPending}
            onClick={() => retryInstall.mutate(instance.id)}
          >
            <RotateCw size={13} />
            {retryInstall.isPending ? "Tentando de novo…" : "Tentar de novo"}
          </Button>
        </div>
      )}

      {launchInstance.isError && <p className="mt-2 text-xs text-destructive">{launchInstance.error.message}</p>}
    </div>
  );
}
