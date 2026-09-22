import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Check, Copy, FileText, RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { EmptyState } from "@/components/empty-state";
import type { Instance } from "@/lib/tauri/commands/instances";
import { listInstanceLogs, readInstanceLog } from "@/lib/tauri/commands/instances";

function formatSize(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  if (bytes >= 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${bytes} B`;
}

/** Aba "Logs" — expõe o `logs/launcher-std{out,err}.log` que o
 *  launcher já grava a cada lançamento (ver core::instances::launch) e
 *  os crash-reports do jogo, sem o usuário precisar navegar até a
 *  pasta manualmente. Só leitura — nada aqui edita ou apaga arquivo. */
export function LogsTab({ instance }: { instance: Instance }) {
  const [selected, setSelected] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  const { data: files, isLoading: loadingFiles, refetch: refetchFiles } = useQuery({
    queryKey: ["instance-logs", instance.id],
    queryFn: () => listInstanceLogs(instance.id),
  });

  useEffect(() => {
    if (files && files.length > 0 && (!selected || !files.some((f) => f.fileName === selected))) {
      setSelected(files[0].fileName);
    }
    if (files && files.length === 0) setSelected(null);
  }, [files, selected]);

  const {
    data: content,
    isLoading: loadingContent,
    refetch: refetchContent,
  } = useQuery({
    queryKey: ["instance-log-content", instance.id, selected],
    queryFn: () => readInstanceLog(instance.id, selected!),
    enabled: Boolean(selected),
  });

  function refresh() {
    void refetchFiles();
    void refetchContent();
  }

  async function copyContent() {
    if (!content) return;
    await navigator.clipboard.writeText(content);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }

  if (loadingFiles) return <Skeleton className="h-96 w-full rounded-xl" />;

  if (!files || files.length === 0) {
    return (
      <EmptyState
        icon={FileText}
        title="Nenhum log ainda"
        description="Logs aparecem aqui depois que a instância for lançada ao menos uma vez."
      />
    );
  }

  return (
    <div className="flex h-full flex-col gap-3">
      <div className="flex items-center gap-2">
        <Select value={selected ?? undefined} onValueChange={setSelected}>
          <SelectTrigger className="w-72">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {files.map((file) => (
              <SelectItem key={file.fileName} value={file.fileName}>
                {file.fileName} · {formatSize(file.sizeBytes)}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Button variant="outline" size="icon" onClick={refresh} title="Atualizar">
          <RefreshCw size={14} />
        </Button>
        <Button variant="outline" size="sm" className="gap-1.5" onClick={() => void copyContent()} disabled={!content}>
          {copied ? <Check size={14} /> : <Copy size={14} />}
          {copied ? "Copiado" : "Copiar"}
        </Button>
      </div>

      <div className="scroll-thin min-h-0 flex-1 overflow-auto rounded-xl border border-border bg-card p-4">
        {loadingContent ? (
          <p className="text-sm text-muted-foreground">Carregando…</p>
        ) : (
          <pre className="font-mono text-xs whitespace-pre-wrap text-foreground">{content || "(arquivo vazio)"}</pre>
        )}
      </div>
    </div>
  );
}
