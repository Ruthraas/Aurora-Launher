import { useQuery } from "@tanstack/react-query";
import { listAccounts } from "@/lib/tauri/commands/accounts";

export const accountsQueryKey = ["accounts"];

export function useAccounts() {
  return useQuery({ queryKey: accountsQueryKey, queryFn: listAccounts });
}
