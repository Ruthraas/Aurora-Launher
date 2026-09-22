import { useEffect, useMemo, useRef, useState } from "react";
import { Search, Rows3, LayoutGrid, Grid3x3, Plus, Download } from "lucide-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Chip } from "@/components/chip";
import { cn } from "@/lib/utils";
import { ResultRow, type DiscoverResult, type ResultAction } from "./ResultRow";
import { ResultGridCard } from "./ResultGridCard";
import { AddModDialog } from "./AddModDialog";
import { DownloadModpackDialog } from "./DownloadModpackDialog";
import { useJobEvents } from "./use-job-events";
import { useJobProgressStore } from "./job-progress-store";
import { useModrinthSearch, PAGE_SIZE as MODRINTH_PAGE_SIZE } from "./use-modrinth-search";
import { useCurseForgeSearch, PAGE_SIZE as CURSEFORGE_PAGE_SIZE } from "./use-curseforge-search";
import { contentTypeFromProjectType, PROJECT_TYPE_LABEL } from "./content-type";
import { getSettings } from "@/lib/tauri/commands/settings";
import { installMod, listInstanceMods } from "@/lib/tauri/commands/mods";
import type { ProjectType } from "@/lib/tauri/commands/modrinth";

type Source = "both" | "modrinth" | "curseforge";
const LOADER_TAGS = ["fabric", "quilt", "forge", "neoforge"] as const;
type LoaderTag = (typeof LOADER_TAGS)[number] | "all";

type LayoutMode = "list" | "grid" | "compact";
const LAYOUT_STORAGE_KEY = "aurora-discover-layout";
const LAYOUT_OPTIONS: { value: LayoutMode; label: string; icon: typeof Rows3 }[] = [
  { value: "list", label: "Lista", icon: Rows3 },
  { value: "grid", label: "Grade", icon: LayoutGrid },
  { value: "compact", label: "Grade compacta", icon: Grid3x3 },
];

function readStoredLayout(): LayoutMode {
  try {
    const stored = localStorage.getItem(LAYOUT_STORAGE_KEY);
    if (stored === "list" || stored === "grid" || stored === "compact") return stored;
  } catch {
    // localStorage indisponível (ex.: contexto privado) — cai no padrão
  }
  return "list";
}

function useDebouncedValue<T>(value: T, delayMs: number): T {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const timer = setTimeout(() => setDebounced(value), delayMs);
    return () => clearTimeout(timer);
  }, [value, delayMs]);
  return debounced;
}

export type BrowserScope = "global" | { instanceId: string };

const GLOBAL_TYPES: ProjectType[] = ["mod", "modpack"];
const INSTANCE_TYPES: ProjectType[] = ["mod", "resourcepack", "shader", "datapack"];

