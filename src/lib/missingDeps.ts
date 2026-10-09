/** A mod the game refused to start without, as reported by Forge / NeoForge. */
export interface MissingDep {
  modId: string;
  requestedBy: string[];
  /** Installed version when the problem is the version, not the absence. */
  installed: string | null;
}

const HEADER = "Missing or unsupported mandatory dependencies";
// `Mod ID: 'architectury', Requested by: 'pandalib', Expected range: '*', Actual version: '[MISSING]'`
const ENTRY = /Mod ID: '([^']+)', Requested by: '([^']+)'.*?Actual version: '([^']*)'/;

/**
 * Reads the loader's dependency report from the game output. Fabric refuses to start
 * and exits instead, which the crash panel already explains.
 */
export function parseMissingDeps(lines: readonly string[]): MissingDep[] {
  const start = lines.findIndex((l) => l.includes(HEADER));
  if (start < 0) return [];
  const found = new Map<string, MissingDep>();
  for (const line of lines.slice(start + 1, start + 200)) {
    const m = ENTRY.exec(line);
    if (!m) continue;
    const [, modId, requestedBy, actual] = m;
    const entry = found.get(modId) ?? { modId, requestedBy: [], installed: actual === "[MISSING]" ? null : actual };
    if (!entry.requestedBy.includes(requestedBy)) entry.requestedBy.push(requestedBy);
    found.set(modId, entry);
  }
  return [...found.values()];
}
