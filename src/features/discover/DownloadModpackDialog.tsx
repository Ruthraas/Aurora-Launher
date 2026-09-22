import { useEffect, useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { Download, Loader2 } from "lucide-react";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { getModpackPreview, installModpack } from "@/lib/tauri/commands/modpack";
import { useInstances } from "@/features/instances/use-instances";
import { loaderName } from "@/features/instances/loader-label";
import { useJobProgressStore } from "./job-progress-store";
import { jobPhaseLabels } from "./job-phase-labels";
import type { DiscoverResult } from "./ResultRow";

function formatBytes(bytes: number): string {
  if (bytes >= 1024 ** 3) return `≈ ${(bytes / 1024 ** 3).toFixed(1)} GB`;
  return `≈ ${Math.round(bytes / 1024 ** 2)} MB`;
}

function loaderVersion(loader: { type: string } & Record<string, string>): string {
  return loader.loaderVersion ?? loader.forgeVersion ?? loader.neoforgeVersion ?? "";
}

export function DownloadModpackDialog({ result, onClose }: { result: DiscoverResult; onClose: () => void }) {
  const { data: instances } = useInstances();
  const project = { source: result.source, projectId: result.projectId };
  const [versionId, setVersionId] = useState<string | undefined>(undefined);
  const [name, setName] = useState(result.title);
  const [jobId, setJobId] = useState<string | null>(null);
  const job = useJobProgressStore((state) => (jobId ? state.jobs[jobId] : undefined));

  const { data: preview, isLoading } = useQuery({
    queryKey: ["modpack-preview", result.source, result.projectId, versionId],
    queryFn: () => getModpackPreview(project, versionId),
  });

  useEffect(() => {
    if (preview && !versionId) setVersionId(preview.versionId);
  }, [preview, versionId]);

  useEffect(() => {
    if (!instances) return;
    const taken = new Set(instances.map((i) => i.name));
    if (!taken.has(result.title)) return;
    let n = 2;
    while (taken.has(`${result.title} (${n})`)) n++;
    setName(`${result.title} (${n})`);
  }, [instances, result.title]);

  const download = useMutation({
    mutationFn: () => installModpack(project, versionId ?? preview!.versionId, name.trim() || result.title),
    onSuccess: (id) => setJobId(id),
  });

  const finished = job?.phase === "finished";
  const failed = job?.phase === "failed";

  return (
    <Dialog
      open
      onOpenChange={(next) => {
        if (!next && (!job || finished || failed)) onClose();
      }}
    >
      <DialogContent className="sm:max-w-lg" aria-live="polite">
        <DialogHeader>
          <DialogTitle>Baixar {result.title}</DialogTitle>
        </DialogHeader>

        {!jobId ? (
          <>
            <p className="text-sm text-muted-foreground">
              O launcher cria uma instância nova com a versão do Minecraft e o loader exatos do modpack e baixa todos
              os mods.
            </p>

            <div>
              <label className="mb-1.5 block text-xs font-medium text-muted-foreground">Nome da instância</label>
              <Input value={name} onChange={(event) => setName(event.target.value)} />
            </div>

            {preview && preview.versions.length > 1 && (
              <div>
                <label className="mb-1.5 block text-xs font-medium text-muted-foreground">Versão do modpack</label>
                <Select value={versionId} onValueChange={setVersionId}>
                  <SelectTrigger className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {preview.versions.map((v) => (
                      <SelectItem key={v.id} value={v.id}>
                        {v.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            )}

            {isLoading ? (
              <p className="text-sm text-muted-foreground">Carregando…</p>
            ) : preview ? (
              <div className="rounded-lg border border-border bg-card text-sm">
                {[
                  ["Minecraft", preview.mcVersion],
                  ["Loader", `${loaderName(preview.loader)} ${loaderVersion(preview.loader as never)}`],
                  ["Mods", `${preview.modCount} mods`],
                  ["Download", formatBytes(preview.downloadSizeBytes)],
                  ["RAM sugerida", `${Math.round(preview.suggestedRamMb / 1024)} GB`],
                ].map(([label, value]) => (
                  <div key={label} className="flex items-center justify-between border-b border-border px-4 py-2.5 last:border-0">
                    <span className="text-muted-foreground">{label}</span>
                    <span className="font-medium text-foreground">{value}</span>
                  </div>
                ))}
              </div>
            ) : null}

            {download.isError && <p className="text-xs text-destructive">{download.error.message}</p>}

            <DialogFooter>
              <Button variant="outline" onClick={onClose}>
                Cancelar
              </Button>
              <Button disabled={!preview || download.isPending} onClick={() => download.mutate()} className="gap-1.5">
                {download.isPending && <Loader2 size={14} className="animate-spin" />}
                <Download size={14} />
                Baixar e criar instância
              </Button>
            </DialogFooter>
          </>
        ) : (
          <div className="flex flex-col items-center gap-3 py-6 text-center">
            {failed ? (
              <p className="text-sm text-destructive">{job?.error ?? "Falha ao instalar o modpack."}</p>
            ) : finished ? (
              <p className="text-sm text-primary">Instância criada — já pode jogar.</p>
            ) : (
              <>
                <Loader2 size={20} className="animate-spin text-primary" />
                <p className="text-sm text-muted-foreground">
                  {job ? `${jobPhaseLabels[job.phase]}${job.total > 0 ? ` — ${job.done}/${job.total}` : ""}` : "Preparando…"}
                </p>
              </>
            )}
            <DialogFooter className="w-full">
              <Button variant="outline" onClick={onClose}>
                Fechar
              </Button>
            </DialogFooter>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
