#!/usr/bin/env python3
"""Procedural pixel-art generator for Dungeon Floor Tiles & Perimeter Walls (Group 6).

Generates 8 seamless 64x64 RGBA tiles in Dark Fantasy Gothic Pixel Art style,
strictly adhering to the Tri-Layer Luminance budget (floor brightness 12%–22%)
for crisp gameplay readability across 3000x3000 arenas:
  - floor_dungeon_plain.png    (64x64)
  - floor_dungeon_cracked.png  (64x64)
  - floor_dungeon_blood.png    (64x64)
  - floor_dungeon_runic.png    (64x64)
  - floor_stone.png            (64x64)
  - floor_dark.png             (64x64)
  - wall_brick.png             (64x64)
  - wall_banner.png            (64x64)

Outputs are saved to `assets/tiles/` and mirrored to `assets/sprites/`.

Usage:
    python scripts/generate_tiles.py
    python scripts/generate_tiles.py --out-root .
"""

from __future__ import annotations

import argparse
import math
import random
from pathlib import Path
from typing import Tuple

from PIL import Image, ImageDraw

RGBA = Tuple[int, int, int, int]

TILE_SIZE = 64


# ---------------------------------------------------------------------------
# Color Palette (Tri-Layer Luminance: Dark Basalt Floor & Gothic Masonry)
# ---------------------------------------------------------------------------

FLOOR_BASALT = [
    (20, 17, 29, 255),      # #14111D
    (23, 19, 33, 255),      # #171321
    (26, 22, 37, 255),      # #1A1625
    (21, 18, 31, 255),      # #15121F
]
FLOOR_SPECKS = [
    (29, 24, 41, 255),      # #1D1829
    (34, 28, 48, 255),      # #221C30
    (13, 11, 20, 255),      # #0D0B14
]
BEVEL_LIGHT: RGBA = (40, 34, 56, 255)       # #282238
BEVEL_BRIGHT: RGBA = (44, 38, 61, 255)      # #2C263D
BEVEL_DARK: RGBA = (12, 10, 19, 255)        # #0C0A13
MORTAR_DEEP: RGBA = (7, 5, 10, 255)         # #07050A
MORTAR_MID: RGBA = (10, 8, 16, 255)         # #0A0810

CRACK_CORE: RGBA = (5, 4, 8, 255)           # #050408
CRACK_SHADOW: RGBA = (10, 8, 18, 255)       # #0A0812
CRACK_CHIP: RGBA = (50, 42, 68, 255)        # #322A44
CRACK_DEBRIS: RGBA = (38, 32, 53, 255)      # #262035

BLOOD_CRUST: RGBA = (32, 3, 8, 255)         # #200308
BLOOD_DARK: RGBA = (69, 7, 19, 255)         # #450713
BLOOD_CORE: RGBA = (107, 15, 35, 255)       # #6B0F23
BLOOD_FRESH: RGBA = (148, 24, 48, 255)      # #941830

RUNE_GROOVE: RGBA = (43, 14, 68, 255)       # #2B0E44
RUNE_MID: RGBA = (91, 22, 140, 255)         # #5B168C
RUNE_NEON: RGBA = (139, 47, 201, 255)       # #8B2FC9
RUNE_SPARK: RGBA = (192, 132, 252, 255)     # #C084FC
RUNE_AURA: RGBA = (59, 22, 90, 102)         # #3B165A (~40% alpha)

WALL_DARK: RGBA = (24, 21, 32, 255)         # #181520
WALL_BASE: RGBA = (36, 32, 48, 255)         # #242030
WALL_MID: RGBA = (43, 38, 56, 255)          # #2B2638
WALL_HI: RGBA = (54, 48, 70, 255)           # #363046
WALL_CAP: RGBA = (70, 62, 90, 255)          # #463E5A
WALL_SHADOW: RGBA = (19, 16, 28, 255)       # #13101C
WALL_DROP: RGBA = (12, 10, 20, 255)         # #0C0A14
WALL_FLOOR: RGBA = (8, 6, 13, 255)          # #08060D
WALL_MOSS: RGBA = (18, 24, 21, 255)         # #121815

