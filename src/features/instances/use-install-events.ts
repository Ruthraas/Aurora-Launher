import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useQueryClient } from "@tanstack/react-query";
import type { InstallFinishedEvent, InstallProgressEvent } from "@/lib/tauri/commands/instances";
import { useInstallProgressStore } from "./install-progress-store";
import { instancesQueryKey } from "./use-instances";

/** Liga os eventos de instalação (Rust → front) à store de progresso e
 *  invalida a lista de instâncias quando uma termina. */
export function useInstallEvents() {
  const setProgress = useInstallProgressStore((state) => state.setProgress);
  const clearProgress = useInstallProgressStore((state) => state.clearProgress);
  const queryClient = useQueryClient();

  useEffect(() => {
    const unlistenProgress = listen<InstallProgressEvent>("instance://install-progress", (event) => {
      setProgress(event.payload);
    }).catch(() => undefined);

    const unlistenFinished = listen<InstallFinishedEvent>("instance://install-finished", (event) => {
      clearProgress(event.payload.instanceId);
      void queryClient.invalidateQueries({ queryKey: instancesQueryKey });
    }).catch(() => undefined);

    return () => {
      void unlistenProgress.then((unlisten) => unlisten?.());
      void unlistenFinished.then((unlisten) => unlisten?.());
    };
  }, [setProgress, clearProgress, queryClient]);
}
