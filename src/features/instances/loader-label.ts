import type { LoaderKind } from "@/lib/tauri/commands/instances";

/** Só o nome (ex.: "Fabric"), sem versão — pra telas que mostram a
 * versão separada (ex.: `DownloadModpackDialog`). */
export function loaderName(loader: LoaderKind): string {
  switch (loader.type) {
    case "fabric":
      return "Fabric";
    case "neoforge":
      return "NeoForge";
    case "forge":
      return "Forge";
    case "quilt":
      return "Quilt";
    default:
      return "Vanilla";
  }
}

/** Nome + versão (ex.: "Fabric 0.15.0") — pra telas compactas que
 * mostram os dois juntos. */
export function loaderLabel(loader: LoaderKind): string {
  switch (loader.type) {
    case "fabric":
      return `Fabric ${loader.loaderVersion}`;
    case "neoforge":
      return `NeoForge ${loader.neoforgeVersion}`;
    case "forge":
      return `Forge ${loader.forgeVersion}`;
    case "quilt":
      return `Quilt ${loader.loaderVersion}`;
    default:
      return "Vanilla";
  }
}