BANNER_IRON: RGBA = (58, 52, 68, 255)       # #3A3444
BANNER_RIVET: RGBA = (110, 102, 126, 255)
BANNER_DARK: RGBA = (53, 11, 20, 255)       # #350B14
BANNER_BASE: RGBA = (96, 20, 35, 255)       # #601423
BANNER_HI: RGBA = (133, 29, 49, 255)        # #851D31
BANNER_BRIGHT: RGBA = (156, 43, 63, 255)    # #9C2B3F
BANNER_GOLD_DARK: RGBA = (138, 114, 61, 255)  # #8A723D
BANNER_GOLD: RGBA = (184, 156, 84, 255)     # #B89C54
BANNER_GOLD_HI: RGBA = (191, 161, 95, 255)  # #BFA15F


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def new_tile() -> Image.Image:
    return Image.new("RGBA", (TILE_SIZE, TILE_SIZE), MORTAR_DEEP)


def put_px(img: Image.Image, x: int, y: int, color: RGBA) -> None:
    if 0 <= x < img.width and 0 <= y < img.height:
        img.putpixel((x, y), color)


def put_wrap(img: Image.Image, x: int, y: int, color: RGBA) -> None:
    img.putpixel((x % img.width, y % img.height), color)


# ---------------------------------------------------------------------------
# 1. floor_dungeon_plain.png (64x64)
# ---------------------------------------------------------------------------

def draw_floor_dungeon_plain(seed: int = 101) -> Image.Image:
    rng = random.Random(seed)
    img = new_tile()
    d = ImageDraw.Draw(img)

    # Outer mortar frame (x=0,63; y=0,63) & center cross (x=31..32; y=31..32)
    # 4 modular slabs: 30x30 each
    slabs = [
        (1, 1, 30, 30, FLOOR_BASALT[0]),
        (33, 1, 62, 30, FLOOR_BASALT[1]),
        (1, 33, 30, 62, FLOOR_BASALT[2]),
        (33, 33, 62, 62, FLOOR_BASALT[3]),
    ]

    for x0, y0, x1, y1, base_col in slabs:
        d.rectangle([(x0, y0), (x1, y1)], fill=base_col)

        # Procedural granite micro-specks
        for y in range(y0 + 1, y1):
            for x in range(x0 + 1, x1):
                r = rng.random()
                if r < 0.08:
                    put_px(img, x, y, rng.choice(FLOOR_SPECKS))

        # Top & left 1px light bevel (#282238)
        d.line([(x0, y0), (x1 - 1, y0)], fill=BEVEL_LIGHT)
        d.line([(x0, y0), (x0, y1 - 1)], fill=BEVEL_LIGHT)
        put_px(img, x0, y0, BEVEL_BRIGHT)

        # Bottom & right 1px shadow bevel (#0C0A13)
        d.line([(x0 + 1, y1), (x1, y1)], fill=BEVEL_DARK)
        d.line([(x1, y0 + 1), (x1, y1)], fill=BEVEL_DARK)

    # Subtle mortar texture variation
    for pos in range(TILE_SIZE):
        if rng.random() < 0.35:
            put_px(img, 31, pos, MORTAR_MID)
            put_px(img, pos, 32, MORTAR_MID)

    # Small 2x2 corner stone chips on 3 corners
    chips = [(29, 29), (33, 29), (29, 33)]
    for cx, cy in chips:
        d.rectangle([(cx, cy), (cx + 1, cy + 1)], fill=MORTAR_DEEP)
        put_px(img, cx - 1 if cx < 32 else cx + 2, cy, BEVEL_DARK)

    return img


# ---------------------------------------------------------------------------
# 2. floor_dungeon_cracked.png (64x64)
# ---------------------------------------------------------------------------

