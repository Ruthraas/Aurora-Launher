import type { InstallStage } from "@/lib/tauri/commands/instances";

export const installStageLabels: Record<InstallStage, string> = {
  fetchingMetadata: "Buscando metadados",
  client: "Baixando client.jar",
  libraries: "Baixando bibliotecas",
  natives: "Extraindo natives",
  assets: "Baixando assets",
  javaRuntime: "Baixando Java",
  processors: "Aplicando patches do loader",
  optionsFile: "Configurando options.txt para otimização...",
};
