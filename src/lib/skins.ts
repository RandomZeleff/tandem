import type { SkinVariant } from "./api";

/** Face of each body part on the skin texture: [x, y, width, height]. */
type Rect = [number, number, number, number];

interface Part {
  base: Rect;
  overlay: Rect | null;
  /** Where it goes on the 16×32 front view. */
  at: [number, number];
  /** Drawn from another part, mirrored (old 64×32 skins have one arm and one leg). */
  mirrorOf?: Rect;
}

function parts(variant: SkinVariant, legacy: boolean): Part[] {
  const arm = variant === "slim" ? 3 : 4;
  const rightArm: Rect = [44, 20, arm, 12];
  const rightLeg: Rect = [4, 20, 4, 12];
  return [
    { base: [8, 8, 8, 8], overlay: [40, 8, 8, 8], at: [4, 0] },
    { base: [20, 20, 8, 12], overlay: legacy ? null : [20, 36, 8, 12], at: [4, 8] },
    // The player's right arm and leg are on the viewer's left.
    { base: rightArm, overlay: legacy ? null : [44, 36, arm, 12], at: [4 - arm, 8] },
    legacy
      ? { base: rightArm, overlay: null, at: [12, 8], mirrorOf: rightArm }
      : { base: [36, 52, arm, 12], overlay: [52, 52, arm, 12], at: [12, 8] },
    { base: rightLeg, overlay: legacy ? null : [4, 36, 4, 12], at: [4, 20] },
    legacy
      ? { base: rightLeg, overlay: null, at: [8, 20], mirrorOf: rightLeg }
      : { base: [20, 52, 4, 12], overlay: [4, 52, 4, 12], at: [8, 20] },
  ];
}

const fronts = new Map<string, Promise<string>>();

/** The player seen from the front (16×32 PNG), for thumbnails. Cached per texture. */
export function skinFront(texture: string, variant: SkinVariant): Promise<string> {
  const key = `${variant}:${texture}`;
  let front = fronts.get(key);
  if (!front) {
    front = draw(texture, variant);
    fronts.set(key, front);
  }
  return front;
}

async function draw(texture: string, variant: SkinVariant): Promise<string> {
  const image = new Image();
  image.src = texture;
  await image.decode();
  const legacy = image.height === 32;
  const canvas = document.createElement("canvas");
  canvas.width = 16;
  canvas.height = 32;
  const ctx = canvas.getContext("2d")!;
  for (const part of parts(variant, legacy)) {
    const [x, y, w, h] = part.base;
    if (part.mirrorOf) {
      ctx.save();
      ctx.translate(part.at[0] + w, part.at[1]);
      ctx.scale(-1, 1);
      ctx.drawImage(image, x, y, w, h, 0, 0, w, h);
      ctx.restore();
      continue;
    }
    ctx.drawImage(image, x, y, w, h, part.at[0], part.at[1], w, h);
    if (part.overlay) {
      const [ox, oy, ow, oh] = part.overlay;
      ctx.drawImage(image, ox, oy, ow, oh, part.at[0], part.at[1], ow, oh);
    }
  }
  return canvas.toDataURL();
}

/** Front of a cape texture (64×32): 10×16 pixels at (1, 1). */
export const CAPE_FRONT = { x: 1, y: 1, width: 10, height: 16, textureWidth: 64, textureHeight: 32 };
