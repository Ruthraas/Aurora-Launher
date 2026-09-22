import { Skeleton } from "@/components/ui/skeleton";
import { useAppInfo } from "./use-app-info";

/** Prova de ponta a ponta do canal de IPC: front → comando Tauri → Rust. */
export function AppInfoBadge() {
  const { data, isLoading, isError } = useAppInfo();

  if (isLoading) {
    return (
      <div className="flex items-center gap-2">
        <Skeleton className="h-2 w-2 rounded-full" />
        <Skeleton className="h-4 w-56" />
      </div>
    );
  }

  if (isError || !data) {
    return (
      <div className="text-destructive flex items-center gap-2 text-sm">
        <span className="bg-destructive h-2 w-2 rounded-full" />
        Falha ao consultar o backend Rust.
      </div>
    );
  }

  return (
    <div className="flex items-center gap-2 text-sm text-muted-foreground">
      <span className="bg-primary h-2 w-2 rounded-full" />
      <span>
        {data.name} v{data.version} · {data.identifier}
      </span>
    </div>
  );
}
