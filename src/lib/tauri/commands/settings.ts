import { callCommand } from "../client";

export interface Settings {
  curseforgeApiKey: string | null;
}

export function getSettings(): Promise<Settings> {
  return callCommand<Settings>("get_settings");
}

export function setCurseForgeApiKey(apiKey: string | null): Promise<null> {
  return callCommand<null>("set_curseforge_api_key", { apiKey });
}
