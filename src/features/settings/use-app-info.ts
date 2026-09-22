import { useQuery } from "@tanstack/react-query";
import { getAppInfo } from "@/lib/tauri/commands/app-info";

export function useAppInfo() {
  return useQuery({ queryKey: ["app-info"], queryFn: getAppInfo });
}
