/** Deterministic visuals (block icon, banner, skin) derived from an id or a name. */

export interface BlockLook {
  top: string;
  shine: string;
  side: string;
  spot: string;
  /** Banner sky bands and hill colour for instance cards. */
  sky: [string, string];
  hill: string;
}

export interface SkinLook {
  skin: string;
  hair: string;
  eyes: string;
}

const BLOCKS: BlockLook[] = [
  { top: "#5DBB3F", shine: "#86D45E", side: "#8B5A2B", spot: "#6B4423", sky: ["#1A2A3A", "#22384E"], hill: "#3A5A2A" },
  { top: "#C98A3C", shine: "#E3AD61", side: "#7A5A3A", spot: "#5E432A", sky: ["#3A2A1A", "#5A3E22"], hill: "#7A5A3A" },
  { top: "#B3261E", shine: "#E04A3F", side: "#6E6E6E", spot: "#555555", sky: ["#2A1416", "#4A1C1E"], hill: "#6E2A26" },
  { top: "#7D7D7D", shine: "#9C9C9C", side: "#5F5F5F", spot: "#4A4A4A", sky: ["#1C1F26", "#2A2F38"], hill: "#4A4F58" },
  { top: "#E3D59A", shine: "#F2E8BE", side: "#D2C285", spot: "#B8A86E", sky: ["#3A2E1A", "#5E4A26"], hill: "#B8A86E" },
  { top: "#4FD6D0", shine: "#9AF0EB", side: "#3A6E8C", spot: "#2A5068", sky: ["#14283A", "#1C3A50"], hill: "#2A5068" },
  { top: "#9B6BE0", shine: "#B99CF2", side: "#3A2A5A", spot: "#2A1E44", sky: ["#1E1630", "#2E2248"], hill: "#4A3A86" },
];

const SKINS: SkinLook[] = [
  { skin: "#C69C6D", hair: "#4A2F1B", eyes: "#3B5BA8" },
  { skin: "#E0B48A", hair: "#C9832E", eyes: "#3E7A3A" },
  { skin: "#B9845A", hair: "#1E1612", eyes: "#5A3A22" },
  { skin: "#F0C8A0", hair: "#E8D27A", eyes: "#3B6EA8" },
  { skin: "#8D5A3A", hair: "#2A1A12", eyes: "#2A2A2A" },
];

function hash(value: string): number {
  let h = 2166136261;
  for (let i = 0; i < value.length; i++) {
    h ^= value.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return Math.abs(h);
}

export function blockLook(id: string): BlockLook {
  return BLOCKS[hash(id) % BLOCKS.length];
}

export function skinLook(name: string): SkinLook {
  return SKINS[hash(name.toLowerCase()) % SKINS.length];
}
