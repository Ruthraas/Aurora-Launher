import type { ProjectType } from "@/lib/tauri/commands/modrinth";
import type { ContentType } from "@/lib/tauri/commands/mods";

/** `ProjectType` (o que a busca do Modrinth/CurseForge devolve) e
 * `ContentType` (o que `install_mod` espera) são quase a mesma coisa —
 * só "resourcepack" vira "resourcePack" (o backend usa `camelCase`
 * pro enum `ContentType`, mas o Modrinth manda `resourcepack` tudo
 * junto pro `project_type`). "modpack" não converte: modpack não se
 * instala numa instância existente, cria uma nova (ver `DownloadModpackDialog`). */
export function contentTypeFromProjectType(type: ProjectType): ContentType | null {
  switch (type) {
    case "mod":
      return "mod";
    case "resourcepack":
      return "resourcePack";
    case "shader":
      return "shader";
    case "datapack":
      return "datapack";
    case "modpack":
      return null;
  }
}

export const PROJECT_TYPE_LABEL: Record<ProjectType, string> = {
  mod: "Mods",
  modpack: "Modpacks",
  resourcepack: "Resource Packs",
  shader: "Shaders",
  datapack: "Datapacks",
};
