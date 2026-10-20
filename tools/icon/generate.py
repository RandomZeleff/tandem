"""Tandem's app icon, pixel art on a 32-unit grid: two blocks side by side, like the
logo of the title bar (grass in front, gold behind).

    python tools/icon/generate.py src-tauri/icons/icon.svg
    pnpm tauri icon src-tauri/icons/icon.svg

The second command writes every size and format (.ico, .icns, PNG) next to it."""
import random
import sys

out = sys.argv[1]
rng = random.Random(4)
rects = []


def px(x, y, w, h, color):
    rects.append(f'<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="{color}"/>')


def textured(x, y, w, h, base, darker, lighter, density=0.18):
    px(x, y, w, h, base)
    for j in range(h):
        for i in range(w):
            r = rng.random()
            if r < density / 2:
                px(x + i, y + j, 1, 1, darker)
            elif r < density:
                px(x + i, y + j, 1, 1, lighter)


# Gold block, behind and higher: x 14..28, y 6..20.
gx, gy, s = 14, 6, 14
textured(gx, gy + 3, s, s - 3, "#8A5E24", "#6E4A1A", "#9C6C2C")
textured(gx, gy, s, 3, "#F2C744", "#E0AE2E", "#FFE38A", 0.3)
for i in range(0, s, 3):  # drips of gold over the side
    px(gx + i + 1, gy + 3, 1, 1 + (i % 2), "#E0AE2E")
px(gx, gy, s, 1, "#FFE9A3")
px(gx + s - 1, gy, 1, s, "#5C3E14")  # shaded right edge
px(gx, gy + s - 1, s, 1, "#5C3E14")

# Grass block, in front and lower: x 4..18, y 12..26.
x, y = 4, 12
textured(x, y + 3, s, s - 3, "#6B4A2C", "#57391F", "#7E5836")
textured(x, y, s, 3, "#5DBB3F", "#4E9A33", "#7FD14F", 0.35)
for i in (1, 2, 4, 7, 8, 10, 12):  # grass hanging over the dirt
    px(x + i, y + 3, 1, 1 + (i % 3 == 1), "#4E9A33")
px(x, y, s, 1, "#8EE05E")
px(x + s - 1, y, 1, s, "#3E2A18")
px(x, y + s - 1, s, 1, "#3E2A18")
# The grass block's shadow on the gold one, where they touch.
px(x + s, y, 1, gy + s - y, "#3A2810")

svg = f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32" shape-rendering="crispEdges">
<rect x="0" y="0" width="32" height="32" rx="7" fill="#16181C"/>
<rect x="0.5" y="0.5" width="31" height="31" rx="6.5" fill="none" stroke="#2C3038" stroke-width="1"/>
{chr(10).join(rects)}
</svg>
'''
open(out, "w", encoding="utf-8").write(svg)
print("ok", len(rects))