def draw_floor_dungeon_cracked() -> Image.Image:
    rng = random.Random(202)
    img = draw_floor_dungeon_plain(seed=101)
    d = ImageDraw.Draw(img)

    # Main diagonal fracture from (14, 8) through (30, 34) to (52, 58)
    main_pts = [
        (14, 8), (18, 14), (21, 17), (25, 24), (28, 28),
        (30, 34), (35, 38), (39, 44), (44, 48), (48, 54), (52, 58),
    ]
    branches = [
        [(25, 24), (20, 26), (15, 29)],
        [(35, 38), (42, 36), (48, 34)],
        [(39, 44), (34, 49), (31, 55)],
    ]

    # Shadow & light chipped rim along fractures
    for path in [main_pts] + branches:
        for (x0, y0), (x1, y1) in zip(path[:-1], path[1:]):
            d.line([(x0 - 1, y0 + 1), (x1 - 1, y1 + 1)], fill=CRACK_SHADOW, width=1)
            d.line([(x0 + 1, y0 - 1), (x1 + 1, y1 - 1)], fill=CRACK_CHIP, width=1)

    # Deep Abyssal black fracture core
    d.line(main_pts, fill=CRACK_CORE, width=2)
    for br in branches:
        d.line(br, fill=CRACK_CORE, width=1)

    # Scattered stone debris flecks (1x1 and 2x1) around the fracture
    debris_coords = [(12, 13), (23, 15), (17, 24), (34, 29), (44, 41), (36, 52), (50, 49), (43, 56)]
    for dx, dy in debris_coords:
        put_px(img, dx, dy, CRACK_DEBRIS)
        if rng.random() < 0.5:
            put_px(img, dx + 1, dy, CRACK_CHIP)

    return img


# ---------------------------------------------------------------------------
# 3. floor_dungeon_blood.png (64x64)
# ---------------------------------------------------------------------------

def draw_floor_dungeon_blood() -> Image.Image:
    rng = random.Random(303)
    img = draw_floor_dungeon_plain(seed=101)
    px = img.load()

    cx, cy = 31.5, 32.5
    # Organic multi-lobe blood pool centered around mortar intersection
    for y in range(12, 54):
        for x in range(12, 54):
            dx = x - cx
            dy = y - cy
            angle = math.atan2(dy, dx)
            dist = math.hypot(dx, dy)
            lobe = 13.5 + 3.2 * math.sin(angle * 3.0 + 0.5) + 1.8 * math.cos(angle * 5.0 - 0.8)
            if dist <= lobe:
                in_seam = (x in (31, 32) or y in (31, 32))
                if in_seam:
                    px[x, y] = BLOOD_CRUST
                elif dist > lobe - 2.2:
                    px[x, y] = BLOOD_CRUST
                elif dist > lobe - 5.5:
                    px[x, y] = BLOOD_DARK
                else:
                    px[x, y] = BLOOD_CORE

    # Blood seeping further down the mortar seams
    for offset in range(-22, 23):
        if rng.random() < 0.85:
            put_px(img, 31, 32 + offset, BLOOD_CRUST)
            put_px(img, 32, 32 + offset, BLOOD_DARK if abs(offset) < 12 else BLOOD_CRUST)
        if rng.random() < 0.85:
            put_px(img, 31 + offset, 31, BLOOD_CRUST)
            put_px(img, 31 + offset, 32, BLOOD_DARK if abs(offset) < 12 else BLOOD_CRUST)

    # Fresh crimson wet glints (4 specular dots)
    for gx, gy in ((27, 28), (35, 30), (29, 36), (34, 35)):
        put_px(img, gx, gy, BLOOD_FRESH)

    # Radial splatter droplets (10 distinct drops 1–3px)
    splatters = [
        (14, 21, 2), (18, 14, 1), (46, 17, 2), (51, 26, 1), (49, 43, 2),
        (43, 51, 1), (19, 48, 2), (13, 39, 1), (24, 12, 1), (53, 36, 1),
    ]
    d = ImageDraw.Draw(img)
    for sx, sy, sr in splatters:
        if sr == 1:
            put_px(img, sx, sy, BLOOD_DARK)
        else:
            d.ellipse([(sx - 1, sy - 1), (sx + 1, sy + 1)], fill=BLOOD_DARK)
            put_px(img, sx, sy, BLOOD_CORE)

    return img


# ---------------------------------------------------------------------------
# 4. floor_dungeon_runic.png (64x64)
# ---------------------------------------------------------------------------

