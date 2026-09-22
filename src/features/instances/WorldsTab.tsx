import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { FolderOpen, Globe, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { EmptyState } from "@/components/empty-state";
import { ConfirmDialog } from "@/components/confirm-dialog";
import type { Instance, WorldInfo } from "@/lib/tauri/commands/instances";
import { deleteInstanceWorld, getWorldIcon, listInstanceWorlds, openInstanceWorldFolder } from "@/lib/tauri/commands/instances";

function formatSize(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${Math.round(bytes / 1024)} KB`;
}

function WorldRow({ instanceId, world, onDeleted }: { instanceId: string; world: WorldInfo; onDeleted: () => void }) {
  const [confirming, setConfirming] = useState(false);
  const { data: iconDataUrl } = useQuery({
    queryKey: ["world-icon", instanceId, world.folderName],
    queryFn: async () => {
      const base64 = await getWorldIcon(instanceId, world.folderName);
      return base64 ? `data:image/png;base64,${base64}` : null;
    },
    enabled: world.hasIcon,
  });

  const remove = useMutation({
    mutationFn: () => deleteInstanceWorld(instanceId, world.folderName),
    onSuccess: onDeleted,
  });

  return (
    <div className="group flex items-center gap-3 py-3">
      {iconDataUrl ? (
        <img src={iconDataUrl} alt="" className="h-10 w-10 shrink-0 rounded-lg object-cover" loading="lazy" />
      ) : (
        <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-surface-2 text-muted-foreground">
          <Globe size={18} />
        </div>
      )}
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium text-foreground">{world.folderName}</p>
        <p className="text-xs text-muted-foreground">
          {formatSize(world.sizeBytes)}
          {world.lastPlayed && ` · ${new Intl.DateTimeFormat("pt-BR", { day: "2-digit", month: "short", year: "numeric" }).format(new Date(world.lastPlayed))}`}
        </p>
      </div>
      <Button
        variant="ghost"
        size="icon-sm"
        className="opacity-0 transition-opacity group-hover:opacity-100"
        onClick={() => void openInstanceWorldFolder(instanceId, world.folderName)}
        title="Abrir pasta do mundo"
      >
        <FolderOpen size={14} />
      </Button>
      <Button
        variant="ghost"
        size="icon-sm"
        className="opacity-0 transition-opacity group-hover:opacity-100"
        onClick={() => setConfirming(true)}
        title="Excluir mundo"
      >
        <Trash2 size={14} />
      </Button>

      <ConfirmDialog
        open={confirming}
        onOpenChange={setConfirming}
        title={`Excluir "${world.folderName}"?`}
        description="Apaga o mundo pra sempre — todo o progresso salvo nele. Não tem como desfazer."
        confirmLabel="Excluir"
        onConfirm={() => {
          remove.mutate();
          setConfirming(false);
        }}
      />
    </div>
  );
}

/** Aba "Mundos" — lista as pastas de `saves/` da instância (o launcher
 *  nunca teve visualizador nenhum pra isso, só dava pra ver navegando
 *  manualmente até a pasta). Só leitura + apagar; renomear mundo mexe
 *  no `level.dat` (NBT) e fica fora do escopo por ora. */
export function WorldsTab({ instance }: { instance: Instance }) {
  const queryClient = useQueryClient();
  const { data: worlds, isLoading } = useQuery({
    queryKey: ["instance-worlds", instance.id],
    queryFn: () => listInstanceWorlds(instance.id),
  });

  function invalidate() {
    void queryClient.invalidateQueries({ queryKey: ["instance-worlds", instance.id] });
  }

  if (isLoading) return <Skeleton className="h-64 w-full rounded-xl" />;

  if (!worlds || worlds.length === 0) {
    return <EmptyState icon={Globe} title="Nenhum mundo ainda" description="Mundos aparecem aqui depois que você criar um dentro do jogo." />;
  }

  return (
    <div className="rounded-xl border border-border bg-card p-5">
      <p className="text-xs font-medium tracking-wide text-muted-foreground uppercase">Mundos ({worlds.length})</p>
      <div className="mt-2 divide-y divide-border">
        {worlds.map((world) => (
          <WorldRow key={world.folderName} instanceId={instance.id} world={world} onDeleted={invalidate} />
        ))}
      </div>
    </div>
  );
}
