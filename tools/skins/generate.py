"""Draws the Tandem skin pack (64x64 PNGs, Minecraft layout): shaded faces, and the
outer layer for hair, hoods, boots, belts and gear so they have depth in game.

    python tools/skins/generate.py            # writes crates/tandem-core/assets/skins
    python tools/skins/generate.py <folder>   # elsewhere, to try changes

Needs Pillow. The skins are Tandem's own work, under the project's licence."""
import random
import sys
from PIL import Image

from pathlib import Path

OUT = sys.argv[1] if len(sys.argv) > 1 else str(Path(__file__).resolve().parents[2] / "crates/tandem-core/assets/skins")

# Light from above and slightly in front.
FACE_LIGHT = {"top": 1.12, "front": 1.0, "right": 0.86, "left": 0.86, "back": 0.78, "bottom": 0.62}
SIDES = ("right", "front", "left", "back")


def hexc(h):
    h = h.lstrip("#")
    return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16), 255)


def shade(c, f):
    return tuple(max(0, min(255, round(v * f))) for v in c[:3]) + (c[3],)


def faces(u, v, w, h, d):
    return {
        "top": (u + d, v, w, d),
        "bottom": (u + d + w, v, w, d),
        "right": (u, v + d, d, h),
        "front": (u + d, v + d, w, h),
        "left": (u + d + w, v + d, d, h),
        "back": (u + 2 * d + w, v + d, w, h),
    }


class Part:
    """One box of the model, on the base or the outer layer."""

    def __init__(self, skin, u, v, w, h, d):
        self.s, self.f, self.h = skin, faces(u, v, w, h, d), h

    def paint(self, face, x, y, w, h, color, noise=0.05, gradient=True):
        fx, fy, fw, fh = self.f[face]
        for j in range(h):
            for i in range(w):
                px, py = x + i, y + j
                if not (0 <= px < fw and 0 <= py < fh):
                    continue
                f = FACE_LIGHT[face] * (1 + self.s.rng.uniform(-noise, noise))
                if gradient and face in SIDES and fh > 1:
                    f *= 1.06 - 0.14 * (py / (fh - 1))  # lighter at the top of a face
                self.s.img.putpixel((fx + px, fy + py), shade(color, f))

    def all(self, color, **kw):
        for face, (_, _, w, h) in self.f.items():
            self.paint(face, 0, 0, w, h, color, **kw)

    def rows(self, y, h, color, faces_=SIDES, **kw):
        for face in faces_:
            self.paint(face, 0, y, self.f[face][2], h, color, **kw)

    def dot(self, face, x, y, color):
        self.paint(face, x, y, 1, 1, color, noise=0, gradient=False)

    def width(self, face):
        return self.f[face][2]


class Skin:
    def __init__(self, seed, slim):
        self.img = Image.new("RGBA", (64, 64), (0, 0, 0, 0))
        self.rng = random.Random(seed)
        a = 3 if slim else 4
        self.head, self.hat = Part(self, 0, 0, 8, 8, 8), Part(self, 32, 0, 8, 8, 8)
        self.body, self.jacket = Part(self, 16, 16, 8, 12, 4), Part(self, 16, 32, 8, 12, 4)
        self.arms = [Part(self, 40, 16, a, 12, 4), Part(self, 32, 48, a, 12, 4)]
        self.sleeves = [Part(self, 40, 32, a, 12, 4), Part(self, 48, 48, a, 12, 4)]
        self.legs = [Part(self, 0, 16, 4, 12, 4), Part(self, 16, 48, 4, 12, 4)]
        self.pants = [Part(self, 0, 32, 4, 12, 4), Part(self, 0, 48, 4, 12, 4)]


def face_features(s, skin, eyes, brows, mouth):
    h = s.head
    for ex in (1, 5):
        h.dot("front", ex, 4, hexc("#F4F4F4"))
        h.dot("front", ex + 1, 4, eyes)
        h.dot("front", ex, 3, brows)
        h.dot("front", ex + 1, 3, brows)
    h.dot("front", 3, 5, shade(skin, 0.86))  # nose shadow
    h.dot("front", 4, 5, shade(skin, 0.86))
    h.paint("front", 3, 6, 2, 1, mouth, noise=0, gradient=False)
    h.paint("bottom", 0, 0, 8, 8, shade(skin, 0.9))


