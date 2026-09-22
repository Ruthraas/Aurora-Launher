import { callCommand } from "../client";

export interface AppInfo {
  name: string;
  version: string;
  identifier: string;
}

export function getAppInfo(): Promise<AppInfo> {
  return callCommand<AppInfo>("get_app_info");
}
