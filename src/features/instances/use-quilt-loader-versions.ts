import { useQuery } from "@tanstack/react-query";
import { fetchQuiltLoaderVersions } from "@/lib/tauri/commands/instances";

export function useQuiltLoaderVersions(mcVersion: string, enabled: boolean) {
  return useQuery({
    queryKey: ["quilt-loader-versions", mcVersion],
    queryFn: () => fetchQuiltLoaderVersions(mcVersion),
    enabled: enabled && mcVersion.length > 0,
  });
}
