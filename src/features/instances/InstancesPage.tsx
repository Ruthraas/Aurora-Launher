import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Boxes, Search } from "lucide-react";
import { PageHeader } from "@/components/page-header";
import { EmptyState } from "@/components/empty-state";
import { Skeleton } from "@/components/ui/skeleton";
import { Input } from "@/components/ui/input";
import { Chip } from "@/components/chip";
import { useInstances } from "./use-instances";
import { useInstallEvents } from "./use-install-events";
import { CreateInstanceDialog } from "./CreateInstanceDialog";
import { ImportInstanceButton } from "./ImportInstanceButton";
import { InstanceCard } from "./InstanceCard";
import type { LoaderKind } from "@/lib/tauri/commands/instances";

type LoaderFilter = "all" | LoaderKind["type"];

const LOADER_FILTER_KEYS: { value: LoaderFilter; labelKey: string }[] = [
  { value: "all", labelKey: "instancesPage.filters.all" },
  { value: "vanilla", labelKey: "instancesPage.filters.vanilla" },
  { value: "fabric", labelKey: "instancesPage.filters.fabric" },
  { value: "forge", labelKey: "instancesPage.filters.forge" },
  { value: "neoforge", labelKey: "instancesPage.filters.neoforge" },
  { value: "quilt", labelKey: "instancesPage.filters.quilt" },
];

export function InstancesPage() {
  const { t } = useTranslation();
  useInstallEvents();
  const { data: instances, isLoading } = useInstances();
  const [query, setQuery] = useState("");
  const [loaderFilter, setLoaderFilter] = useState<LoaderFilter>("all");

  const filtered = useMemo(() => {
    if (!instances) return [];
    const q = query.trim().toLowerCase();
    return instances
      .filter((instance) => loaderFilter === "all" || instance.loader.type === loaderFilter)
      .filter((instance) => instance.name.toLowerCase().includes(q));
  }, [instances, query, loaderFilter]);

  return (
    <div className="flex h-full flex-col">
      <PageHeader
        title={t("instancesPage.title")}
        actions={
          <>
            <div className="relative hidden sm:block">
              <Search className="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-muted-foreground" size={14} />
              <Input
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder={t("instancesPage.searchPlaceholder")}
                className="h-8 w-44 pl-8 text-sm"
              />
            </div>
            <ImportInstanceButton />
            <CreateInstanceDialog />
          </>
        }
      />

      <div className="flex flex-wrap gap-2 border-b border-border px-6 py-3">
        {LOADER_FILTER_KEYS.map((filter) => (
          <Chip key={filter.value} active={loaderFilter === filter.value} onClick={() => setLoaderFilter(filter.value)}>
            {t(filter.labelKey)}
          </Chip>
        ))}
      </div>

      <div className="scroll-thin flex-1 overflow-y-auto p-6">
        {isLoading ? (
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
            {Array.from({ length: 4 }).map((_, i) => (
              <Skeleton key={i} className="h-24 rounded-lg" />
            ))}
          </div>
        ) : !instances || instances.length === 0 ? (
          <EmptyState icon={Boxes} title={t("instances.emptyTitle")} description={t("instances.emptyDescription")} />
        ) : filtered.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t("instancesPage.noMatch")}</p>
        ) : (
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
            {filtered.map((instance) => (
              <InstanceCard key={instance.id} instance={instance} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
