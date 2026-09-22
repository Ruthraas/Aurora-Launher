import { callCommand } from "../client";

export type InstanceStatus = "notInstalled" | "installing" | "ready" | "incomplete" | "error";

export type LoaderKind =
  | { type: "vanilla" }
  | { type: "fabric"; loaderVersion: string }
  | { type: "neoforge"; neoforgeVersion: string }
  | { type: "forge"; forgeVersion: string }
  | { type: "quilt"; loaderVersion: string };

export type JvmFlagPreset = "none" | "g1gcOptimized";

export interface ModpackOrigin {
  source: "modrinth" | "curseforge";
  projectId: string;
  versionId: string;
}

export interface ManualDownload {
  fileName: string;
  projectUrl: string;
}

export interface Instance {
  schemaVersion: number;
  id: string;
  name: string;
  mcVersion: string;
  loader: LoaderKind;
  ramMinMb: number;
  ramMaxMb: number;
  jvmFlagPreset: JvmFlagPreset;
  customJvmArgs: string;
  status: InstanceStatus;
  errorMessage: string | null;
  modpackOrigin: ModpackOrigin | null;
  missingManualDownloads: ManualDownload[];
  createdAt: string;
  lastPlayed: string | null;
}

export function listInstances(): Promise<Instance[]> {
  return callCommand<Instance[]>("list_instances");
}

export interface CreateInstanceParams {
  name: string;
  mcVersion: string;
  loaderKind: "vanilla" | "fabric" | "neoforge" | "forge" | "quilt";
  loaderVersion?: string;
  ramMinMb: number;
  ramMaxMb: number;
}

export function createInstance(params: CreateInstanceParams): Promise<Instance> {
  return callCommand<Instance>("create_instance", { ...params });
}

export function deleteInstance(id: string): Promise<null> {
  return callCommand<null>("delete_instance", { id });
}

export function retryInstanceInstall(id: string): Promise<Instance> {
  return callCommand<Instance>("retry_instance_install", { id });
}

export function launchInstance(id: string): Promise<null> {
  return callCommand<null>("launch_instance", { id });
}

export function updateInstanceSettings(
  id: string,
  ramMinMb: number,
  ramMaxMb: number,
  jvmFlagPreset: JvmFlagPreset,
  customJvmArgs?: string,
): Promise<Instance> {
  return callCommand<Instance>("update_instance_settings", { id, ramMinMb, ramMaxMb, jvmFlagPreset, customJvmArgs });
}

export function openInstanceFolder(id: string): Promise<null> {
  return callCommand<null>("open_instance_folder", { id });
}

export interface FabricLoaderEntry {
  loader: { version: string; stable: boolean };
}

export function fetchFabricLoaderVersions(mcVersion: string): Promise<FabricLoaderEntry[]> {
  return callCommand<FabricLoaderEntry[]>("fetch_fabric_loader_versions", { mcVersion });
}

export function fetchNeoForgeVersions(mcVersion: string): Promise<string[]> {
  return callCommand<string[]>("fetch_neoforge_versions", { mcVersion });
}

export function fetchForgeVersions(mcVersion: string): Promise<string[]> {
  return callCommand<string[]>("fetch_forge_versions", { mcVersion });
}

export interface QuiltLoaderEntry {
  loader: { version: string };
}

export function fetchQuiltLoaderVersions(mcVersion: string): Promise<QuiltLoaderEntry[]> {
  return callCommand<QuiltLoaderEntry[]>("fetch_quilt_loader_versions", { mcVersion });
}

export type InstallStage =
  | "fetchingMetadata"
  | "client"
  | "libraries"
  | "natives"
  | "assets"
  | "javaRuntime"
  | "processors"
  | "optionsFile";

export interface InstallProgressEvent {
  instanceId: string;
  stage: InstallStage;
  completed: number;
  total: number;
}

export interface InstallFinishedEvent {
  instanceId: string;
  ok: boolean;
  error: string | null;
}
