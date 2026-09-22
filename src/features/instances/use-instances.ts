import { useQuery } from "@tanstack/react-query";
import { listInstances } from "@/lib/tauri/commands/instances";

export const instancesQueryKey = ["instances"];

export function useInstances() {
  return useQuery({ queryKey: instancesQueryKey, queryFn: listInstances });
}
