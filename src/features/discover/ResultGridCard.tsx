import type { DiscoverResult, ResultAction } from "./ResultRow";
import { ActionButton } from "./ResultRow";

const SOURCE_LABEL: Record<DiscoverResult["source"], string> = {
  modrinth: "Modrinth",
  curseforge: "CurseForge",
};

export function ResultGridCard({ result, action }: { result: DiscoverResult; action: ResultAction }) {
  return (
    <div className="flex flex-col gap-2.5 rounded-lg border border-border bg-card p-3.5 transition-colors hover:border-primary/40">
      <a href={result.url} target="_blank" rel="noreferrer" className="flex items-start gap-3 hover:opacity-80">
        {result.iconUrl ? (
          <img src={result.iconUrl} alt="" className="h-11 w-11 shrink-0 rounded-md object-cover" loading="lazy" />
        ) : (
          <div className="flex h-11 w-11 shrink-0 items-center justify-center rounded-md bg-surface-2 text-lg font-semibold text-muted-foreground">
            {result.title.charAt(0).toUpperCase()}
          </div>
        )}
        <div className="min-w-0">
          <p className="truncate text-sm font-semibold text-foreground">{result.title}</p>
          <p className="text-xs text-muted-foreground">{SOURCE_LABEL[result.source]}</p>
        </div>
      </a>

      <p className="line-clamp-2 text-xs text-muted-foreground">{result.description}</p>

      <div className="mt-auto">
        <ActionButton action={action} />
      </div>
    </div>
  );
}
