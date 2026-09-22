import { useTranslation } from "react-i18next";
import { PageHeader } from "@/components/page-header";
import { SectionLabel } from "@/components/section-label";
import { LanguageSwitch } from "./LanguageSwitch";
import { AppInfoBadge } from "./AppInfoBadge";
import { CurseForgeApiKeySetting } from "./CurseForgeApiKeySetting";

export function SettingsPage() {
  const { t } = useTranslation();

  return (
    <div className="flex h-full flex-col">
      <PageHeader title={t("settings.title")} />
      <div className="scroll-thin flex-1 overflow-y-auto p-6">
        <div className="flex max-w-xl flex-col gap-3">
          <SectionLabel>{t("settings.language")}</SectionLabel>
          <div className="rounded-lg border border-border bg-card px-4 py-3.5">
            <LanguageSwitch />
          </div>
        </div>

        <div className="mt-6 flex max-w-xl flex-col gap-3">
          <SectionLabel>CurseForge</SectionLabel>
          <div className="rounded-lg border border-border bg-card px-4 py-3.5">
            <CurseForgeApiKeySetting />
          </div>
        </div>

        <div className="mt-6 flex max-w-xl flex-col gap-3">
          <SectionLabel>{t("settings.about")}</SectionLabel>
          <div className="rounded-lg border border-border bg-card px-4 py-3.5">
            <p className="text-sm text-muted-foreground">{t("home.disclaimer")}</p>
            <div className="mt-3 border-t border-border pt-3">
              <AppInfoBadge />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
