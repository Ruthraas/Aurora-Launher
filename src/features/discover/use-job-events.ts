import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useQueryClient } from "@tanstack/react-query";
import type { JobProgressEvent } from "@/lib/tauri/commands/mods";
import { useJobProgressStore } from "./job-progress-store";
import { instancesQueryKey } from "@/features/instances/use-instances";

/** Liga `job://progress` (instalar mod / baixar modpack) à store — e
 *  invalida a lista de instâncias quando um job termina, já que ambos
 *  os jobs podem mudar o que aparece lá (mod instalado, instância nova
 *  de um modpack). */
export function useJobEvents() {
  const setProgress = useJobProgressStore((state) => state.setProgress);
  const queryClient = useQueryClient();

  useEffect(() => {
    const unlisten = listen<JobProgressEvent>("job://progress", (event) => {
      setProgress(event.payload);
      if (["finished", "cancelled", "failed"].includes(event.payload.phase)) {
        void queryClient.invalidateQueries({ queryKey: instancesQueryKey });
      }
    }).catch(() => undefined);

    return () => {
      void unlisten.then((fn) => fn?.());
    };
  }, [setProgress, queryClient]);
}
