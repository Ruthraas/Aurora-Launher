import { useMutation, useQueryClient } from "@tanstack/react-query";
import {
  addOfflineAccount,
  logout,
  removeAccount,
  setActiveAccount,
  type Account,
  type AppErrorPayload,
} from "@/lib/tauri/commands/accounts";
import { accountsQueryKey } from "./use-accounts";

export function useAddOfflineAccount() {
  const queryClient = useQueryClient();
  return useMutation<Account, AppErrorPayload, string>({
    mutationFn: addOfflineAccount,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: accountsQueryKey }),
  });
}

export function useRemoveAccount() {
  const queryClient = useQueryClient();
  return useMutation<null, AppErrorPayload, string>({
    mutationFn: removeAccount,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: accountsQueryKey }),
  });
}

export function useSetActiveAccount() {
  const queryClient = useQueryClient();
  return useMutation<null, AppErrorPayload, string>({
    mutationFn: setActiveAccount,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: accountsQueryKey }),
  });
}

export function useLogout() {
  const queryClient = useQueryClient();
  return useMutation<null, AppErrorPayload, void>({
    mutationFn: logout,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: accountsQueryKey }),
  });
}
