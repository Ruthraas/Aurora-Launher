import { callCommand } from "../client";
export type { AppErrorPayload } from "../types";

export interface Account {
  kind: "offline";
  id: string;
  uuid: string;
  username: string;
  addedAt: string;
  skinFile: string | null;
  capeFile: string | null;
}

export interface AccountsResponse {
  accounts: Account[];
  activeAccountId: string | null;
}

export function listAccounts(): Promise<AccountsResponse> {
  return callCommand<AccountsResponse>("list_accounts");
}

export function addOfflineAccount(nickname: string): Promise<Account> {
  return callCommand<Account>("add_offline_account", { nickname });
}

export function removeAccount(id: string): Promise<null> {
  return callCommand<null>("remove_account", { id });
}

export function setActiveAccount(id: string): Promise<null> {
  return callCommand<null>("set_active_account", { id });
}

export function logout(): Promise<null> {
  return callCommand<null>("logout");
}

export function importAccountSkinByUsername(id: string, username: string): Promise<Account> {
  return callCommand<Account>("import_account_skin_by_username", { id, username });
}

export function uploadAccountSkin(id: string, pngBytes: number[]): Promise<Account> {
  return callCommand<Account>("upload_account_skin", { id, pngBytes });
}

export function uploadAccountCape(id: string, pngBytes: number[]): Promise<Account> {
  return callCommand<Account>("upload_account_cape", { id, pngBytes });
}

export function clearAccountSkin(id: string): Promise<Account> {
  return callCommand<Account>("clear_account_skin", { id });
}

export function clearAccountCape(id: string): Promise<Account> {
  return callCommand<Account>("clear_account_cape", { id });
}

export function getAccountTexture(id: string, texture: "skin" | "cape"): Promise<string | null> {
  return callCommand<string | null>("get_account_texture", { id, texture });
}