def miner():
    s = Skin(7, slim=False)
    skin, hair = hexc("#C8916A"), hexc("#4A2F1C")
    s.head.all(skin)
    s.head.paint("top", 0, 0, 8, 8, hair)
    for face in ("right", "left"):
        s.head.paint(face, 0, 0, 8, 4, hair)
        s.head.paint(face, 0, 4, 2, 1, hair)
    s.head.paint("back", 0, 0, 8, 7, hair)
    s.head.paint("front", 0, 0, 8, 2, hair)
    face_features(s, skin, hexc("#2E5C8A"), shade(hair, 0.85), hexc("#7A3E30"))
    beard = hexc("#5A3A22")
    s.head.paint("front", 1, 6, 6, 2, beard)
    s.head.paint("front", 0, 5, 1, 3, beard)
    s.head.paint("front", 7, 5, 1, 3, beard)
    s.head.paint("front", 3, 6, 2, 1, hexc("#7A3E30"), noise=0)

    # Outer layer: helmet with a brim, lamp, tufts of hair, a beard in relief.
    yellow = hexc("#F2B630")
    s.hat.paint("top", 0, 0, 8, 8, yellow)
    s.hat.paint("top", 3, 0, 2, 8, shade(yellow, 0.9))
    s.hat.rows(0, 2, yellow)
    s.hat.rows(2, 1, shade(yellow, 0.72), gradient=False)
    s.hat.paint("front", 3, 0, 2, 2, hexc("#5A5A5A"), noise=0)
    s.hat.dot("front", 3, 1, hexc("#FFF6C8"))
    s.hat.dot("front", 4, 1, hexc("#FFE680"))
    for face in ("right", "left"):
        s.hat.paint(face, 1, 3, 2, 2, shade(hair, 1.1))
    s.hat.paint("back", 1, 3, 6, 2, shade(hair, 1.1))
    s.hat.paint("front", 1, 7, 6, 1, shade(beard, 0.9))
    s.hat.paint("front", 2, 6, 1, 1, shade(beard, 0.9))
    s.hat.paint("front", 5, 6, 1, 1, shade(beard, 0.9))

    # Base: flannel shirt. Outer: denim overalls, straps, pocket, tool loop.
    shirt, denim = hexc("#5E6B58"), hexc("#3E5F8C")
    s.body.all(shirt)
    for face in SIDES:
        w = s.body.width(face)
        for i in range(w):
            if i % 3 == 0:
                s.body.paint(face, i, 0, 1, 12, shade(shirt, 0.8))
        for y in (2, 5, 8, 11):
            s.body.paint(face, 0, y, w, 1, shade(shirt, 0.8))
    s.jacket.rows(6, 6, denim)
    s.jacket.paint("front", 1, 0, 6, 6, denim)
    s.jacket.paint("front", 3, 0, 2, 6, (0, 0, 0, 0), noise=0)  # straps, not a full bib
    s.jacket.paint("front", 2, 2, 4, 4, denim)
    s.jacket.paint("front", 2, 3, 4, 2, shade(denim, 0.84))  # bib pocket
    s.jacket.dot("front", 1, 1, hexc("#E0C878"))
    s.jacket.dot("front", 6, 1, hexc("#E0C878"))
    s.jacket.paint("back", 1, 0, 2, 6, denim)
    s.jacket.paint("back", 5, 0, 2, 6, denim)
    s.jacket.rows(6, 1, shade(denim, 0.78), gradient=False)
    s.jacket.paint("front", 6, 7, 1, 4, hexc("#7A5A34"))  # hammer loop
    # Clear the transparent gap painted above (Part.paint blends nothing).
    fx, fy, _, _ = s.jacket.f["front"]
    for y in range(0, 2):
        for x in range(3, 5):
            s.img.putpixel((fx + x, fy + y), (0, 0, 0, 0))

    # Arms: skin, rolled sleeves (outer), work gloves (outer).
    glove = hexc("#9A7444")
    for arm, sleeve in zip(s.arms, s.sleeves):
        arm.all(skin)
        arm.rows(0, 5, shirt)
        arm.paint("top", 0, 0, 4, 4, shirt)
        arm.rows(9, 3, glove)
        arm.paint("bottom", 0, 0, 4, 4, shade(glove, 0.9))
        sleeve.rows(4, 1, shade(shirt, 1.05))
        sleeve.rows(9, 1, shade(glove, 1.08))

    # Legs: denim, chunky boots on the outer layer.
    boot = hexc("#3B2A1C")
    for leg, pant in zip(s.legs, s.pants):
        leg.all(denim)
        leg.rows(9, 3, boot)
        leg.paint("bottom", 0, 0, 4, 4, shade(boot, 0.8))
        pant.rows(8, 4, boot)
        pant.rows(8, 1, hexc("#5A4030"), gradient=False)
        pant.rows(11, 1, shade(boot, 0.7), gradient=False)
    return s.img


