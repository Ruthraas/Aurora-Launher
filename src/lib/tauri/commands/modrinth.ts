import { callCommand } from "../client";

export type ProjectType = "mod" | "modpack" | "resourcepack" | "shader" | "datapack";

export interface ModrinthHit {
  projectId: string;
  slug: string;
  projectType: ProjectType;
  title: string;
  description: string;
  iconUrl: string | null;
  downloads: number;
  categories: string[];
  versions: string[];
}

export interface ModrinthSearchResponse {
  hits: ModrinthHit[];
  totalHits: number;
  offset: number;
  limit: number;
}

export interface SearchModrinthParams {
  query: string;
  projectType: ProjectType;
  mcVersion?: string;
  offset: number;
  limit: number;
}

export function searchModrinth(params: SearchModrinthParams): Promise<ModrinthSearchResponse> {
  return callCommand<ModrinthSearchResponse>("search_modrinth", { ...params });
}
