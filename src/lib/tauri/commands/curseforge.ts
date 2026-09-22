import { callCommand } from "../client";
import type { ProjectType } from "./modrinth";

export interface CurseForgeHit {
  id: number;
  slug: string;
  name: string;
  summary: string;
  downloadCount: number;
  logo: { thumbnailUrl: string | null } | null;
  categories: { name: string }[];
}

export interface CurseForgeSearchResponse {
  hits: CurseForgeHit[];
  totalHits: number;
}

export interface SearchCurseForgeParams {
  query: string;
  projectType: ProjectType;
  offset: number;
  limit: number;
}

export function searchCurseForge(params: SearchCurseForgeParams): Promise<CurseForgeSearchResponse> {
  return callCommand<CurseForgeSearchResponse>("search_curseforge", { ...params });
}
