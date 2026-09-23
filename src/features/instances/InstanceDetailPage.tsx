import { useState } from "react";
import { Link, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { Blocks, FileText, Globe, LayoutGrid } from "lucide-react";
import { PlayButton } from "@/components/play-button";
import { cn } from "@/lib/utils";
import { useInstances } from "./use-instances";
import { useInstallEvents } from "./use-install-events";
import { useLaunchInstance } from "./use-instance-mutations";
import { OverviewTab } from "./OverviewTab";
import { ModsTab } from "./ModsTab";
import { LogsTab } from "./LogsTab";
import { WorldsTab } from "./WorldsTab";
import { OptimizeButton } from "./OptimizeButton";
import { InstanceHeader } from "./InstanceHeader";
import { useJobEvents } from "@/features/discover/use-job-events";

type Tab = "overview" | "mods" | "worlds" | "logs";

const TABS: { value: Tab; labelKey: string; icon: typeof LayoutGrid }[] = [
  { value: "overview", labelKey: "instanceDetail.tabs.overview", icon: LayoutGrid },
  { value: "mods", labelKey: "instanceDetail.tabs.mods", icon: Blocks },
  { value: "worlds", labelKey: "instanceDetail.tabs.worlds", icon: Globe },
  { value: "logs", labelKey: "instanceDetail.tabs.logs", icon: FileText },
];

export function InstanceDetailPage() {
  const { t } = useTranslation();
  useInstallEvents();
  useJobEvents();
  const { id } = useParams<{ id: string }>();
  const { data: instances, isLoading } = useInstances();
  const launchInstance = useLaunchInstance();
  const [tab, setTab] = useState<Tab>("overview");

  const instance = instances?.find((i) => i.id === id);

  if (isLoading) return null;
  if (!instance) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3">
        <p className="text-sm text-muted-foreground">{t("instanceDetail.notFound")}</p>
        <Link to="/instances" className="text-sm text-primary hover:underline">
          {t("instanceDetail.backToInstances")}
        </Link>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between gap-4 border-b border-border px-6 py-4">
        <InstanceHeader instance={instance} />
        {(instance.status === "ready" || instance.status === "incomplete") && (
          <div className="flex shrink-0 items-center gap-2">
            <OptimizeButton instanceId={instance.id} />
            <PlayButton disabled={launchInstance.isPending} onClick={() => launchInstance.mutate(instance.id)}>
              {launchInstance.isPending ? t("instanceCard.opening") : t("instanceCard.play")}
            </PlayButton>
          </div>
        )}
      </div>

      <div className="flex gap-5 border-b border-border px-6">
        {TABS.map(({ value, labelKey, icon: Icon }) => (
          <button
            key={value}
            type="button"
            onClick={() => setTab(value)}
            className={cn(
              "flex cursor-pointer items-center gap-1.5 border-b-2 py-2.5 text-sm font-medium transition-colors",
              tab === value ? "border-primary text-foreground" : "border-transparent text-muted-foreground hover:text-foreground",
            )}
          >
            <Icon size={14} />
            {t(labelKey)}
          </button>
        ))}
      </div>

      <div className="scroll-thin flex-1 overflow-y-auto p-6">
        {tab === "overview" ? (
          <OverviewTab instance={instance} />
        ) : tab === "mods" ? (
          <ModsTab instance={instance} />
        ) : tab === "worlds" ? (
          <WorldsTab instance={instance} />
        ) : (
          <LogsTab instance={instance} />
        )}
      </div>
    </div>
  );
}
