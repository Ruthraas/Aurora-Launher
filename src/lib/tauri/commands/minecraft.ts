import { callCommand } from "../client";

export interface VersionManifestEntry {
  id: string;
  type: "release" | "snapshot" | "old_beta" | "old_alpha";
  url: string;
  sha1: string;
  releaseTime: string;
}

export interface VersionManifest {
  latest: { release: string; snapshot: string };
  versions: VersionManifestEntry[];
}

export function fetchVersionManifest(): Promise<VersionManifest> {
  return callCommand<VersionManifest>("fetch_version_manifest");
}