def draw_floor_dungeon_runic() -> Image.Image:
    base = draw_floor_dungeon_plain(seed=101)
    glow = Image.new("RGBA", (TILE_SIZE, TILE_SIZE), (0, 0, 0, 0))
    rune = Image.new("RGBA", (TILE_SIZE, TILE_SIZE), (0, 0, 0, 0))
    d_gl = ImageDraw.Draw(glow)
    d = ImageDraw.Draw(rune)

    cx, cy = 32, 32
    r_outer = 19

    # Soft translucent violet stone aura (~40% alpha)
    d_gl.ellipse([(cx - r_outer - 2, cy - r_outer - 2), (cx + r_outer + 2, cy + r_outer + 2)], outline=RUNE_AURA, width=5)
    d_gl.ellipse([(cx - 11, cy - 11), (cx + 11, cy + 11)], outline=RUNE_AURA, width=3)

    # Carved outer seal circle (diameter ~38px)
    d.ellipse([(cx - r_outer, cy - r_outer), (cx + r_outer, cy + r_outer)], outline=RUNE_GROOVE, width=3)
    d.ellipse([(cx - r_outer, cy - r_outer), (cx + r_outer, cy + r_outer)], outline=RUNE_NEON, width=1)

    # 8-pointed Abyssal star rune inside circle
    star_pts_1 = []
    star_pts_2 = []
    for i in range(4):
        a1 = i * math.pi / 2.0
        a2 = a1 + math.pi / 4.0
        star_pts_1.append((int(round(cx + math.cos(a1) * 16)), int(round(cy + math.sin(a1) * 16))))
        star_pts_2.append((int(round(cx + math.cos(a2) * 16)), int(round(cy + math.sin(a2) * 16))))

    for poly in (star_pts_1, star_pts_2):
        d.polygon(poly, outline=RUNE_MID)
        for px0, py0 in poly:
            put_px(rune, px0, py0, RUNE_SPARK)

    # Stylized central eye inside star
    d.polygon([(cx - 7, cy), (cx, cy - 4), (cx + 7, cy), (cx, cy + 4)], outline=RUNE_NEON)
    d.rectangle([(cx - 1, cy - 2), (cx + 1, cy + 2)], fill=RUNE_MID)
    put_px(rune, cx, cy, RUNE_SPARK)

    # Small carved cuneiform glyphs in the 4 outer slab corners
    glyphs = [
        [(8, 8), (12, 8), (10, 12)],
        [(52, 8), (56, 8), (54, 12)],
        [(8, 56), (12, 56), (10, 52)],
        [(52, 56), (56, 56), (54, 52)],
    ]
    for g in glyphs:
        d_gl.line(g, fill=RUNE_AURA, width=3)
        d.line(g, fill=RUNE_MID, width=1)
        put_px(rune, g[1][0], g[1][1], RUNE_NEON)

    out = Image.alpha_composite(base, glow)
    return Image.alpha_composite(out, rune)


# ---------------------------------------------------------------------------
# 5. floor_stone.png (64x64 seamless cobblestones)
# ---------------------------------------------------------------------------

def draw_floor_stone() -> Image.Image:
    rng = random.Random(505)
    img = Image.new("RGBA", (TILE_SIZE, TILE_SIZE), (8, 6, 12, 255))  # #08060C damp soil

    cobble_tones = [
        (20, 17, 29, 255),      # #14111D
        (27, 23, 38, 255),      # #1B1726
        (35, 30, 49, 255),      # #231E31
        (24, 20, 35, 255),
    ]
    hi_col = (46, 39, 64, 255)  # #2E2740
    lo_col = (14, 12, 21, 255)  # #0E0C15

    # 4 seamless rows (height 16px each) with staggered widths summing to 64px
    rows = [
        (0, 16, [14, 20, 30], 0),
        (16, 32, [20, 30, 14], 8),
        (32, 48, [30, 14, 20], 18),
        (48, 64, [14, 30, 20], 4),
    ]

    for y0, y1, widths, x_off in rows:
        cursor = x_off
        for w in widths:
            base = rng.choice(cobble_tones)
            # Draw rounded cobblestone with 1px mortar margin, wrapping modulo 64
            for ly in range(y0 + 1, y1 - 1):
                for lx in range(1, w - 1):
                    # Round corners
                    if (ly in (y0 + 1, y1 - 2)) and (lx in (1, w - 2)):
                        continue
                    wx = (cursor + lx) % TILE_SIZE
                    wy = ly % TILE_SIZE
                    col = base
                    if ly == y0 + 1 or lx == 1:
                        col = hi_col
                    elif ly == y1 - 2 or lx == w - 2:
                        col = lo_col
                    elif rng.random() < 0.08:
                        col = rng.choice(FLOOR_SPECKS)
                    put_px(img, wx, wy, col)
            cursor += w

    return img


# ---------------------------------------------------------------------------
# 6. floor_dark.png (64x64 seamless polished obsidian)
# ---------------------------------------------------------------------------