def explorer():
    s = Skin(11, slim=True)
    skin, hair, cloak = hexc("#E2B48E"), hexc("#B4532A"), hexc("#3F7A3A")
    s.head.all(skin)
    for face in ("top", "right", "left", "back"):
        w, h = s.head.f[face][2:]
        s.head.paint(face, 0, 0, w, h, hair)
    s.head.paint("front", 0, 0, 8, 2, hair)
    s.head.paint("front", 0, 2, 1, 6, hair)
    s.head.paint("front", 7, 2, 1, 6, hair)
    face_features(s, skin, hexc("#3C8A4A"), shade(hair, 0.85), hexc("#B8584A"))
    s.head.dot("front", 2, 5, hexc("#D8907A"))
    s.head.dot("front", 5, 5, hexc("#D8907A"))

    # Hood in relief, open on the face, a lock of hair falling out of it.
    for face in ("top", "right", "left", "back"):
        w, h = s.hat.f[face][2:]
        s.hat.paint(face, 0, 0, w, h, cloak)
    s.hat.paint("front", 0, 0, 8, 1, cloak)
    s.hat.paint("front", 0, 1, 1, 6, cloak)
    s.hat.paint("front", 7, 1, 1, 6, cloak)
    s.hat.paint("front", 1, 1, 6, 1, shade(cloak, 0.78), gradient=False)
    s.hat.paint("front", 1, 2, 1, 4, shade(hair, 1.08))
    s.hat.paint("back", 0, 7, 8, 1, shade(cloak, 0.8))

    # Base: leather tunic. Outer: cloak on the back, belt, bandolier, pouch.
    leather, dark = hexc("#8A5A34"), hexc("#4E331C")
    s.body.all(leather)
    s.body.paint("front", 3, 0, 2, 3, hexc("#E8DCC0"))
    for face in SIDES:
        s.body.paint(face, 0, 11, s.body.width(face), 1, shade(leather, 0.8))
    s.jacket.paint("back", 0, 0, 8, 12, cloak)
    s.jacket.paint("back", 0, 11, 8, 1, shade(cloak, 0.75), gradient=False)
    s.jacket.paint("top", 0, 0, 8, 4, cloak)
    for face in ("right", "left"):
        s.jacket.paint(face, 3, 0, 1, 12, cloak)
    s.jacket.rows(8, 1, dark, gradient=False)
    s.jacket.dot("front", 4, 8, hexc("#E0B040"))
    for i in range(7):
        s.jacket.dot("front", i, 1 + i, shade(dark, 1.15))
    s.jacket.paint("front", 5, 9, 3, 2, hexc("#6A4626"))  # pouch on the hip
    s.jacket.dot("front", 6, 9, hexc("#E0B040"))

    # Slim arms: sleeves, bracers in relief, bare hands.
    for arm, sleeve in zip(s.arms, s.sleeves):
        arm.all(leather)
        arm.rows(10, 2, skin)
        arm.paint("bottom", 0, 0, 3, 4, skin)
        sleeve.rows(6, 3, dark)
        sleeve.rows(6, 1, shade(dark, 1.25), gradient=False)

    # Legs: trousers, tall boots with a folded top on the outer layer.
    trousers, boot = hexc("#C9B48A"), hexc("#4A3322")
    for leg, pant in zip(s.legs, s.pants):
        leg.all(trousers)
        leg.rows(7, 5, boot)
        leg.paint("bottom", 0, 0, 4, 4, shade(boot, 0.8))
        pant.rows(6, 2, shade(boot, 1.25))
        pant.rows(11, 1, shade(boot, 0.7), gradient=False)
    return s.img




