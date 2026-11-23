#!/usr/bin/env python3
"""Procedural pixel-art sprite generator for Hero Titans (Tiers 17 & 18).

Generates monumental 128x128 RGBA sprites (normal + enraged variants) for:
  - Tier 17: Void Lord (mob_tier17_void_lord[_enraged].png)
  - Tier 18: Inferno Archdemon (mob_tier18_inferno_archdemon[_enraged].png)

All outputs include contact shadows, spatial/thermal lensing, crisp 1px
contrast outlines (#0C0A12), and translucent glow layers, ready for `textures.rs`.

Usage:
    python scripts/generate_titans.py
    python scripts/generate_titans.py --out-root .
"""

from __future__ import annotations

import argparse
import math
from pathlib import Path
from typing import Tuple

from PIL import Image, ImageDraw

RGBA = Tuple[int, int, int, int]

SIZE = 128
OUTLINE_DARK: RGBA = (12, 10, 18, 255)      # #0C0A12


# ---------------------------------------------------------------------------
# Color Palettes (Tiers 17 & 18)
# ---------------------------------------------------------------------------

T17_VOID = {
    "shadow": (0, 0, 0, 160),
    "lens_violet": (114, 9, 183, 105),      # #7209B7 (a=105)
    "lens_aqua": (76, 201, 240, 190),       # #4CC9F0
    "wing_void": (8, 4, 15, 255),           # #08040F
    "wing_mid": (22, 11, 38, 255),          # #160B26
    "uv_rib": (114, 9, 183, 255),           # #7209B7
    "aqua": (76, 201, 240, 255),            # #4CC9F0
    "magenta": (247, 37, 133, 255),         # #F72585
    "glyph": (224, 251, 252, 255),          # #E0FBFC
    "stone": (24, 16, 42, 255),
    "stone_hi": (66, 52, 102, 255),
    "mask": (210, 214, 228, 255),           # #D2D6E4
    "mask_shadow": (142, 146, 164, 255),    # #8E92A4
    "black_hole": (0, 0, 0, 255),
    "white": (255, 255, 255, 255),
    "glow_aqua": (76, 201, 240, 85),
    "glow_magenta": (247, 37, 133, 80),
}

T18_INFERNO = {
    "shadow": (0, 0, 0, 160),
    "thermal_glow": (255, 60, 0, 95),       # rgba(255, 60, 0, 95)
    "thermal_rage": (255, 80, 0, 130),
    "wing_dark": (52, 12, 18, 255),         # #340C12
    "wing_mid": (68, 14, 22, 255),          # #440E16
    "obs": (22, 14, 20, 255),               # #160E14
    "obs_hi": (78, 54, 70, 255),            # #4E3646
    "lava_deep": (190, 18, 0, 255),         # #BE1200
    "lava_red": (217, 4, 41, 255),          # #D90429
    "lava_orange": (255, 76, 0, 255),       # #FF4C00
    "lava_gold": (255, 175, 0, 255),        # #FFAF00
    "lava_core": (255, 248, 210, 255),      # #FFF8D2
    "gold_hi": (212, 168, 55, 255),         # #D4A837
    "gold_dark": (155, 115, 38, 255),       # #9B7326
    "bone": (198, 184, 162, 255),           # #C6B8A2
    "bone_shadow": (122, 108, 92, 255),
    "fang": (255, 255, 255, 255),
    "aura_fire": (255, 76, 0, 65),
    "aura_wing": (255, 110, 0, 95),
    "aura_gold": (255, 175, 0, 60),
}


# ---------------------------------------------------------------------------
# Shared Pixel-Art Helpers
# ---------------------------------------------------------------------------

def new_canvas(w: int = SIZE, h: int = SIZE) -> Image.Image:
    return Image.new("RGBA", (w, h), (0, 0, 0, 0))


def put_px(img: Image.Image, x: int, y: int, color: RGBA) -> None:
    if 0 <= x < img.width and 0 <= y < img.height:
        img.putpixel((x, y), color)


def apply_1px_outline(layer: Image.Image, outline_color: RGBA = OUTLINE_DARK) -> Image.Image:
    """Apply a crisp 4-connected 1px contrast outline around opaque pixels."""
    w, h = layer.size
    src = layer.load()
    out = layer.copy()
    dst = out.load()

    for y in range(h):
        for x in range(w):
            if src[x, y][3] > 0:
                continue
            for nx, ny in ((x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)):
                if 0 <= nx < w and 0 <= ny < h and src[nx, ny][3] >= 180:
                    dst[x, y] = outline_color
                    break
    return out


