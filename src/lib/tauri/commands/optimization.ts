import { callCommand } from "../client";
import type { InstanceCompat } from "./mods";

export interface OptimizationEntry {
  projectId: string;
  name: string;
  description: string;
  compat: InstanceCompat;
}

export function getRecommendedOptimizations(instanceId: string): Promise<OptimizationEntry[]> {
  return callCommand<OptimizationEntry[]>("get_recommended_optimizations", { instanceId });
}

export function installOptimizations(instanceId: string, projectIds: string[]): Promise<string> {
  return callCommand<string>("install_optimizations", { instanceId, projectIds });
}