export function ContentBrowser({ scope }: { scope: BrowserScope }) {
  useJobEvents();
  const queryClient = useQueryClient();
  const allowedTypes = scope === "global" ? GLOBAL_TYPES : INSTANCE_TYPES;

  const [projectType, setProjectType] = useState<ProjectType>(allowedTypes[0]);
  const [source, setSource] = useState<Source>("both");
  const [loaderTag, setLoaderTag] = useState<LoaderTag>("all");
  const [query, setQuery] = useState("");
  const [page, setPage] = useState(0);
  const [addModTarget, setAddModTarget] = useState<DiscoverResult | null>(null);
  const [downloadModpackTarget, setDownloadModpackTarget] = useState<DiscoverResult | null>(null);
  const [layout, setLayout] = useState<LayoutMode>(readStoredLayout);
  const debouncedQuery = useDebouncedValue(query, 350);

  useEffect(() => {
    try {
      localStorage.setItem(LAYOUT_STORAGE_KEY, layout);
    } catch {
      // sem storage disponível — a escolha só não sobrevive a um reload
    }
  }, [layout]);

  const { data: settings } = useQuery({ queryKey: ["settings"], queryFn: getSettings });
  const hasCurseForgeKey = Boolean(settings?.curseforgeApiKey);
  const wantsModrinth = source === "both" || source === "modrinth";
  const wantsCurseForge = (source === "both" || source === "curseforge") && hasCurseForgeKey;

  useEffect(() => {
    setPage(0);
  }, [source, projectType, debouncedQuery]);

  const pageSize = source === "both" ? 10 : source === "modrinth" ? MODRINTH_PAGE_SIZE : CURSEFORGE_PAGE_SIZE;
  const modrinth = useModrinthSearch(debouncedQuery, projectType, page, wantsModrinth, pageSize);
  const curseforge = useCurseForgeSearch(debouncedQuery, projectType, page, wantsCurseForge, pageSize);

  const isLoading = (wantsModrinth && modrinth.isLoading) || (wantsCurseForge && curseforge.isLoading);
  const isError = source === "modrinth" ? modrinth.isError : source === "curseforge" ? curseforge.isError : false;

  const allResults: DiscoverResult[] = useMemo(() => {
    const fromModrinth: DiscoverResult[] =
      wantsModrinth && modrinth.data
        ? modrinth.data.hits.map((hit) => ({
            id: `modrinth:${hit.projectId}`,
            projectId: hit.projectId,
            source: "modrinth" as const,
            projectType: hit.projectType,
            title: hit.title,
            description: hit.description,
            iconUrl: hit.iconUrl,
            downloads: hit.downloads,
            categories: hit.categories,
            url: `https://modrinth.com/${hit.projectType}/${hit.slug}`,
          }))
        : [];
    const fromCurseForge: DiscoverResult[] =
      wantsCurseForge && curseforge.data
        ? curseforge.data.hits.map((hit) => ({
            id: `curseforge:${hit.id}`,
            projectId: String(hit.id),
            source: "curseforge" as const,
            projectType,
            title: hit.name,
            description: hit.summary,
            iconUrl: hit.logo?.thumbnailUrl ?? null,
            downloads: hit.downloadCount,
            categories: hit.categories.map((c) => c.name),
            url: `https://www.curseforge.com/minecraft/${projectType === "mod" ? "mc-mods" : projectType === "modpack" ? "modpacks" : projectType === "resourcepack" ? "texture-packs" : projectType === "shader" ? "shaders" : "data-packs"}/${hit.slug}`,
          }))
        : [];
    return [...fromModrinth, ...fromCurseForge];
  }, [wantsModrinth, wantsCurseForge, modrinth.data, curseforge.data, projectType]);

  const results =
    loaderTag === "all" || projectType !== "mod"
      ? allResults
      : allResults.filter((r) => r.categories.some((c) => c.toLowerCase() === loaderTag));

  const totalHits =
    source === "modrinth" ? modrinth.data?.totalHits : source === "curseforge" ? curseforge.data?.totalHits : undefined;
  const totalPages = totalHits ? Math.max(1, Math.ceil(totalHits / pageSize)) : 1;
  const showPagination = source !== "both" && totalHits !== undefined && totalHits > pageSize;

  // instância específica: instala direto (sem escolher instância) e
  // mostra o estado ali na própria linha do resultado.
  const instanceId = scope === "global" ? null : scope.instanceId;
  const { data: installedMods } = useQuery({
    queryKey: ["instance-mods", instanceId],
    queryFn: () => listInstanceMods(instanceId as string),
    enabled: Boolean(instanceId),
  });
  const installedProjectIds = new Set((installedMods ?? []).map((m) => m.projectId));
  const [installingId, setInstallingId] = useState<string | null>(null);
  const [justInstalledId, setJustInstalledId] = useState<string | null>(null);
  const installJob = useJobProgressStore((state) => (installingId ? state.jobs[installingId] : undefined));

  const directInstall = useMutation({
    mutationFn: (result: DiscoverResult) => {
      const contentType = contentTypeFromProjectType(result.projectType);
      if (!contentType) throw new Error("tipo de conteúdo inválido");
      return installMod(
        { source: result.source, projectId: result.projectId, contentType, title: result.title, iconUrl: result.iconUrl ?? undefined },
        [instanceId as string],
      );
    },
    onSuccess: (id, result) => {
      setInstallingId(id);
      setJustInstalledId(result.id);
    },
  });

  // mesmo cuidado do OverviewTab: invalidar tem que ser um efeito, não
  // rodar direto no corpo do componente (senão reagenda a cada render
  // enquanto o job fica "finished" parado).
  const handledInstallId = useRef<string | null>(null);
  useEffect(() => {
    if (installJob?.phase !== "finished" || !instanceId || !installingId || handledInstallId.current === installingId) {
      return;
    }
    handledInstallId.current = installingId;
    void queryClient.invalidateQueries({ queryKey: ["instance-mods", instanceId] });
  }, [installJob?.phase, instanceId, installingId, queryClient]);

  function actionFor(result: DiscoverResult): ResultAction {
    if (result.projectType === "modpack") {
      return { label: "Baixar", icon: Download, onClick: () => setDownloadModpackTarget(result) };
    }
    if (instanceId) {
      const already = installedProjectIds.has(result.projectId) || justInstalledId === result.id;
      return {
        label: already ? "Instalado" : "Adicionar",
        icon: Plus,
        disabled: already,
        loading: directInstall.isPending && directInstall.variables?.id === result.id,
        onClick: () => directInstall.mutate(result),
      };
    }
    return { label: "Adicionar", icon: Plus, onClick: () => setAddModTarget(result) };
  }

  return (
    <div className="flex h-full flex-col">
      <div className="flex flex-col gap-3 border-b border-border pb-4">
        <div className="relative">
          <Search className="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-muted-foreground" size={14} />
          <Input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Buscar…"
            className="h-10 pl-8 text-sm"
          />
        </div>

        {allowedTypes.length > 1 && (
          <div className="flex flex-wrap gap-5 border-b border-border">
            {allowedTypes.map((type) => (
              <button
                key={type}
                type="button"
                onClick={() => setProjectType(type)}
                className={cn(
                  "cursor-pointer border-b-2 pb-2 text-sm font-medium whitespace-nowrap transition-colors",
                  projectType === type
                    ? "border-primary text-foreground"
                    : "border-transparent text-muted-foreground hover:text-foreground",
                )}
              >
                {PROJECT_TYPE_LABEL[type]}
              </button>
            ))}
          </div>
        )}

        <div className="flex flex-wrap items-center gap-2">
          <Chip active={source === "both"} onClick={() => setSource("both")}>
            Ambos
          </Chip>
          <Chip active={source === "modrinth"} onClick={() => setSource("modrinth")}>
            Modrinth
          </Chip>
          <Chip active={source === "curseforge"} onClick={() => setSource("curseforge")}>
            CurseForge
          </Chip>

          {projectType === "mod" && !instanceId && (
            <>
              <div className="mx-1 h-4 w-px bg-border" />
              <Chip active={loaderTag === "all"} onClick={() => setLoaderTag("all")}>
                Todos os loaders
              </Chip>
              {LOADER_TAGS.map((tag) => (
                <Chip key={tag} active={loaderTag === tag} onClick={() => setLoaderTag(tag)}>
                  {tag === "fabric" ? "Fabric" : tag === "quilt" ? "Quilt" : tag === "forge" ? "Forge" : "NeoForge"}
                </Chip>
              ))}
            </>
          )}

          <div className="ml-auto flex gap-1 rounded-lg border border-border p-0.5">
            {LAYOUT_OPTIONS.map(({ value, label, icon: Icon }) => (
              <button
                key={value}
                type="button"
                title={label}
                aria-label={label}
                aria-pressed={layout === value}
                onClick={() => setLayout(value)}
                className={cn(
                  "flex h-6 w-7 cursor-pointer items-center justify-center rounded-md transition-colors",
                  layout === value ? "bg-primary/15 text-primary" : "text-muted-foreground hover:text-foreground",
                )}
              >
                <Icon size={14} />
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="scroll-thin flex-1 overflow-y-auto py-2">
        {source === "curseforge" && !hasCurseForgeKey ? (
          <p className="py-4 text-sm text-muted-foreground">
            Precisa configurar a chave da API do CurseForge primeiro — vai em Configurações.
          </p>
        ) : isError ? (
          <p className="py-4 text-sm text-destructive">Não deu pra buscar agora — tenta de novo em instantes.</p>
        ) : isLoading ? (
          <p className="py-4 text-sm text-muted-foreground">Carregando…</p>
        ) : results.length === 0 ? (
          <p className="py-4 text-sm text-muted-foreground">Nada encontrado.</p>
        ) : layout === "list" ? (
          <div className="divide-y divide-border">
            {results.map((result) => (
              <ResultRow key={result.id} result={result} action={actionFor(result)} />
            ))}
          </div>
        ) : (
          <div
            className="grid gap-3 py-3"
            style={{ gridTemplateColumns: `repeat(auto-fill, minmax(${layout === "compact" ? 180 : 240}px, 1fr))` }}
          >
            {results.map((result) => (
              <ResultGridCard key={result.id} result={result} action={actionFor(result)} />
            ))}
          </div>
        )}
      </div>

      {showPagination && (
        <div className="flex items-center justify-center gap-3 border-t border-border py-3">
          <Button variant="outline" size="sm" disabled={page === 0} onClick={() => setPage((p) => p - 1)}>
            Anterior
          </Button>
          <span className="text-xs text-muted-foreground">
            Página {page + 1} de {totalPages}
          </span>
          <Button variant="outline" size="sm" disabled={page + 1 >= totalPages} onClick={() => setPage((p) => p + 1)}>
            Próxima
          </Button>
        </div>
      )}

      {directInstall.isError && <p className="pt-2 text-xs text-destructive">{directInstall.error.message}</p>}

      {addModTarget && <AddModDialog result={addModTarget} onClose={() => setAddModTarget(null)} />}
      {downloadModpackTarget && (
        <DownloadModpackDialog result={downloadModpackTarget} onClose={() => setDownloadModpackTarget(null)} />
      )}
    </div>
  );
}
