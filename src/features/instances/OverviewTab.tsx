import { useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Check, FolderOpen, Loader2, Sparkles } from "lucide-react";
import { Slider } from "@/components/ui/slider";
import { ProgressBar } from "@/components/progress-bar";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Button } from "@/components/ui/button";
import type { Instance, JvmFlagPreset } from "@/lib/tauri/commands/instances";
import { openInstanceFolder, updateInstanceSettings } from "@/lib/tauri/commands/instances";
import { getRecommendedOptimizations, installOptimizations } from "@/lib/tauri/commands/optimization";
import { useJobEvents } from "@/features/discover/use-job-events";
import { useJobProgressStore } from "@/features/discover/job-progress-store";
import { jobPhaseLabels } from "@/features/discover/job-phase-labels";
import { instancesQueryKey } from "./use-instances";
import { InstalledContentList } from "./InstalledContentList";

const RAM_MIN_GB = 1;
const RAM_MAX_GB = 16;

const JVM_PRESET_LABEL: Record<JvmFlagPreset, string> = {
  none: "Padrão (nenhuma flag extra)",
  g1gcOptimized: "G1GC otimizado (recomendado)",
};

export function OverviewTab({ instance }: { instance: Instance }) {
  useJobEvents();
  const queryClient = useQueryClient();
  const [ramGb, setRamGb] = useState(Math.round(instance.ramMaxMb / 1024));
  const [preset, setPreset] = useState<JvmFlagPreset>(instance.jvmFlagPreset);
  const [jobId, setJobId] = useState<string | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [showSuccess, setShowSuccess] = useState(false);
  const [dismissed, setDismissed] = useState(false);
  const job = useJobProgressStore((state) => (jobId ? state.jobs[jobId] : undefined));

  useEffect(() => {
    setRamGb(Math.round(instance.ramMaxMb / 1024));
    setPreset(instance.jvmFlagPreset);
  }, [instance.id, instance.ramMaxMb, instance.jvmFlagPreset]);

  const saveSettings = useMutation({
    mutationFn: (next: { ramGb: number; preset: JvmFlagPreset }) =>
      updateInstanceSettings(instance.id, 1024, next.ramGb * 1024, next.preset),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: instancesQueryKey }),
  });

  const { data: optimizations, isLoading: loadingOptimizations } = useQuery({
    queryKey: ["optimizations", instance.id],
    queryFn: () => getRecommendedOptimizations(instance.id),
  });

  useEffect(() => {
    if (!optimizations) return;
    const compatibleNotInstalled = optimizations.filter((o) => o.compat.status === "compatible").map((o) => o.projectId);
    setSelected(new Set(compatibleNotInstalled));
  }, [optimizations]);

  const install = useMutation({
    mutationFn: () => installOptimizations(instance.id, Array.from(selected)),
    onSuccess: (id) => setJobId(id),
  });

  const installing = jobId && job && job.phase !== "finished" && job.phase !== "failed" && job.phase !== "cancelled";

  // Ao terminar: atualiza os dados (o que reflete no `InstalledContentList`
  // e deixaria o card sumir sozinho quando o refetch chegasse), mas
  // não espera por isso pra tirar o card da tela — mostra "Instalado
  // com sucesso" por um instante e já esconde, sem depender do timing
  // de rede pra dar a sensação de "terminou". Precisa ser um efeito
  // (não rodar direto no corpo do componente): fazer isso no render
  // reagendaria a cada re-render enquanto o job ficasse "finished"
  // parado. O `ref` garante que só trata cada job uma vez.
  const handledJobId = useRef<string | null>(null);
  useEffect(() => {
    if (job?.phase !== "finished" || !jobId || handledJobId.current === jobId) return;
    handledJobId.current = jobId;
    void queryClient.invalidateQueries({ queryKey: ["optimizations", instance.id] });
    void queryClient.invalidateQueries({ queryKey: ["instance-mods", instance.id] });
    setShowSuccess(true);
    const timer = setTimeout(() => {
      setShowSuccess(false);
      setDismissed(true);
    }, 1500);
    return () => clearTimeout(timer);
  }, [job?.phase, jobId, instance.id, queryClient]);

  function toggle(projectId: string) {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(projectId)) next.delete(projectId);
      else next.add(projectId);
      return next;
    });
  }

  // some sozinho assim que não sobrar nenhuma recomendação acionável —
  // seja porque já instalou tudo, seja porque nada é compatível. Uma
  // instância "otimizada" não deveria continuar mostrando o cartão de
  // "otimizar" pra sempre. `dismissed` cobre o instante entre "acabou
  // de instalar" e o refetch confirmar isso (não espera a rede pra
  // sumir).
  const hasActionableOptimizations = optimizations?.some((o) => o.compat.status === "compatible") ?? false;
  const showCard = !dismissed && (installing || showSuccess || hasActionableOptimizations);

  return (
    <div className="grid gap-6 lg:grid-cols-3">
      <div className="flex flex-col gap-6 lg:col-span-2">
        {loadingOptimizations ? (
          <div className="flex min-h-[120px] items-center justify-center rounded-xl border border-border bg-card">
            <p className="text-sm text-muted-foreground">Carregando recomendações…</p>
          </div>
        ) : optimizations && showCard ? (
          <div className="flex flex-col rounded-xl border border-border bg-card p-5">
            <p className="flex items-center gap-1.5 text-xs font-medium tracking-wide text-muted-foreground uppercase">
              <Sparkles size={13} />
              Otimizar desempenho
            </p>

            {showSuccess ? (
              <div className="flex flex-1 flex-col items-center justify-center gap-2 py-8">
                <span className="flex h-10 w-10 items-center justify-center rounded-full bg-primary/15 text-primary">
                  <Check size={20} />
                </span>
                <p className="text-sm font-medium text-foreground">Instalado com sucesso</p>
              </div>
            ) : installing ? (
              <div className="flex flex-1 flex-col justify-center gap-2 py-8">
                <div className="flex items-center justify-between text-sm">
                  <span className="text-foreground">{job ? jobPhaseLabels[job.phase] : "Instalando mods…"}</span>
                  {job && job.total > 0 && (
                    <span className="text-muted-foreground">
                      {job.done}/{job.total}
                    </span>
                  )}
                </div>
                <ProgressBar value={job && job.total > 0 ? (job.done / job.total) * 100 : 0} />
              </div>
            ) : (
              <>
                <p className="mt-1 text-sm text-muted-foreground">
                  Mods compatíveis com {instance.mcVersion} e {instance.loader.type}. Você confere a lista antes de
                  instalar.
                </p>

                <div className="mt-4 flex flex-1 flex-col gap-1">
                  {optimizations.map((entry) => {
                    const alreadyInstalled = entry.compat.status === "alreadyInstalled";
                    const incompatible = entry.compat.status === "incompatible";
                    return (
                      <label
                        key={entry.projectId}
                        className={`flex items-center gap-3 rounded-lg px-2 py-2 ${incompatible || alreadyInstalled ? "opacity-50" : "cursor-pointer hover:bg-surface-2"}`}
                      >
                        <input
                          type="checkbox"
                          className="h-4 w-4 accent-primary"
                          checked={alreadyInstalled || selected.has(entry.projectId)}
                          disabled={alreadyInstalled || incompatible}
                          onChange={() => toggle(entry.projectId)}
                        />
                        <div className="min-w-0 flex-1">
                          <p className="text-sm font-medium text-foreground">{entry.name}</p>
                          <p className="truncate text-xs text-muted-foreground">
                            {entry.compat.status === "alreadyInstalled"
                              ? "Já instalado"
                              : entry.compat.status === "incompatible"
                                ? entry.compat.reason
                                : entry.description}
                          </p>
                        </div>
                      </label>
                    );
                  })}
                </div>

                <div className="mt-4 flex items-center justify-between">
                  <p className="text-xs text-muted-foreground">
                    {selected.size} de {optimizations.length} selecionados
                  </p>
                  <Button size="sm" disabled={selected.size === 0 || install.isPending} onClick={() => install.mutate()}>
                    {install.isPending ? <Loader2 size={14} className="mr-1.5 animate-spin" /> : null}
                    Instalar {selected.size} mod(s)
                  </Button>
                </div>
                {install.isError && <p className="mt-2 text-xs text-destructive">{install.error.message}</p>}
              </>
            )}
          </div>
        ) : null}

        <InstalledContentList instanceId={instance.id} />
      </div>

      <div className="flex flex-col gap-4">
        <div className="rounded-xl border border-border bg-card p-5">
          <div className="mb-2 flex items-center justify-between">
            <p className="text-xs font-medium tracking-wide text-muted-foreground uppercase">Memória</p>
            <span className="text-sm font-semibold text-primary">{ramGb} GB</span>
          </div>
          <Slider
            min={RAM_MIN_GB}
            max={RAM_MAX_GB}
            step={1}
            value={[ramGb]}
            onValueChange={([value]) => setRamGb(value)}
            onValueCommit={() => saveSettings.mutate({ ramGb, preset })}
          />
        </div>

        <div className="rounded-xl border border-border bg-card p-5">
          <p className="mb-2 text-xs font-medium tracking-wide text-muted-foreground uppercase">Flags da JVM</p>
          <Select
            value={preset}
            onValueChange={(value) => {
              const next = value as JvmFlagPreset;
              setPreset(next);
              saveSettings.mutate({ ramGb, preset: next });
            }}
          >
            <SelectTrigger className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="none">{JVM_PRESET_LABEL.none}</SelectItem>
              <SelectItem value="g1gcOptimized">{JVM_PRESET_LABEL.g1gcOptimized}</SelectItem>
            </SelectContent>
          </Select>
        </div>

        <Button variant="outline" size="sm" className="gap-1.5" onClick={() => void openInstanceFolder(instance.id)}>
          <FolderOpen size={14} />
          Abrir pasta
        </Button>
      </div>
    </div>
  );
}
