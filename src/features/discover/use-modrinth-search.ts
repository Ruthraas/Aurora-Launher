import { useQuery, keepPreviousData } from "@tanstack/react-query";
import { searchModrinth, type ProjectType } from "@/lib/tauri/commands/modrinth";

const PAGE_SIZE = 20;

export function useModrinthSearch(
  query: string,
  projectType: ProjectType,
  page: number,
  enabled = true,
  limit = PAGE_SIZE,
) {
  return useQuery({
    queryKey: ["modrinth-search", projectType, query, page, limit],
    queryFn: () => searchModrinth({ query, projectType, offset: page * limit, limit }),
    placeholderData: keepPreviousData,
    enabled,
  });
}

export { PAGE_SIZE };