def compose(
    shadow: Image.Image,
    body: Image.Image,
    glow: Image.Image,
    top_overlay: Image.Image | None = None,
    outline_color: RGBA = OUTLINE_DARK,
) -> Image.Image:
    outlined = apply_1px_outline(body, outline_color)
    img = Image.alpha_composite(shadow, outlined)
    img = Image.alpha_composite(img, glow)
    if top_overlay is not None:
        img = Image.alpha_composite(img, top_overlay)
    return img


# ---------------------------------------------------------------------------
# Tier 17: Void Lord (128x128, Normal & Enraged)
# ---------------------------------------------------------------------------

def draw_tier17_void_lord(enraged: bool = False) -> Image.Image:
    p = T17_VOID
    shadow = new_canvas()
    body = new_canvas()
    glow = new_canvas()
    top = new_canvas()

    d_sh = ImageDraw.Draw(shadow)
    d = ImageDraw.Draw(body)
    d_gl = ImageDraw.Draw(glow)
    d_top = ImageDraw.Draw(top)

    ext = 8 if enraged else 0

    # Layer 0: Contact Shadow & Gravitational Lensing Halo
    d_sh.ellipse([(20, 108), (108, 124)], fill=p["shadow"])
    d_gl.ellipse([(14, 104), (114, 126)], outline=p["lens_violet"], width=3)
    d_gl.ellipse([(22, 108), (106, 123)], outline=p["lens_aqua"], width=1)

    # Layer 1: Six Astral Wings
    lx_top = max(2, 24 - ext)
    rx_top = min(125, 104 + ext)
    upper_l = [(64, 48), (lx_top, 16), (lx_top - 4 if enraged else 18, 34), (44, 58)]
    upper_r = [(64, 48), (rx_top, 16), (rx_top + 4 if enraged else 110, 34), (84, 58)]
    for poly, outer_edge, rib_a, rib_b in (
        (upper_l, [(64, 48), (lx_top, 16), (upper_l[2])], (56, 46), (lx_top + 4, 24)),
        (upper_r, [(64, 48), (rx_top, 16), (upper_r[2])], (72, 46), (rx_top - 4, 24)),
    ):
        d.polygon(poly, fill=p["wing_void"])
        d.line([rib_a, rib_b], fill=p["uv_rib"], width=2)
        d.line([(poly[0][0], poly[0][1] + 4), (poly[2][0], poly[2][1])], fill=p["uv_rib"], width=2)
        d.line(outer_edge, fill=p["aqua"], width=2)
        if enraged:
            d_gl.line(outer_edge, fill=p["glow_aqua"], width=5)

    mid_l = [(54, 56), (max(2, 12 - ext // 2), 48), (max(1, 4 - ext // 3), 64), (42, 74)]
    mid_r = [(74, 56), (min(125, 116 + ext // 2), 48), (min(126, 124 + ext // 3), 64), (86, 74)]
    for poly, vein in (
        (mid_l, [(48, 60), (14, 54), (mid_l[2])]),
        (mid_r, [(80, 60), (114, 54), (mid_r[2])]),
    ):
        d.polygon(poly, fill=p["wing_mid"])
        d.line(vein, fill=p["magenta"], width=2)
        d.line([poly[1], poly[2]], fill=p["aqua"], width=1)
        if enraged:
            d_gl.line(vein, fill=p["glow_magenta"], width=4)

    # Lower smoky mantle
    d.polygon([(44, 68), (84, 68), (94, 110), (34, 110)], fill=p["wing_void"])
    d.polygon([(48, 70), (80, 70), (86, 108), (42, 108)], fill=p["wing_mid"])
    d.line([(44, 68), (34, 108)], fill=p["uv_rib"], width=2)
    d.line([(84, 68), (94, 108)], fill=p["uv_rib"], width=2)
    for x in range(34, 95):
        for y in range(100, 111):
            if y >= 106 and (x + y) % 2 == 0:
                put_px(body, x, y, (0, 0, 0, 0))
            elif y >= 102 and (x * 3 + y) % 5 == 0:
                put_px(body, x, y, (0, 0, 0, 0))

    # Astral torso
    d.polygon([(44, 42), (84, 42), (78, 82), (50, 82)], fill=p["stone"])
    d.line([(44, 42), (84, 42)], fill=p["stone_hi"], width=2)
    for ry in (48, 56, 68, 76):
        d.line([(46, ry), (54, ry + 2)], fill=p["uv_rib"], width=2)
        d.line([(82, ry), (74, ry + 2)], fill=p["uv_rib"], width=2)

    # Floating Monolith Pauldrons
    for x0, x1 in ((28, 44), (84, 100)):
        d.polygon([(x0 + 4, 34), (x1 - 4, 34), (x1, 44), (x1 - 3, 54), (x0 + 3, 54), (x0, 44)], fill=p["stone"])
        d.line([(x0 + 4, 34), (x1 - 4, 34)], fill=p["stone_hi"], width=2)
        gx = (x0 + x1) // 2
        d.line([(gx, 38), (gx, 50)], fill=p["glyph"], width=1)
        d.line([(gx - 3, 42), (gx + 3, 42)], fill=p["aqua"], width=1)
        d.line([(gx - 2, 47), (gx + 2, 47)], fill=p["aqua"], width=1)
        d_gl.ellipse([(gx - 5, 38), (gx + 5, 50)], fill=p["glow_aqua"])

    # Floating Claws
    for gx0, gx1, sign in ((16, 32, -1), (96, 112, 1)):
        d.rectangle([(gx0, 68), (gx1, 79)], fill=p["stone"])
        d.line([(gx0, 68), (gx1, 68)], fill=p["stone_hi"], width=2)
        d.line([(gx0 + 3, 73), (gx1 - 3, 73)], fill=p["uv_rib"], width=2)
        for cx in (gx0 + 3, (gx0 + gx1) // 2, gx1 - 3):
            d.line([(cx, 79), (cx + sign * 2, 84), (cx - sign, 88)], fill=p["aqua"], width=2)
            put_px(body, cx - sign, 88, p["glyph"])

    # Layer 2: Chest Singularity (center 64, 62)
    cx, cy = 64, 62
    disk_r = 18 if enraged else 14

    if enraged:
        for i in range(8):
            angle = i * (math.pi / 4.0)
            x1 = int(cx + math.cos(angle) * 14)
            y1 = int(cy + math.sin(angle) * 14)
            xm = int(cx + math.cos(angle + 0.14) * 28)
            ym = int(cy + math.sin(angle + 0.14) * 28)
            x2 = int(cx + math.cos(angle - 0.06) * 44)
            y2 = int(cy + math.sin(angle - 0.06) * 44)
            col = p["aqua"] if i % 2 == 0 else p["magenta"]
            d_gl.line([(x1, y1), (xm, ym), (x2, y2)], fill=col, width=3)
            d_gl.line([(x1, y1), (xm, ym), (x2, y2)], fill=p["glyph"], width=1)

    d_gl.ellipse([(cx - disk_r - 6, cy - disk_r // 2 - 4), (cx + disk_r + 6, cy + disk_r // 2 + 4)], fill=p["glow_magenta"])
    d_gl.ellipse([(cx - disk_r - 2, cy - disk_r + 2), (cx + disk_r + 2, cy + disk_r - 2)], outline=p["glow_aqua"], width=4)

    eh_r = 10
    d_top.ellipse([(cx - eh_r - 2, cy - eh_r - 2), (cx + eh_r + 2, cy + eh_r + 2)], fill=p["aqua"])
    d_top.ellipse([(cx - eh_r, cy - eh_r), (cx + eh_r, cy + eh_r)], fill=p["black_hole"])
    d_top.arc([(cx - 10, cy - 10), (cx + 10, cy + 10)], start=205, end=335, fill=p["white"], width=2)
    d_top.arc([(cx - 11, cy - 11), (cx + 11, cy + 11)], start=30, end=140, fill=p["magenta"], width=1)
    d_top.line([(cx - disk_r - 10, cy), (cx + disk_r + 10, cy)], fill=p["aqua"], width=2)
    d_top.line([(cx - disk_r - 6, cy - 1), (cx + disk_r + 6, cy - 1)], fill=p["glyph"], width=1)

    # Layer 3: Death Mask & 7-Peak Crown
    mask_poly = [(55, 24), (73, 24), (74, 36), (68, 46), (60, 46), (54, 36)]
    d.polygon(mask_poly, fill=p["mask"])
    d.polygon([(56, 38), (72, 38), (68, 46), (60, 46)], fill=p["mask_shadow"])
    d.rectangle([(57, 30), (62, 35)], fill=p["black_hole"])
    d.rectangle([(66, 30), (71, 35)], fill=p["black_hole"])
    for ex, ey in ((59, 32), (69, 32)):
        d.rectangle([(ex, ey), (ex + 1, ey + 1)], fill=p["glyph"])
        d_gl.line([(ex - 3, ey), (ex + 4, ey)], fill=p["glyph"], width=1)
        d_gl.line([(ex, ey - 3), (ex, ey + 4)], fill=p["aqua"], width=1)

    spires = [
        (46, 17, 50, 26),
        (52, 13, 55, 25),
        (58, 9, 60, 24),
        (64, 4, 64, 24),
        (70, 9, 68, 24),
        (76, 13, 73, 25),
        (82, 17, 78, 26),
    ]
    for tip_x, tip_y, base_x, base_y in spires:
        d.polygon([(tip_x, tip_y), (base_x - 3, base_y), (base_x + 3, base_y)], fill=p["wing_void"])
        d.line([(tip_x, tip_y), (base_x, base_y)], fill=p["stone_hi"], width=1)
        d.polygon([(tip_x, tip_y - 2), (tip_x + 2, tip_y + 1), (tip_x, tip_y + 3), (tip_x - 2, tip_y + 1)], fill=p["aqua"])
        put_px(body, tip_x, tip_y, p["glyph"])
        if enraged:
            d_gl.ellipse([(tip_x - 3, tip_y - 3), (tip_x + 3, tip_y + 3)], fill=p["glow_aqua"])

    return compose(shadow, body, glow, top_overlay=top)


# ---------------------------------------------------------------------------
# Tier 18: Inferno Archdemon (128x128, Normal & Enraged)
# ---------------------------------------------------------------------------

def draw_tier18_inferno_archdemon(enraged: bool = False) -> Image.Image:
    p = T18_INFERNO
    shadow = new_canvas()
    body = new_canvas()
    glow = new_canvas()

    d_sh = ImageDraw.Draw(shadow)
    d = ImageDraw.Draw(body)
    d_gl = ImageDraw.Draw(glow)

    d_gl.ellipse([(10, 106), (118, 127)], fill=p["thermal_rage"] if enraged else p["thermal_glow"])
    d_sh.ellipse([(18, 110), (110, 125)], fill=p["shadow"])
    d_gl.ellipse([(24, 112), (104, 123)], outline=p["lava_orange"], width=2)

    left_wing = [(48, 46), (14, 18), (2, 46), (10, 74), (28, 66), (44, 76)]
    right_wing = [(80, 46), (114, 18), (126, 46), (118, 74), (100, 66), (84, 76)]

    if enraged:
        for poly in (left_wing, right_wing):
            d_sh.line(poly + [poly[0]], fill=p["aura_wing"], width=8)
            d_sh.line(poly + [poly[0]], fill=p["lava_gold"], width=3)

    for poly, spars, veins in (
        (
            left_wing,
            [((48, 46), (14, 18)), ((44, 52), (2, 46)), ((44, 58), (10, 74)), ((44, 62), (28, 66))],
            [((38, 48), (22, 34), (10, 44)), ((36, 56), (20, 58), (14, 68))],
        ),
        (
            right_wing,
            [((80, 46), (114, 18)), ((84, 52), (126, 46)), ((84, 58), (118, 74)), ((84, 62), (100, 66))],
            [((90, 48), (106, 34), (118, 44)), ((92, 56), (108, 58), (114, 68))],
        ),
    ):
        d.polygon(poly, fill=p["wing_dark"])
        cx_w = sum(pt[0] for pt in poly) // len(poly)
        cy_w = sum(pt[1] for pt in poly) // len(poly)
        d.polygon([poly[0], poly[1], (cx_w, cy_w), poly[4]], fill=p["wing_mid"])
        for v in veins:
            d.line(v, fill=p["lava_red"], width=3)
            d.line(v, fill=p["lava_orange"], width=1)
        for a, b in spars:
            d.line([a, b], fill=p["obs"], width=4)
            d.line([a, b], fill=p["obs_hi"], width=1)

    tail_pts = [(48, 86), (34, 96), (20, 96), (14, 86)]
    d.line(tail_pts, fill=p["obs"], width=5)
    d.line(tail_pts, fill=p["obs_hi"], width=1)
    for sx, sy in ((34, 93), (24, 93), (17, 90)):
        d.polygon([(sx, sy), (sx - 2, sy - 4), (sx + 2, sy)], fill=p["bone"])
    d.polygon([(14, 86), (6, 80), (10, 74), (18, 80)], fill=p["lava_orange"])
    d.polygon([(13, 84), (8, 79), (11, 76), (16, 81)], fill=p["lava_gold"])
    put_px(body, 10, 77, p["lava_core"])
    d_gl.ellipse([(5, 73), (19, 87)], fill=p["aura_fire"])

    for lx0, lx1, hx0, hx1 in ((40, 54, 36, 52), (74, 88, 76, 92)):
        d.rectangle([(lx0, 78), (lx1, 105)], fill=p["obs"])
        d.rectangle([(lx0 + 2, 82), (lx1 - 2, 96)], fill=p["obs_hi"])
        d.line([(lx0, 82), (lx1, 82)], fill=p["gold_hi"], width=2)
        d.line([(lx0, 96), (lx1, 96)], fill=p["gold_dark"], width=2)
        d.polygon([(hx0, 114), (hx0 + 3, 104), (hx1 - 3, 104), (hx1, 114)], fill=p["obs"])
        d.line([(hx0, 114), (hx1, 114)], fill=p["lava_orange"], width=2)
        d.line([(hx0 + 4, 112), (hx1 - 4, 112)], fill=p["lava_gold"], width=1)

    d.polygon([(40, 42), (88, 42), (82, 82), (46, 82)], fill=p["obs"])
    d.line([(40, 42), (88, 42)], fill=p["gold_hi"], width=2)
    d.line([(46, 82), (82, 82)], fill=p["gold_dark"], width=3)
    d.line([(42, 44), (47, 80)], fill=p["gold_dark"], width=2)
    d.line([(86, 44), (81, 80)], fill=p["gold_dark"], width=2)
    d.rectangle([(45, 74), (83, 79)], fill=p["gold_dark"])
    d.rectangle([(59, 73), (69, 80)], fill=p["gold_hi"])
    d.rectangle([(62, 75), (66, 78)], fill=p["lava_orange"])

    d_gl.ellipse([(49, 45), (79, 71)], fill=p["aura_fire"])
    d.rectangle([(54, 50), (74, 66)], fill=p["lava_deep"])
    d.rectangle([(56, 52), (72, 64)], fill=p["lava_orange"])
    d.rectangle([(59, 55), (69, 62)], fill=p["lava_gold"])
    d.rectangle([(61, 57), (67, 60)], fill=p["lava_core"])
    if enraged:
        d_gl.ellipse([(51, 47), (77, 69)], fill=p["aura_gold"])
    d.rectangle([(54, 50), (74, 66)], outline=p["gold_hi"], width=2)
    for gx in (59, 64, 69):
        d.line([(gx, 50), (gx, 66)], fill=p["obs"], width=2)
    d.line([(54, 58), (74, 58)], fill=p["obs"], width=2)

    d.polygon([(30, 52), (44, 52), (40, 78), (28, 74)], fill=p["obs"])
    d.line([(30, 52), (28, 74)], fill=p["obs_hi"], width=2)
    d.rectangle([(27, 72), (39, 80)], fill=p["gold_dark"])
    d.polygon([(84, 52), (98, 56), (104, 74), (92, 76)], fill=p["obs"])
    d.rectangle([(94, 68), (106, 78)], fill=p["gold_dark"])

    for x0, x1, sign in ((28, 46, -1), (82, 100, 1)):
        d.polygon([(x0, 44), (x0 + 4, 36), (x1 - 4, 36), (x1, 44), (x1 - 2, 54), (x0 + 2, 54)], fill=p["obs"])
        d.line([(x0 + 4, 36), (x1 - 4, 36)], fill=p["gold_hi"], width=2)
        tx = x0 + 6 if sign < 0 else x1 - 6
        d.polygon([(tx - 3, 38), (tx + 3, 38), (tx + sign * 10, 22)], fill=p["bone"])
        d.line([(tx, 38), (tx + sign * 10, 22)], fill=p["bone_shadow"], width=1)

    bx = 102
    d_gl.polygon([(bx, 5), (bx + 12, 18), (bx + 15, 69), (bx - 11, 69), (bx - 11, 18)], fill=p["aura_fire"])
    d.polygon([(bx, 10), (bx + 8, 20), (bx + 8, 70), (bx - 8, 70), (bx - 8, 20)], fill=p["obs"])
    d.line([(bx - 8, 20), (bx - 8, 70)], fill=p["obs_hi"], width=1)
    for sy in range(22, 66, 8):
        d.polygon([(bx + 8, sy), (bx + 14, sy + 3), (bx + 8, sy + 6)], fill=p["lava_orange"])
        put_px(body, bx + 13, sy + 3, p["lava_core"])
        if enraged:
            d_gl.line([(bx + 12, sy + 3), (bx + 19, sy - 2)], fill=p["lava_gold"], width=2)
    d.polygon([(bx, 14), (bx + 4, 22), (bx + 4, 68), (bx - 4, 68), (bx - 4, 22)], fill=p["lava_orange"])
    d.line([(bx, 16), (bx, 67)], fill=p["lava_gold"], width=3)
    d.line([(bx, 18), (bx, 66)], fill=p["lava_core"], width=1)
    d.polygon([(bx - 14, 68), (bx + 14, 68), (bx + 11, 73), (bx - 11, 73)], fill=p["gold_hi"])
    d.rectangle([(bx - 3, 73), (bx + 3, 82)], fill=p["wing_dark"])
    d.ellipse([(bx - 4, 80), (bx + 4, 86)], fill=p["lava_gold"])

    left_horn = [(56, 28), (44, 24), (36, 15), (32, 4), (39, 10), (48, 19), (58, 24)]
    right_horn = [(72, 28), (84, 24), (92, 15), (96, 4), (89, 10), (80, 19), (70, 24)]
    for horn, tip in ((left_horn, (32, 4)), (right_horn, (96, 4))):
        d.polygon(horn, fill=p["bone"])
        d.line([horn[0], horn[1], horn[2], tip], fill=p["obs"], width=2)
        d.line([horn[6], horn[5], horn[4], tip], fill=p["bone_shadow"], width=1)

    d.polygon([(55, 29), (49, 27), (54, 32)], fill=p["bone"])
    d.polygon([(73, 29), (79, 27), (74, 32)], fill=p["bone"])

    d.polygon([(56, 26), (72, 26), (74, 37), (70, 44), (58, 44), (54, 37)], fill=p["obs"])
    d.line([(57, 26), (71, 26)], fill=p["obs_hi"], width=2)
    d.rectangle([(57, 30), (62, 33)], fill=p["lava_red"])
    d.rectangle([(66, 30), (71, 33)], fill=p["lava_red"])
    d.rectangle([(59, 31), (61, 32)], fill=p["lava_core"])
    d.rectangle([(67, 31), (69, 32)], fill=p["lava_core"])
    if enraged:
        d_gl.line([(55, 31), (63, 31)], fill=p["lava_gold"], width=2)
        d_gl.line([(65, 31), (73, 31)], fill=p["lava_gold"], width=2)
    d.rectangle([(58, 36), (70, 42)], fill=p["lava_deep"])
    d.rectangle([(60, 38), (68, 41)], fill=p["lava_orange"])
    for fx in (59, 62, 66, 69):
        d.line([(fx, 36), (fx, 38)], fill=p["fang"])
        put_px(body, fx, 42, p["fang"])

    return compose(shadow, body, glow)


# ---------------------------------------------------------------------------
# Batch Generation Pipeline
# ---------------------------------------------------------------------------

GENERATORS = [
    ("mob_tier17_void_lord.png", lambda: draw_tier17_void_lord(enraged=False)),
    ("mob_tier17_void_lord_enraged.png", lambda: draw_tier17_void_lord(enraged=True)),
    ("mob_tier18_inferno_archdemon.png", lambda: draw_tier18_inferno_archdemon(enraged=False)),
    ("mob_tier18_inferno_archdemon_enraged.png", lambda: draw_tier18_inferno_archdemon(enraged=True)),
]


def generate_all(out_root: Path) -> list[Path]:
    mobs_dir = out_root / "assets" / "mobs"
    sprites_dir = out_root / "assets" / "sprites"
    titans_dir = out_root / "assets" / "titans"
    out_dirs = [mobs_dir, sprites_dir, titans_dir]
    for d in out_dirs:
        d.mkdir(parents=True, exist_ok=True)

    saved: list[Path] = []
    for filename, fn in GENERATORS:
        img = fn()
        for d in out_dirs:
            path = d / filename
            img.save(path, format="PNG")
            saved.append(path)
    return saved


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Generate Hero Titans (Tiers 17 & 18, 128x128 RGBA) sprites."
    )
    parser.add_argument(
        "--out-root",
        type=Path,
        default=Path("."),
        help="Project root directory where assets/mobs/, assets/titans/ and assets/sprites/ will be written.",
    )
    args = parser.parse_args()

    files = generate_all(args.out_root)
    print(f"Generated {len(files)} PNG files under {(args.out_root / 'assets').resolve()}")


if __name__ == "__main__":
    main()
