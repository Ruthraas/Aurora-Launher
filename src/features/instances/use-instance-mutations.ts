import { useMutation, useQueryClient } from "@tanstack/react-query";
import {
  createInstance,
  deleteInstance,
  launchInstance,
  retryInstanceInstall,
  type CreateInstanceParams,
  type Instance,
} from "@/lib/tauri/commands/instances";
import type { AppErrorPayload } from "@/lib/tauri/types";
import { instancesQueryKey } from "./use-instances";

export function useCreateInstance() {
  const queryClient = useQueryClient();
  return useMutation<Instance, AppErrorPayload, CreateInstanceParams>({
    mutationFn: createInstance,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: instancesQueryKey }),
  });
}

export function useRetryInstanceInstall() {
  const queryClient = useQueryClient();
  return useMutation<Instance, AppErrorPayload, string>({
    mutationFn: retryInstanceInstall,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: instancesQueryKey }),
  });
}

export function useDeleteInstance() {
  const queryClient = useQueryClient();
  return useMutation<null, AppErrorPayload, string>({
    mutationFn: deleteInstance,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: instancesQueryKey }),
  });
}

export function useLaunchInstance() {
  return useMutation<null, AppErrorPayload, string>({
    mutationFn: launchInstance,
  });
}
