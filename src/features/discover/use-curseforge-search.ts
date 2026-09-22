import { useQuery, keepPreviousData } from "@tanstack/react-query";
import { searchCurseForge } from "@/lib/tauri/commands/curseforge";
import type { ProjectType } from "@/lib/tauri/commands/modrinth";

const PAGE_SIZE = 20;

export function useCurseForgeSearch(
  query: string,
  projectType: ProjectType,
  page: number,
  enabled: boolean,
  limit = PAGE_SIZE,
) {
  return useQuery({
    queryKey: ["curseforge-search", projectType, query, page, limit],
    queryFn: () => searchCurseForge({ query, projectType, offset: page * limit, limit }),
    placeholderData: keepPreviousData,
    enabled,
    retry: false,
  });
}

export { PAGE_SIZE };