def draw_floor_dark() -> Image.Image:
    rng = random.Random(606)
    img = Image.new("RGBA", (TILE_SIZE, TILE_SIZE), (10, 9, 16, 255))  # #0A0910

    obs_tones = [
        (10, 9, 16, 255),       # #0A0910
        (17, 14, 26, 255),      # #110E1A
        (24, 19, 36, 255),      # #181324
    ]
    glint = (32, 26, 48, 255)
    vein_dark = (43, 14, 68, 255)   # #2B0E44
    vein_hi = (72, 24, 114, 255)

    # Smooth low-contrast obsidian sheen bands (wrapping seamlessly)
    for y in range(TILE_SIZE):
        for x in range(TILE_SIZE):
            band = (x + y) % 32
            if band in (0, 1):
                put_px(img, x, y, obs_tones[2])
            elif band in (2, 16, 17):
                put_px(img, x, y, obs_tones[1])
            elif rng.random() < 0.06:
                put_px(img, x, y, obs_tones[1])

    # Fine diagonal glass specular reflections
    for start_x, start_y, length in ((6, 10, 14), (38, 28, 16), (18, 48, 12)):
        for i in range(length):
            put_wrap(img, start_x + i, start_y - i, glint)

    # Sparse blue-violet Void veins (wrapping cleanly)
    vein_x, vein_y = 12, 20
    for step in range(36):
        put_wrap(img, vein_x, vein_y, vein_hi if step % 7 == 0 else vein_dark)
        vein_x += 1
        vein_y += rng.choice((-1, 0, 1))

    return img


# ---------------------------------------------------------------------------
# 7. wall_brick.png (64x64 perimeter castle wall)
# ---------------------------------------------------------------------------

def draw_wall_brick(seed: int = 707) -> Image.Image:
    rng = random.Random(seed)
    img = Image.new("RGBA", (TILE_SIZE, TILE_SIZE), MORTAR_DEEP)
    d = ImageDraw.Draw(img)

    # Top parapet / cornice (y=0..12) lit by cold moonlight
    d.rectangle([(0, 0), (63, 4)], fill=WALL_CAP)
    d.line([(0, 0), (63, 0)], fill=(86, 76, 110, 255))
    d.rectangle([(0, 5), (63, 10)], fill=WALL_HI)
    # Parapet vertical bevel joints
    for jx in (16, 48):
        d.line([(jx, 0), (jx, 10)], fill=WALL_DARK)
        d.line([(jx + 1, 0), (jx + 1, 10)], fill=(82, 74, 104, 255))
    # Deep cornice drop-shadow (y=11..12)
    d.rectangle([(0, 11), (63, 12)], fill=WALL_DROP)

    # Main castle masonry body (y=13..54): 3 courses of 14px height, seamless X wrap
    courses = [
        (13, 27, [32, 32], 0),
        (27, 41, [34, 30], 16),
        (41, 55, [30, 34], 6),
    ]
    for y0, y1, widths, x_off in courses:
        cursor = x_off
        for w in widths:
            fill_col = WALL_BASE if rng.random() < 0.55 else WALL_MID
            for ly in range(y0 + 1, y1):
                for lx in range(1, w):
                    wx = (cursor + lx) % TILE_SIZE
                    col = fill_col
                    if ly == y0 + 1:
                        col = WALL_HI
                    elif ly == y1 - 1:
                        col = WALL_SHADOW
                    elif lx == 1:
                        col = WALL_MID
                    elif rng.random() < 0.08:
                        col = WALL_DARK if rng.random() < 0.5 else WALL_HI
                    put_px(img, wx, ly, col)
            cursor += w

    # Dark mossy / grime streaks in 3 masonry joints
    for mx, my, mh in ((15, 27, 9), (47, 13, 11), (31, 41, 10)):
        for dy in range(mh):
            put_px(img, mx, my + dy, WALL_MOSS)
            if dy < mh // 2:
                put_px(img, (mx + 1) % TILE_SIZE, my + dy, WALL_DROP)

    # Base plinth (y=55..63) with deep floor-contact shadow
    d.rectangle([(0, 55), (63, 59)], fill=WALL_DARK)
    d.line([(0, 55), (63, 55)], fill=WALL_HI)
    d.rectangle([(0, 60), (63, 63)], fill=WALL_FLOOR)

    return img


