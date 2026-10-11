import type { Instance } from "./api";

/** Same keys as `tandem_core::optimizations`. */
export const OPTIMIZATIONS_SETTING = "tandem_optimizations";
export const ACTIVATION_SETTING = "tandem_optimizations_activation";
export const instanceOptimizationsKey = (id: string) => `${OPTIMIZATIONS_SETTING}.${id}`;

/** Fabric and Quilt, Minecraft 1.20 to 1.21.11: mirrors `optimizations::supports`. */
export function supportsOptimizations(instance: Pick<Instance, "loader" | "gameVersion">): boolean {
  if (instance.loader !== "fabric" && instance.loader !== "quilt") return false;
  const m = /^1\.(20|21)(?:\.(\d+))?$/.exec(instance.gameVersion);
  return m !== null && (m[1] === "20" || Number(m[2] ?? 0) <= 11);
}
