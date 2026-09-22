import { Layers, Loader2, type LucideIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { ProjectType } from "@/lib/tauri/commands/modrinth";

export interface DiscoverResult {
  id: string;
  projectId: string;
  source: "modrinth" | "curseforge";
  projectType: ProjectType;
  title: string;
  description: string;
  iconUrl: string | null;
  downloads: number;
  categories: string[];
  url: string;
}

export interface ResultAction {
  label: string;
  icon: LucideIcon;
  onClick: () => void;
  disabled?: boolean;
  loading?: boolean;
}

const SOURCE_LABEL: Record<DiscoverResult["source"], string> = {
  modrinth: "Modrinth",
  curseforge: "CurseForge",
};

function ActionButton({ action }: { action: ResultAction }) {
  const Icon = action.icon;
  return (
    <Button
      variant="outline"
      size="sm"
      className="shrink-0 gap-1 border-primary/40 text-primary hover:bg-primary/10"
      disabled={action.disabled || action.loading}
      onClick={action.onClick}
    >
      {action.loading ? <Loader2 size={13} className="animate-spin" /> : <Icon size={13} />}
      {action.label}
    </Button>
  );
}

export function ResultRow({ result, action }: { result: DiscoverResult; action: ResultAction }) {
  return (
    <div className="flex items-center gap-3 py-3">
      {result.iconUrl ? (
        <img src={result.iconUrl} alt="" className="h-9 w-9 shrink-0 rounded-md object-cover" loading="lazy" />
      ) : (
        <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md bg-surface-2 text-muted-foreground">
          <Layers size={16} />
        </div>
      )}

      <a href={result.url} target="_blank" rel="noreferrer" className="min-w-0 flex-1 hover:opacity-80">
        <p className="flex items-baseline gap-2 truncate">
          <span className="text-sm font-semibold text-foreground">{result.title}</span>
          <span className="text-xs text-muted-foreground">{SOURCE_LABEL[result.source]}</span>
        </p>
        <p className="truncate text-xs text-muted-foreground">{result.description}</p>
      </a>

      <ActionButton action={action} />
    </div>
  );
}

export { ActionButton };