# ---------------------------------------------------------------------------
# 8. wall_banner.png (64x64 perimeter wall with cult banner)
# ---------------------------------------------------------------------------

def draw_wall_banner() -> Image.Image:
    base = draw_wall_brick(seed=707)
    overlay = Image.new("RGBA", (TILE_SIZE, TILE_SIZE), (0, 0, 0, 0))
    d = ImageDraw.Draw(overlay)

    # Soft cast shadow onto wall bricks to the right and bottom of banner
    shadow_poly = [
        (22, 13), (46, 13), (46, 56),
        (41, 51), (34, 58), (27, 51), (22, 57),
    ]
    d.polygon(shadow_poly, fill=(0, 0, 0, 110))

    # Wrought blackened-iron bracket mounted on cornice (y=8..12)
    d.rectangle([(18, 9), (45, 12)], fill=BANNER_IRON)
    d.line([(18, 9), (45, 9)], fill=BANNER_RIVET)
    for rx in (20, 31, 43):
        d.rectangle([(rx, 10), (rx + 1, 11)], fill=BANNER_RIVET)

    # Hanging dark-crimson cloth banner (x=20..43, y=12..56) with 3 tattered bottom prongs
    def hem_y(x: int) -> int:
        # Three jagged prongs peaking at x=23, 32, 40
        if x < 27:
            return 56 - abs(x - 23) * 2
        if x < 37:
            return 57 - abs(x - 32) * 2
        return 55 - abs(x - 40) * 2

    for x in range(20, 44):
        bottom = hem_y(x)
        # Vertical cloth folds alternating shadow, base, highlight
        fold = (x - 20) % 6
        if fold in (0, 5):
            col = BANNER_DARK
        elif fold in (1, 4):
            col = BANNER_BASE
        elif fold == 2:
            col = BANNER_HI
        else:
            col = BANNER_BRIGHT

        for y in range(12, bottom + 1):
            c = BANNER_DARK if y == 12 else col
            put_px(overlay, x, y, c)

    # Tarnished gold side trim lines
    for y in range(13, 50):
        put_px(overlay, 21, y, BANNER_GOLD_DARK)
        put_px(overlay, 42, y, BANNER_GOLD_DARK)

    # Embroidered gold cult emblem in center (around x=31..32, y=24..40)
    # Diamond & vertical ritual sigil
    d.polygon([(32, 22), (38, 30), (32, 38), (26, 30)], outline=BANNER_GOLD)
    d.line([(32, 20), (32, 41)], fill=BANNER_GOLD_HI)
    d.line([(28, 30), (36, 30)], fill=BANNER_GOLD_HI)
    d.rectangle([(31, 29), (33, 31)], fill=BANNER_DARK)
    put_px(overlay, 32, 30, BANNER_GOLD_HI)
    # Upper crescent horns of cult mark
    d.line([(28, 24), (32, 26), (36, 24)], fill=BANNER_GOLD_DARK)

    return Image.alpha_composite(base, overlay)


# ---------------------------------------------------------------------------
# Batch Generation Pipeline
# ---------------------------------------------------------------------------

GENERATORS = [
    ("floor_dungeon_plain.png", draw_floor_dungeon_plain),
    ("floor_dungeon_cracked.png", draw_floor_dungeon_cracked),
    ("floor_dungeon_blood.png", draw_floor_dungeon_blood),
    ("floor_dungeon_runic.png", draw_floor_dungeon_runic),
    ("floor_stone.png", draw_floor_stone),
    ("floor_dark.png", draw_floor_dark),
    ("wall_brick.png", draw_wall_brick),
    ("wall_banner.png", draw_wall_banner),
]


def generate_all(out_root: Path) -> list[Path]:
    out_dirs = [out_root / "assets" / "tiles", out_root / "assets" / "sprites"]
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
        description="Generate Group 6 seamless 64x64 Dungeon Floor & Wall tiles (RGBA)."
    )
    parser.add_argument(
        "--out-root",
        type=Path,
        default=Path("."),
        help="Project root directory where assets/tiles/ and assets/sprites/ will be written.",
    )
    args = parser.parse_args()

    files = generate_all(args.out_root)
    print(f"Generated {len(files)} PNG files under {(args.out_root / 'assets').resolve()}")


if __name__ == "__main__":
    main()
