import { create } from "zustand";
import type { InstallProgressEvent } from "@/lib/tauri/commands/instances";

interface InstallProgressState {
  progress: Record<string, InstallProgressEvent>;
  setProgress: (event: InstallProgressEvent) => void;
  clearProgress: (instanceId: string) => void;
}

/** Progresso de instalação é push (eventos Tauri), não pull — por isso
 *  vive numa store Zustand, não numa query do TanStack. */
export const useInstallProgressStore = create<InstallProgressState>((set) => ({
  progress: {},
  setProgress: (event) => set((state) => ({ progress: { ...state.progress, [event.instanceId]: event } })),
  clearProgress: (instanceId) =>
    set((state) => {
      const next = { ...state.progress };
      delete next[instanceId];
      return { progress: next };
    }),
}));