def pattern(part, face, x, y, rows, palette, noise=0.03):
    """Pixel art from strings: one character per pixel, '.' left untouched."""
    for j, row in enumerate(rows):
        for i, ch in enumerate(row):
            if ch != ".":
                part.paint(face, x + i, y + j, 1, 1, palette[ch], noise=noise)


def dino():
    s = Skin(21, slim=False)
    skin, hair = hexc("#E6B38E"), hexc("#6B4423")
    green, belly, spike = hexc("#5DB548"), hexc("#D9E87A"), hexc("#F08A2C")
    s.head.all(skin)
    s.head.paint("top", 0, 0, 8, 8, hair)
    s.head.paint("front", 0, 0, 8, 3, hair)
    face_features(s, skin, hexc("#5A3A22"), shade(hair, 0.8), hexc("#B05A4A"))
    s.head.dot("front", 2, 5, hexc("#EE9A86"))
    s.head.dot("front", 5, 5, hexc("#EE9A86"))
    # Hood: the dinosaur head, eyes on top, teeth over the forehead.
    for face in ("top", "right", "left", "back"):
        w, h = s.hat.f[face][2:]
        s.hat.paint(face, 0, 0, w, h, green)
    s.hat.paint("front", 0, 0, 8, 2, green)
    s.hat.paint("front", 0, 2, 1, 6, green)
    s.hat.paint("front", 7, 2, 1, 6, green)
    pattern(s.hat, "front", 1, 2, ["W.W.W.W"], {"W": hexc("#FAFAF0")}, noise=0)
    pattern(s.hat, "front", 1, 0, ["WW..WW", "WK..KW"],
            {"W": hexc("#FAFAF0"), "K": hexc("#1E1E1E")}, noise=0)
    s.hat.paint("top", 3, 0, 2, 4, spike)
    pattern(s.hat, "back", 3, 0, ["OO", ".O", "OO", "O.", "OO", ".O", "OO"], {"O": spike})
    s.hat.paint("right", 0, 7, 8, 1, shade(green, 0.8))
    s.hat.paint("left", 0, 7, 8, 1, shade(green, 0.8))
    # Onesie with a light belly, spikes down the back.
    s.body.all(green)
    s.body.paint("front", 2, 1, 4, 11, belly)
    for y in (3, 6, 9):
        s.body.paint("front", 2, y, 4, 1, shade(belly, 0.88))
    pattern(s.jacket, "back", 3, 0, ["OO", ".O", "OO", "O.", "OO", ".O", "OO", "O.", "OO", ".O", "OO"], {"O": spike})
    s.jacket.paint("front", 2, 9, 4, 2, shade(belly, 0.82))
    for arm, sleeve in zip(s.arms, s.sleeves):
        arm.all(green)
        arm.rows(10, 2, skin)
        arm.paint("bottom", 0, 0, 4, 4, skin)
        sleeve.rows(9, 1, shade(green, 1.12))
        pattern(sleeve, "front", 0, 11, ["W.W."], {"W": hexc("#FAFAF0")}, noise=0)
    for leg, pant in zip(s.legs, s.pants):
        leg.all(green)
        leg.rows(9, 3, shade(green, 0.9))
        leg.paint("bottom", 0, 0, 4, 4, belly)
        pant.rows(9, 3, green)
        pattern(pant, "front", 0, 11, ["WWWW"], {"W": hexc("#FAFAF0")}, noise=0)
    return s.img


