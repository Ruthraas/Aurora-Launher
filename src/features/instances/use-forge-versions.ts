import { useQuery } from "@tanstack/react-query";
import { fetchForgeVersions } from "@/lib/tauri/commands/instances";

export function useForgeVersions(mcVersion: string, enabled: boolean) {
  return useQuery({
    queryKey: ["forge-versions", mcVersion],
    queryFn: () => fetchForgeVersions(mcVersion),
    enabled: enabled && mcVersion.length > 0,
  });
}
