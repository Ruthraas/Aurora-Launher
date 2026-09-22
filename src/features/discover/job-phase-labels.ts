import type { JobPhase } from "@/lib/tauri/commands/mods";

export const jobPhaseLabels: Record<JobPhase, string> = {
  fetchingMetadata: "Buscando metadados",
  creatingInstance: "Criando instância",
  downloading: "Baixando",
  extracting: "Extraindo",
  configuringOptions: "Configurando options.txt para otimização...",
  finished: "Concluído",
  cancelled: "Cancelado",
  failed: "Falhou",
};