def astronaut():
    s = Skin(33, slim=False)
    skin, hair = hexc("#8D5A3B"), hexc("#1E1A18")
    suit, orange, grey = hexc("#ECEEF0"), hexc("#F07A2A"), hexc("#8A9099")
    s.head.all(skin)
    s.head.paint("top", 0, 0, 8, 8, hair)
    s.head.paint("front", 0, 0, 8, 2, hair)
    face_features(s, skin, hexc("#3A2418"), shade(hair, 1.2), hexc("#5A2A22"))
    # Open helmet: a white shell framing the face, a red light on top.
    for face in ("top", "right", "left", "back"):
        w, h = s.hat.f[face][2:]
        s.hat.paint(face, 0, 0, w, h, suit)
    s.hat.paint("front", 0, 0, 8, 1, suit)
    s.hat.paint("front", 0, 0, 1, 8, suit)
    s.hat.paint("front", 7, 0, 1, 8, suit)
    s.hat.paint("front", 1, 1, 6, 1, hexc("#3A4A66"), noise=0)
    s.hat.paint("top", 3, 3, 2, 2, hexc("#E83A3A"), noise=0)
    s.hat.paint("right", 2, 3, 3, 3, grey)
    s.hat.paint("left", 3, 3, 3, 3, grey)
    s.hat.paint("back", 0, 7, 8, 1, orange)
    # Suit, chest panel, life-support pack on the back.
    s.body.all(suit)
    s.body.rows(10, 1, grey)
    pattern(s.jacket, "front", 1, 2, ["KKKKKK", "KRYGBK", "KKKKKK"],
            {"K": hexc("#3A3F48"), "R": hexc("#FF4A4A"), "Y": hexc("#FFD43A"), "G": hexc("#4AE06A"), "B": hexc("#3AB0FF")}, noise=0)
    s.jacket.paint("front", 5, 7, 2, 2, orange)
    s.jacket.paint("back", 1, 1, 6, 9, grey)
    s.jacket.paint("back", 2, 2, 4, 1, shade(grey, 0.75))
    s.jacket.paint("back", 2, 4, 4, 1, shade(grey, 0.75))
    s.jacket.dot("back", 6, 8, hexc("#4AE06A"))
    for arm, sleeve in zip(s.arms, s.sleeves):
        arm.all(suit)
        arm.rows(3, 1, orange)
        arm.rows(9, 3, grey)
        sleeve.rows(3, 1, shade(orange, 1.05))
        sleeve.rows(9, 3, grey)
        sleeve.rows(9, 1, shade(grey, 1.2), gradient=False)
    for leg, pant in zip(s.legs, s.pants):
        leg.all(suit)
        leg.rows(4, 1, orange)
        pant.paint("front", 0, 5, 4, 2, grey)
        pant.rows(9, 3, hexc("#5A606A"))
        pant.rows(9, 1, shade(grey, 1.15), gradient=False)
        leg.paint("bottom", 0, 0, 4, 4, hexc("#3A3F48"))
    return s.img


def pirate():
    s = Skin(45, slim=True)
    skin, hair = hexc("#D9A07A"), hexc("#2A1A12")
    red, white, gold = hexc("#B02A2A"), hexc("#F4F0E6"), hexc("#F2C23A")
    s.head.all(skin)
    for face in ("right", "left", "back", "top"):
        w, h = s.head.f[face][2:]
        s.head.paint(face, 0, 0, w, h, hair)
    s.head.paint("front", 0, 0, 8, 3, hair)
    s.head.paint("front", 0, 3, 1, 5, hair)
    s.head.paint("front", 7, 3, 1, 5, hair)
    face_features(s, skin, hexc("#3A6A9A"), shade(hair, 1.1), hexc("#A84A3A"))
    s.head.paint("front", 5, 3, 2, 2, hexc("#141414"), noise=0)
    pattern(s.head, "front", 0, 2, ["K....K.."], {"K": hexc("#141414")}, noise=0)
    # Polka-dot bandana knotted at the back, a gold earring.
    for face in ("top", "right", "left", "back"):
        s.hat.paint(face, 0, 0, s.hat.f[face][2], 3 if face != "top" else 8, red)
    s.hat.paint("front", 0, 0, 8, 2, red)
    for face in ("top", "front", "right", "left", "back"):
        w, h = s.hat.f[face][2:]
        rows = 8 if face == "top" else (2 if face == "front" else 3)
        for j in range(rows):
            for i in range(w):
                if j % 2 == 0 and (i + j) % 4 == 1:
                    s.hat.dot(face, i, j, white)
    s.hat.paint("back", 3, 3, 2, 1, shade(red, 0.85))
    s.hat.paint("back", 2, 4, 1, 3, red)
    s.hat.paint("back", 5, 4, 1, 2, red)
    s.hat.dot("left", 4, 5, gold)
    # Striped shirt, red sash, big buckle.
    navy = hexc("#26365C")
    s.body.all(white)
    for face in SIDES:
        for y in range(0, 12, 2):
            s.body.paint(face, 0, y, s.body.width(face), 1, navy)
    s.jacket.rows(8, 2, red)
    s.jacket.paint("front", 3, 8, 2, 2, gold)
    s.jacket.paint("left", 1, 10, 2, 2, red)
    for arm, sleeve in zip(s.arms, s.sleeves):
        arm.all(white)
        for y in range(0, 9, 2):
            arm.rows(y, 1, navy)
        arm.rows(9, 3, skin)
        arm.paint("bottom", 0, 0, 3, 4, skin)
        sleeve.rows(8, 1, white)
    trousers, boot = hexc("#3A2E28"), hexc("#141414")
    for leg, pant in zip(s.legs, s.pants):
        leg.all(trousers)
        leg.rows(7, 5, boot)
        leg.paint("bottom", 0, 0, 4, 4, boot)
        pant.rows(6, 2, hexc("#2A2A2A"))
        pant.dot("front", 1, 4, shade(trousers, 0.7))
    return s.img


