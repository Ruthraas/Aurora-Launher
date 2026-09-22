import { useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Check, ChevronDown, Loader2 } from "lucide-react";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { ProgressBar } from "@/components/progress-bar";
import { cn } from "@/lib/utils";
import { getModCompatibility, installMod } from "@/lib/tauri/commands/mods";
import { useInstances } from "@/features/instances/use-instances";
import { CreateInstanceDialog } from "@/features/instances/CreateInstanceDialog";
import { useJobEvents } from "./use-job-events";
import { useJobProgressStore } from "./job-progress-store";
import type { DiscoverResult } from "./ResultRow";

export function AddModDialog({ result, onClose }: { result: DiscoverResult; onClose: () => void }) {
  useJobEvents();
  const queryClient = useQueryClient();
  const { data: instances } = useInstances();
  const project = {
    source: result.source,
    projectId: result.projectId,
    contentType: "mod" as const,
    title: result.title,
    iconUrl: result.iconUrl ?? undefined,
  };

  const { data: entries, isLoading, isError, refetch } = useQuery({
    queryKey: ["mod-compatibility", result.source, result.projectId],
    queryFn: () => getModCompatibility(project),
  });

  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [showIncompatible, setShowIncompatible] = useState(false);
  const [justInstalled, setJustInstalled] = useState(false);
  const [jobId, setJobId] = useState<string | null>(null);
  const job = useJobProgressStore((state) => (jobId ? state.jobs[jobId] : undefined));

  const compatible = entries?.filter((e) => e.compat.status === "compatible") ?? [];
  const alreadyInstalled = entries?.filter((e) => e.compat.status === "alreadyInstalled") ?? [];
  const incompatible = entries?.filter((e) => e.compat.status === "incompatible") ?? [];

  // se só tem uma compatível, já vem marcada (regra do guia)
  useEffect(() => {
    if (compatible.length === 1) setSelected(new Set([compatible[0].instanceId]));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [entries]);

  const instanceName = (id: string) => instances?.find((i) => i.id === id)?.name ?? id;

  function toggle(id: string) {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  const install = useMutation({
    mutationFn: () => installMod(project, Array.from(selected)),
    // `onSuccess` aqui só significa "o job foi enfileirado" — o Tauri
    // devolve o job id na hora, o download em si continua rodando em
    // background. Bug real já corrigido: antes disparava "Instalado."
    // e invalidava a lista de compatibilidade nesse ponto, antes do
    // download de verdade terminar — a UI mentia enquanto o mod ainda
    // tava baixando. Agora só declara sucesso quando o evento
    // `job://progress` confirma `phase === "finished"` (efeito abaixo).
    onSuccess: (id) => setJobId(id),
  });

  const installing = job && job.phase !== "finished" && job.phase !== "failed" && job.phase !== "cancelled";

  const handledJobId = useRef<string | null>(null);
  useEffect(() => {
    if (job?.phase !== "finished" || !jobId || handledJobId.current === jobId) return;
    handledJobId.current = jobId;
    setJustInstalled(true);
    setSelected(new Set());
    void queryClient.invalidateQueries({ queryKey: ["mod-compatibility", result.source, result.projectId] });
    const timer = setTimeout(() => setJustInstalled(false), 2500);
    return () => clearTimeout(timer);
  }, [job?.phase, jobId, queryClient, result.source, result.projectId]);

  return (
    <Dialog open onOpenChange={(next) => !next && onClose()}>
      <DialogContent className="sm:max-w-lg" aria-live="polite">
        <DialogHeader>
          <DialogTitle>Adicionar {result.title} a uma instância</DialogTitle>
        </DialogHeader>

        <p className="text-sm text-muted-foreground">
          Escolha em quais instâncias instalar. Só as que suportam este mod podem ser selecionadas.
        </p>

        {isLoading ? (
          <div className="flex flex-col gap-2">
            {Array.from({ length: 3 }).map((_, i) => (
              <Skeleton key={i} className="h-12 rounded-lg" />
            ))}
          </div>
        ) : isError ? (
          <div className="flex flex-col items-center gap-2 py-4">
            <p className="text-sm text-destructive">Não deu pra conferir a compatibilidade agora.</p>
            <Button variant="outline" size="sm" onClick={() => void refetch()}>
              Tentar de novo
            </Button>
          </div>
        ) : !instances || instances.length === 0 ? (
          <div className="flex flex-col items-center gap-3 py-6 text-center">
            <p className="text-sm text-muted-foreground">Você ainda não tem nenhuma instância.</p>
            <CreateInstanceDialog />
          </div>
        ) : compatible.length === 0 ? (
          <div className="flex flex-col items-center gap-3 py-6 text-center">
            <p className="text-sm text-muted-foreground">Nenhuma instância sua é compatível com esse mod agora.</p>
            <CreateInstanceDialog />
          </div>
        ) : (
          <div className="scroll-thin max-h-80 overflow-y-auto">
            <p className="mb-1.5 text-xs font-medium tracking-wide text-muted-foreground uppercase">Compatíveis</p>
            <div className="flex flex-col gap-1.5">
              {compatible.map((entry) => {
                const isSelected = selected.has(entry.instanceId);
                const modRef = entry.compat.status === "compatible" ? entry.compat.modRef : null;
                const deps = entry.compat.status === "compatible" ? entry.compat.depsToInstall : [];
                return (
                  <label
                    key={entry.instanceId}
                    className={cn(
                      "flex cursor-pointer items-center justify-between gap-3 rounded-lg border px-3 py-2.5 transition-colors",
                      isSelected ? "border-primary/40 bg-primary/5" : "border-border hover:bg-surface-2",
                    )}
                  >
                    <div className="flex items-center gap-3">
                      <input
                        type="checkbox"
                        className="h-4 w-4 accent-primary"
                        checked={isSelected}
                        onChange={() => toggle(entry.instanceId)}
                      />
                      <div>
                        <p className="text-sm font-medium text-foreground">{instanceName(entry.instanceId)}</p>
                        {deps.length > 0 && (
                          <p className="text-xs text-muted-foreground">
                            Também será instalado: {deps.map((d) => d.fileName).join(", ")}
                          </p>
                        )}
                      </div>
                    </div>
                    {modRef && <span className="shrink-0 text-xs text-muted-foreground">v{modRef.versionNumber}</span>}
                  </label>
                );
              })}
            </div>

            {alreadyInstalled.length > 0 && (
              <>
                <p className="mt-4 mb-1.5 text-xs font-medium tracking-wide text-muted-foreground uppercase">Já instalado</p>
                <div className="flex flex-col gap-1.5">
                  {alreadyInstalled.map((entry) => (
                    <div
                      key={entry.instanceId}
                      aria-disabled="true"
                      className="flex items-center justify-between gap-3 rounded-lg border border-border px-3 py-2.5 opacity-45"
                    >
                      <div className="flex items-center gap-3">
                        <Check size={14} className="text-muted-foreground" />
                        <p className="text-sm font-medium text-foreground">{instanceName(entry.instanceId)}</p>
                      </div>
                      <span className="shrink-0 text-xs text-muted-foreground">
                        Já instalado
                        {entry.compat.status === "alreadyInstalled" && ` · v${entry.compat.installedVersion}`}
                      </span>
                    </div>
                  ))}
                </div>
              </>
            )}

            {incompatible.length > 0 && (
              <div className="mt-4">
                <button
                  type="button"
                  onClick={() => setShowIncompatible((v) => !v)}
                  className="flex cursor-pointer items-center gap-1 text-xs font-medium tracking-wide text-muted-foreground uppercase hover:text-foreground"
                >
                  <ChevronDown size={12} className={cn("transition-transform", showIncompatible && "rotate-180")} />
                  Incompatíveis ({incompatible.length})
                </button>
                {showIncompatible && (
                  <div className="mt-1.5 flex flex-col gap-1.5">
                    {incompatible.map((entry) => (
                      <div
                        key={entry.instanceId}
                        aria-disabled="true"
                        className="flex items-center justify-between gap-3 rounded-lg border border-border px-3 py-2.5 opacity-45"
                      >
                        <p className="text-sm font-medium text-foreground">{instanceName(entry.instanceId)}</p>
                        <span className="shrink-0 text-xs text-muted-foreground">
                          {entry.compat.status === "incompatible" && entry.compat.reason}
                        </span>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}
          </div>
        )}

        {installing && (
          <div className="flex flex-col gap-1.5">
            <div className="flex items-center justify-between text-xs text-muted-foreground">
              <span>Instalando…</span>
              {job.total > 0 && (
                <span>
                  {job.done}/{job.total}
                </span>
              )}
            </div>
            <ProgressBar value={job.total > 0 ? (job.done / job.total) * 100 : 0} />
          </div>
        )}
        {job?.phase === "failed" && <p className="text-xs text-destructive">{job.error ?? "Falha ao instalar."}</p>}
        {install.isError && <p className="text-xs text-destructive">{install.error.message}</p>}
        {justInstalled && <p className="text-xs text-primary">Instalado.</p>}

        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            Cancelar
          </Button>
          <Button disabled={selected.size === 0 || install.isPending || Boolean(installing)} onClick={() => install.mutate()}>
            {install.isPending || installing ? <Loader2 size={14} className="mr-1.5 animate-spin" /> : null}
            Adicionar a {selected.size} instância(s)
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
