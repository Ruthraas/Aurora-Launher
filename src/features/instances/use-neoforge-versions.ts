import { useQuery } from "@tanstack/react-query";
import { fetchNeoForgeVersions } from "@/lib/tauri/commands/instances";

export function useNeoForgeVersions(mcVersion: string, enabled: boolean) {
  return useQuery({
    queryKey: ["neoforge-versions", mcVersion],
    queryFn: () => fetchNeoForgeVersions(mcVersion),
    enabled: enabled && mcVersion.length > 0,
  });
}
