import { invoke } from "@tauri-apps/api/core";

/**
 * Wrapper fino sobre `invoke`. Todo comando exposto pelo Rust deve ter
 * uma função tipada em `lib/tauri/commands/*` que chama isto — nada na
 * UI deve importar `@tauri-apps/api` diretamente.
 */
export function callCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(command, args);
}