def robot():
    s = Skin(57, slim=False)
    metal, dark, glow = hexc("#9AA4B0"), hexc("#4A525C"), hexc("#6FF2FF")
    s.head.all(metal)
    pattern(s.head, "front", 0, 0, [
        "R......R",
        ".KKKKKK.",
        ".KCKKCK.",
        ".KCKKCK.",
        ".KKKKKK.",
        ".KCKKCK.",
        ".KKCCKK.",
        "R......R",
    ], {"R": shade(metal, 0.7), "K": hexc("#14181C"), "C": glow}, noise=0)
    s.head.paint("top", 0, 0, 8, 8, shade(metal, 1.05))
    s.head.rows(7, 1, dark)
    # Outer layer: ear bolts, an antenna light, a raised screen frame.
    s.hat.paint("right", 2, 2, 4, 4, dark)
    s.hat.paint("right", 3, 3, 2, 2, hexc("#E83A3A"), noise=0)
    s.hat.paint("left", 2, 2, 4, 4, dark)
    s.hat.paint("left", 3, 3, 2, 2, hexc("#E83A3A"), noise=0)
    s.hat.paint("top", 3, 3, 2, 2, hexc("#FFD43A"), noise=0)
    s.hat.paint("front", 0, 0, 8, 1, shade(metal, 1.1))
    s.body.all(metal)
    s.body.rows(11, 1, dark)
    pattern(s.jacket, "front", 1, 1, [
        "KKKKKK",
        "KRKYKG",
        "KKKKKK",
        "......",
        ".DDDD.",
        ".D..D.",
        ".DDDD.",
    ], {"K": hexc("#2A3038"), "R": hexc("#FF4A4A"), "Y": hexc("#FFD43A"), "G": hexc("#4AE06A"), "D": dark}, noise=0)
    for y in (2, 4, 6, 8):
        s.jacket.paint("back", 1, y, 6, 1, dark)
    for arm, sleeve in zip(s.arms, s.sleeves):
        arm.all(metal)
        arm.rows(5, 1, dark)
        arm.rows(10, 2, dark)
        sleeve.rows(0, 3, shade(metal, 1.1))
        sleeve.rows(0, 1, shade(metal, 1.2), gradient=False)
    for leg, pant in zip(s.legs, s.pants):
        leg.all(metal)
        leg.rows(5, 1, dark)
        pant.rows(9, 3, dark)
        pant.rows(9, 1, shade(dark, 1.3), gradient=False)
        leg.paint("bottom", 0, 0, 4, 4, hexc("#2A3038"))
    return s.img


SKINS = {
    "mineur.png": miner,
    "exploratrice.png": explorer,
    "dino.png": dino,
    "astronaute.png": astronaut,
    "pirate.png": pirate,
    "robot.png": robot,
}
for name, draw in SKINS.items():
    draw().save(f"{OUT}/{name}")
print("ok")
