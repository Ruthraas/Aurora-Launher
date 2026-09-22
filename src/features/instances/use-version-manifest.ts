import { useQuery } from "@tanstack/react-query";
import { fetchVersionManifest } from "@/lib/tauri/commands/minecraft";

export function useVersionManifest() {
  return useQuery({
    queryKey: ["version-manifest"],
    queryFn: fetchVersionManifest,
    staleTime: 1000 * 60 * 30,
  });
}
