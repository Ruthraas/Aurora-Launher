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
            <div key={`${mod.source}:${mod.projectId}`} className="group flex items-center gap-3 py-3">
              {mod.iconUrl ? (
                <img src={mod.iconUrl} alt="" className="h-10 w-10 shrink-0 rounded-lg object-cover" loading="lazy" />
              ) : (
                <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-surface-2 text-sm font-semibold text-muted-foreground">
                  {(mod.title ?? mod.fileName).charAt(0).toUpperCase()}
                </div>
              )}
              <div className="min-w-0 flex-1">
                <p className="truncate text-sm font-medium text-foreground">{mod.title ?? mod.fileName}</p>
                <div className="mt-1 flex flex-wrap items-center gap-1.5">
                  <span className="rounded-full border border-border bg-surface-2 px-2 py-0.5 text-[11px] font-medium text-muted-foreground">
                    v{mod.versionNumber}
                  </span>
                  <span className="rounded-full border border-border bg-surface-2 px-2 py-0.5 text-[11px] font-medium text-muted-foreground">
                    {mod.source === "modrinth" ? "Modrinth" : "CurseForge"}
                  </span>
                  {!mod.explicit && (
                    <span className="rounded-full border border-border px-2 py-0.5 text-[11px] font-medium text-muted-foreground">
                      dependência
                    </span>
                  )}
                </div>
              </div>
              <Button
                variant="ghost"
                size="icon-sm"
                className="opacity-0 transition-opacity group-hover:opacity-100"
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
