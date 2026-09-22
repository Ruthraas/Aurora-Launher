import { create } from "zustand";
import type { JobProgressEvent } from "@/lib/tauri/commands/mods";

interface JobProgressState {
  jobs: Record<string, JobProgressEvent>;
  setProgress: (event: JobProgressEvent) => void;
  clear: (jobId: string) => void;
}

/** Progresso de jobs (instalar mod / baixar modpack) é push (eventos
 *  Tauri `job://progress`), igual o progresso de instalação de
 *  instância — mesma razão pra usar Zustand em vez de uma query. */
export const useJobProgressStore = create<JobProgressState>((set) => ({
  jobs: {},
  setProgress: (event) => set((state) => ({ jobs: { ...state.jobs, [event.jobId]: event } })),
  clear: (jobId) =>
    set((state) => {
      const next = { ...state.jobs };
      delete next[jobId];
      return { jobs: next };
    }),
}));
