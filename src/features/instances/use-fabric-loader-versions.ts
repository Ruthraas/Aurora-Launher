import { useQuery } from "@tanstack/react-query";
import { fetchFabricLoaderVersions } from "@/lib/tauri/commands/instances";

export function useFabricLoaderVersions(mcVersion: string, enabled: boolean) {
  return useQuery({
    queryKey: ["fabric-loader-versions", mcVersion],
    queryFn: () => fetchFabricLoaderVersions(mcVersion),
    enabled: enabled && mcVersion.length > 0,
  });
}
