import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { listInstanceMods, removeInstanceMod } from "@/lib/tauri/commands/mods";

export function InstalledContentList({ instanceId }: { instanceId: string }) {
  const queryClient = useQueryClient();
  const { data: installed, isLoading } = useQuery({
    queryKey: ["instance-mods", instanceId],
    queryFn: () => listInstanceMods(instanceId),
  });

  const remove = useMutation({
    mutationFn: (mod: { source: "modrinth" | "curseforge"; projectId: string }) =>
      removeInstanceMod(instanceId, mod.source, mod.projectId),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["instance-mods", instanceId] }),
  });

  return (
    <div className="rounded-xl border border-border bg-card p-5">
      <p className="text-xs font-medium tracking-wide text-muted-foreground uppercase">
        Instalado {installed && `(${installed.length})`}
      </p>
      {isLoading ? (
        <p className="mt-2 text-sm text-muted-foreground">Carregando…</p>
      ) : !installed || installed.length === 0 ? (
        <p className="mt-2 text-sm text-muted-foreground">Nada instalado ainda — busca na aba Mods pra adicionar.</p>
      ) : (
        <div className="mt-3 divide-y divide-border">
          {installed.map((mod) => (
            <div key={`${mod.source}:${mod.projectId}`} className="flex items-center gap-3 py-2.5">
              {mod.iconUrl ? (
                <img src={mod.iconUrl} alt="" className="h-8 w-8 shrink-0 rounded-md object-cover" loading="lazy" />
              ) : (
                <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-surface-2 text-xs font-semibold text-muted-foreground">
                  {(mod.title ?? mod.fileName).charAt(0).toUpperCase()}
                </div>
              )}
              <div className="min-w-0 flex-1">
                <p className="truncate text-sm font-medium text-foreground">{mod.title ?? mod.fileName}</p>
                <p className="text-xs text-muted-foreground">
                  v{mod.versionNumber} · {mod.source === "modrinth" ? "Modrinth" : "CurseForge"}
                  {!mod.explicit && " · dependência"}
                </p>
              </div>
              <Button
                variant="ghost"
                size="icon-sm"
                disabled={remove.isPending}
                onClick={() => remove.mutate({ source: mod.source, projectId: mod.projectId })}
              >
                <Trash2 size={14} />
              </Button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
