import { useTranslation } from "react-i18next";
import { PageHeader } from "@/components/page-header";
import { ContentBrowser } from "./ContentBrowser";

export function DiscoverPage() {
  const { t } = useTranslation();

  return (
    <div className="flex h-full flex-col">
      <PageHeader title={t("nav.discover")} subtitle="Modrinth e CurseForge na mesma busca" />
      <div className="min-h-0 flex-1 px-6 py-4">
        <ContentBrowser scope="global" />
      </div>
    </div>
  );
}
