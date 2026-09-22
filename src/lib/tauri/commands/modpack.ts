import { callCommand } from "../client";
import type { LoaderKind } from "./instances";
import type { ProjectRef } from "./mods";

export interface ModpackVersionOption {
  id: string;
  label: string;
}

export interface ModpackPreview {
  versionId: string;
  mcVersion: string;
  loader: LoaderKind;
  modCount: number;
  downloadSizeBytes: number;
  suggestedRamMb: number;
  versions: ModpackVersionOption[];
}

export function getModpackPreview(project: ProjectRef, version?: string): Promise<ModpackPreview> {
  return callCommand<ModpackPreview>("get_modpack_preview", { project, version });
}

export function installModpack(project: ProjectRef, versionId: string, name: string): Promise<string> {
  return callCommand<string>("install_modpack", { project, versionId, name });
}
