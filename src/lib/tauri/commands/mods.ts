import { callCommand } from "../client";

export type ModSource = "modrinth" | "curseforge";
export type ContentType = "mod" | "resourcePack" | "shader" | "datapack";

export interface ProjectRef {
  source: ModSource;
  projectId: string;
  /** Ignorado pelo fluxo de modpack — opcional só por isso. */
  contentType?: ContentType;
  title?: string;
  iconUrl?: string;
}

export interface ModRef {
  source: ModSource;
  projectId: string;
  contentType: ContentType;
  versionId: string;
  versionNumber: string;
  fileName: string;
  downloadUrl: string;
  sha1: string;
  title: string | null;
  iconUrl: string | null;
}

export type InstanceCompat =
  | { status: "compatible"; modRef: ModRef; depsToInstall: ModRef[] }
  | { status: "alreadyInstalled"; installedVersion: string }
  | { status: "incompatible"; reason: string };

export interface InstanceCompatEntry {
  instanceId: string;
  compat: InstanceCompat;
}

export function getModCompatibility(project: ProjectRef): Promise<InstanceCompatEntry[]> {
  return callCommand<InstanceCompatEntry[]>("get_mod_compatibility", { project });
}

export function installMod(project: ProjectRef, instanceIds: string[]): Promise<string> {
  return callCommand<string>("install_mod", { project, instanceIds });
}

export function cancelJob(jobId: string): Promise<null> {
  return callCommand<null>("cancel_job", { jobId });
}

export interface InstalledModEntry {
  source: ModSource;
  projectId: string;
  versionId: string;
  versionNumber: string;
  fileName: string;
  sha1: string;
  installedAt: string;
  explicit: boolean;
  title: string | null;
  iconUrl: string | null;
}

export function listInstanceMods(instanceId: string): Promise<InstalledModEntry[]> {
  return callCommand<InstalledModEntry[]>("list_instance_mods", { instanceId });
}

export function removeInstanceMod(instanceId: string, source: ModSource, projectId: string): Promise<null> {
  return callCommand<null>("remove_instance_mod", { instanceId, source, projectId });
}

export type JobPhase =
  | "fetchingMetadata"
  | "creatingInstance"
  | "downloading"
  | "extracting"
  | "configuringOptions"
  | "finished"
  | "cancelled"
  | "failed";

export interface JobProgressEvent {
  jobId: string;
  phase: JobPhase;
  done: number;
  total: number;
  error: string | null;
}
