#!/usr/bin/env python3
"""Procedural pixel-art sprite generator for Bosses & Heroes (32x48 RGBA).

Generates individual animation frames (32x48 px), horizontal 4-frame sprite
strips (128x48 px), and the downed-boss marker (32x48 px) with crisp 1px
silhouette outlines and alpha transparency, ready for Rust (`textures.rs`).

Usage:
    python scripts/generate_bosses.py
    python scripts/generate_bosses.py --out-root .
"""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Callable, Iterable, Tuple

from PIL import Image, ImageDraw

RGBA = Tuple[int, int, int, int]

FRAME_W = 32
FRAME_H = 48
FRAMES_PER_ANIM = 4

# ---------------------------------------------------------------------------
# Color Palettes (Exact Hex & RGBA specifications)
# ---------------------------------------------------------------------------

SHADOW_GROUND: RGBA = (0, 0, 0, 120)
OUTLINE_DARK: RGBA = (26, 32, 44, 255)      # #1A202C

PALADIN = {
    "steel_light": (237, 242, 247, 255),    # #EDF2F7
    "steel_base": (203, 213, 224, 255),     # #CBD5E0
    "steel_shadow": (74, 85, 104, 255),     # #4A5568
    "gold_light": (250, 240, 137, 255),     # #FAF089
    "gold_base": (214, 158, 46, 255),       # #D69E2E
    "cape_light": (229, 62, 62, 255),       # #E53E3E
    "cape_base": (197, 48, 48, 255),        # #C53030
    "cape_shadow": (116, 42, 42, 255),      # #742A2A
    "blade_light": (255, 255, 255, 255),    # #FFFFFF
    "blade_neon": (99, 179, 237, 255),      # #63B3ED
    "halo_glow": (236, 201, 75, 180),       # #ECC94B (a=180)
    "halo_beam": (255, 255, 255, 225),
    "outline": OUTLINE_DARK,
}

DARK_KNIGHT = {
    "obs_light": (74, 85, 104, 255),        # #4A5568
    "obs_base": (26, 32, 44, 255),          # #1A202C
    "deep_shadow": (13, 17, 23, 255),       # #0D1117
    "blood": (229, 62, 62, 255),            # #E53E3E
    "blood_dark": (139, 26, 26, 255),
    "eyes": (255, 26, 26, 255),             # #FF1A1A
    "cape": (74, 14, 23, 255),              # #4A0E17
    "cape_light": (110, 24, 36, 255),
    "horn_tip": (203, 213, 224, 255),
    "outline": (13, 17, 23, 255),           # #0D1117 for crisp obsidian silhouette
}

ARCHMAGE = {
    "robe_base": (50, 38, 89, 255),         # #322659
    "robe_light": (85, 60, 154, 255),       # #553C9A
    "robe_shadow": (29, 21, 56, 255),
    "gold_trim": (214, 158, 46, 255),       # #D69E2E
    "gold_light": (250, 240, 137, 255),     # #FAF089
    "crystal": (79, 209, 197, 255),         # #4FD1C5
    "crystal_light": (230, 255, 250, 255),  # #E6FFFA
    "crystal_aura": (79, 209, 197, 95),
    "hood_shadow": (15, 10, 28, 255),       # #0F0A1C
    "staff_silver": (203, 213, 224, 255),
    "staff_shadow": (113, 128, 150, 255),
    "outline": OUTLINE_DARK,
}

ROGUE = {
    "leather_dark": (116, 66, 16, 255),     # #744210
    "leather_light": (151, 90, 22, 255),    # #975A16
    "cloak_base": (34, 84, 61, 255),        # #22543D
    "cloak_light": (47, 110, 81, 255),
    "cloak_shadow": (20, 52, 38, 255),
    "blade_steel": (203, 213, 224, 255),    # #CBD5E0
    "blade_light": (237, 242, 247, 255),
    "poison": (56, 161, 105, 255),          # #38A169
    "poison_bright": (104, 211, 145, 255),
    "mask": (45, 55, 72, 255),              # #2D3748
    "skin_shadow": (180, 145, 120, 255),
    "eyes": (104, 211, 145, 255),
    "outline": OUTLINE_DARK,
}

DOWNED = {
    "stone_base": (113, 128, 150, 255),     # #718096
    "stone_light": (160, 174, 192, 255),    # #A0AEC0
    "stone_dark": (74, 85, 104, 255),       # #4A5568
    "blade": (203, 213, 224, 255),          # #CBD5E0
    "blade_light": (237, 242, 247, 255),
    "gold_dull": (183, 135, 42, 255),
    "orb_core": (99, 179, 237, 220),        # #63B3ED
    "orb_inner": (235, 248, 255, 240),
    "orb_ring": (144, 205, 244, 140),       # #90CDF4 (a=140)
    "outline": OUTLINE_DARK,
}

# ---------------------------------------------------------------------------
# Animation Keyframes
# ---------------------------------------------------------------------------

IDLE_PARAMS = [
    {"bob_y": 0, "leg_l": 0, "leg_r": 0, "cape_dx": 0, "sword_dy": 0, "halo_exp": 0},
    {"bob_y": 1, "leg_l": 0, "leg_r": 0, "cape_dx": 1, "sword_dy": 0, "halo_exp": 0},
    {"bob_y": 1, "leg_l": 0, "leg_r": 0, "cape_dx": 0, "sword_dy": 0, "halo_exp": 1},
    {"bob_y": 0, "leg_l": 0, "leg_r": 0, "cape_dx": -1, "sword_dy": 0, "halo_exp": 0},
]

WALK_PARAMS = [
    {"bob_y": 1, "leg_l": 0, "leg_r": 0, "cape_dx": 0, "sword_dy": 0, "halo_exp": 0},
    {"bob_y": 0, "leg_l": -2, "leg_r": 1, "cape_dx": -2, "sword_dy": 1, "halo_exp": 0},
    {"bob_y": 1, "leg_l": 0, "leg_r": 0, "cape_dx": 0, "sword_dy": 0, "halo_exp": 0},
    {"bob_y": 0, "leg_l": 1, "leg_r": -2, "cape_dx": 2, "sword_dy": -1, "halo_exp": 0},
]


# ---------------------------------------------------------------------------
# Pixel Drawing & Post-Processing Helpers
# ---------------------------------------------------------------------------

def new_canvas(w: int = FRAME_W, h: int = FRAME_H) -> Image.Image:
    return Image.new("RGBA", (w, h), (0, 0, 0, 0))


def put_px(img: Image.Image, x: int, y: int, color: RGBA) -> None:
    if 0 <= x < img.width and 0 <= y < img.height:
        img.putpixel((x, y), color)


def apply_1px_outline(layer: Image.Image, outline_color: RGBA) -> Image.Image:
    """Apply a crisp 4-connected 1px silhouette outline around opaque pixels."""
    w, h = layer.size
    src = layer.load()
    out = layer.copy()
    dst = out.load()

    for y in range(h):
        for x in range(w):
            if src[x, y][3] > 0:
                continue
            for nx, ny in ((x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)):
                if 0 <= nx < w and 0 <= ny < h and src[nx, ny][3] >= 200:
                    dst[x, y] = outline_color
                    break
    return out


def compose_frame(
    shadow_layer: Image.Image,
    body_layer: Image.Image,
    glow_layer: Image.Image | None,
    outline_color: RGBA,
) -> Image.Image:
    """Composite ground shadow -> 1px-outlined body -> translucent glow."""
    outlined = apply_1px_outline(body_layer, outline_color)
    frame = Image.alpha_composite(shadow_layer, outlined)
    if glow_layer is not None:
        frame = Image.alpha_composite(frame, glow_layer)
    return frame


def make_strip(frames: Iterable[Image.Image]) -> Image.Image:
    frame_list = list(frames)
    strip = new_canvas(FRAME_W * len(frame_list), FRAME_H)
    for idx, frame in enumerate(frame_list):
        strip.paste(frame, (idx * FRAME_W, 0))
    return strip


# ---------------------------------------------------------------------------
# Asset 1.1: High Paladin (Соло-Босс)
# ---------------------------------------------------------------------------

def draw_paladin(params: dict) -> Image.Image:
    p = PALADIN
    by = params["bob_y"]
    leg_l = params["leg_l"]
    leg_r = params["leg_r"]
    cdx = params["cape_dx"]
    sdy = params["sword_dy"]
    hexp = params["halo_exp"]

    shadow = new_canvas()
    body = new_canvas()
    glow = new_canvas()

    d_sh = ImageDraw.Draw(shadow)
    d = ImageDraw.Draw(body)
    d_gl = ImageDraw.Draw(glow)

    # Layer 0: Ground shadow
    d_sh.ellipse([(6, 43), (26, 47)], fill=SHADOW_GROUND)

    # Layer 1: Crimson cape behind back
    cape_poly = [
        (12, 19 + by),
        (20, 19 + by),
        (23 + cdx, 41),
        (9 + cdx, 41),
    ]
    d.polygon(cape_poly, fill=p["cape_base"])
    # Central cloth highlight fold
    d.polygon(
        [(15, 20 + by), (17, 20 + by), (18 + cdx, 39), (14 + cdx, 39)],
        fill=p["cape_light"],
    )
    # Shaded lower folds
    d.line([(9 + cdx, 41), (23 + cdx, 41)], fill=p["cape_shadow"])
    d.line([(10 + cdx, 40), (22 + cdx, 40)], fill=p["cape_shadow"])

    # Layer 2: Legs / Greaves & Gold Sabatons
    # Left leg
    d.rectangle([(11, 33 + leg_l), (14, 42 + leg_l)], fill=p["steel_base"])
    d.line([(12, 33 + leg_l), (12, 41 + leg_l)], fill=p["steel_light"])
    d.line([(14, 33 + leg_l), (14, 41 + leg_l)], fill=p["steel_shadow"])
    d.rectangle([(10, 42 + leg_l), (14, 44 + leg_l)], fill=p["gold_base"])
    d.line([(10, 42 + leg_l), (13, 42 + leg_l)], fill=p["gold_light"])

    # Right leg
    d.rectangle([(17, 33 + leg_r), (20, 42 + leg_r)], fill=p["steel_base"])
    d.line([(18, 33 + leg_r), (18, 41 + leg_r)], fill=p["steel_light"])
    d.line([(20, 33 + leg_r), (20, 41 + leg_r)], fill=p["steel_shadow"])
    d.rectangle([(17, 42 + leg_r), (21, 44 + leg_r)], fill=p["gold_base"])
    d.line([(18, 42 + leg_r), (21, 42 + leg_r)], fill=p["gold_light"])

    # Layer 3: Cuirass & Gold Plate Belt
    d.rectangle([(11, 19 + by), (20, 32 + by)], fill=p["steel_base"])
    d.line([(20, 19 + by), (20, 30 + by)], fill=p["steel_shadow"])
    d.rectangle([(13, 20 + by), (18, 28 + by)], fill=p["steel_light"])
    # Gold cross of faith
    d.rectangle([(15, 22 + by), (16, 27 + by)], fill=p["gold_light"])
    d.line([(13, 23 + by), (18, 23 + by)], fill=p["gold_light"])
    # Gold plate belt
    d.rectangle([(11, 30 + by), (20, 32 + by)], fill=p["gold_base"])
    d.rectangle([(15, 30 + by), (16, 32 + by)], fill=p["gold_light"])

    # Layer 4: Chamfered Gold Pauldrons
    for x0, x1 in ((8, 12), (19, 23)):
        d.rectangle([(x0, 18 + by), (x1, 23 + by)], fill=p["gold_base"])
        d.line([(x0 + 1, 18 + by), (x1 - 1, 18 + by)], fill=p["gold_light"])
        d.line([(x0, 23 + by), (x1, 23 + by)], fill=p["steel_shadow"])
        # Chamfer outer corners
        put_px(body, x0, 18 + by, (0, 0, 0, 0))
        put_px(body, x1, 18 + by, (0, 0, 0, 0))

    # Layer 5: Helmet, T-Visor, Glowing Eyes & Trefoil Crown
    d.rectangle([(12, 8 + by), (19, 18 + by)], fill=p["steel_base"])
    d.rectangle([(13, 9 + by), (17, 13 + by)], fill=p["steel_light"])
    d.line([(19, 9 + by), (19, 18 + by)], fill=p["steel_shadow"])
    # T-shaped visor slit
    d.line([(13, 14 + by), (18, 14 + by)], fill=p["outline"])
    d.rectangle([(15, 14 + by), (16, 17 + by)], fill=p["outline"])
    # Sky-blue glowing eyes
    put_px(body, 14, 14 + by, p["blade_neon"])
    put_px(body, 17, 14 + by, p["blade_neon"])
    # Gold trefoil crown
    d.line([(12, 8 + by), (19, 8 + by)], fill=p["gold_base"])
    d.line([(13, 8 + by), (18, 8 + by)], fill=p["gold_light"])
    d.rectangle([(15, 6 + by), (16, 7 + by)], fill=p["gold_light"])
    put_px(body, 13, 7 + by, p["gold_base"])
    put_px(body, 18, 7 + by, p["gold_base"])

    # Layer 6: Floating Holy Halo (translucent glow layer)
    halo_top = max(0, 2 + by - hexp)
    halo_bot = 6 + by
    d_gl.ellipse([(9, halo_top), (22, halo_bot)], outline=p["halo_glow"], width=1)
    d_gl.line([(12, halo_top), (19, halo_top)], fill=p["halo_beam"])

    # Layer 7: Heraldic Shield in Left Hand
    shield_poly = [
        (5, 21 + by),
        (11, 21 + by),
        (11, 31 + by),
        (8, 35 + by),
        (5, 31 + by),
    ]
    d.polygon(shield_poly, fill=p["steel_base"], outline=p["gold_base"])
    d.line([(6, 22 + by), (6, 30 + by)], fill=p["steel_light"])
    # Gold cross on shield
    d.line([(8, 23 + by), (8, 32 + by)], fill=p["gold_light"])
    d.line([(6, 26 + by), (10, 26 + by)], fill=p["gold_light"])

    # Layer 8: Two-Handed Greatsword in Right Hand
    sy = by + sdy
    # Blade neon edges & white core
    d.rectangle([(22, 8 + sy), (24, 26 + sy)], fill=p["blade_neon"])
    d.line([(23, 8 + sy), (23, 26 + sy)], fill=p["blade_light"])
    put_px(body, 23, 7 + sy, p["blade_light"])
    # Crossguard
    d.line([(20, 27 + sy), (26, 27 + sy)], fill=p["gold_base"])
    d.line([(22, 27 + sy), (24, 27 + sy)], fill=p["gold_light"])
    # Grip & pommel
    d.line([(23, 28 + sy), (23, 31 + sy)], fill=p["cape_shadow"])
    put_px(body, 23, 32 + sy, p["gold_light"])

    return compose_frame(shadow, body, glow, p["outline"])


# ---------------------------------------------------------------------------
# Asset 1.2: Dark Knight / Berserker (Кооп-Танк)
# ---------------------------------------------------------------------------

def draw_dark_knight(params: dict) -> Image.Image:
    p = DARK_KNIGHT
    by = params["bob_y"]
    leg_l = params["leg_l"]
    leg_r = params["leg_r"]
    cdx = params["cape_dx"]
    sdy = params["sword_dy"]

    shadow = new_canvas()
    body = new_canvas()
    d_sh = ImageDraw.Draw(shadow)
    d = ImageDraw.Draw(body)

    # Layer 0: Ground shadow
    d_sh.ellipse([(5, 43), (27, 47)], fill=SHADOW_GROUND)

    # Layer 1: Tattered Dark Cloak with torn bottom edge
    cape_poly = [
        (11, 19 + by),
        (21, 19 + by),
        (24 + cdx, 41),
        (8 + cdx, 41),
    ]
    d.polygon(cape_poly, fill=p["cape"])
    d.polygon(
        [(14, 21 + by), (17, 21 + by), (18 + cdx, 38), (13 + cdx, 38)],
        fill=p["cape_light"],
    )
    # Ragged tears along hem
    for tx in (10 + cdx, 14 + cdx, 19 + cdx, 22 + cdx):
        put_px(body, tx, 41, (0, 0, 0, 0))
        put_px(body, tx, 40, (0, 0, 0, 0))

    # Layer 2: Obsidian Legs & Sabatons
    d.rectangle([(11, 33 + leg_l), (14, 42 + leg_l)], fill=p["obs_base"])
    d.line([(12, 33 + leg_l), (12, 41 + leg_l)], fill=p["obs_light"])
    d.rectangle([(10, 42 + leg_l), (14, 44 + leg_l)], fill=p["deep_shadow"])
    d.line([(10, 42 + leg_l), (13, 42 + leg_l)], fill=p["obs_light"])

    d.rectangle([(17, 33 + leg_r), (20, 42 + leg_r)], fill=p["obs_base"])
    d.line([(18, 33 + leg_r), (18, 41 + leg_r)], fill=p["obs_light"])
    d.rectangle([(17, 42 + leg_r), (21, 44 + leg_r)], fill=p["deep_shadow"])
    d.line([(18, 42 + leg_r), (21, 42 + leg_r)], fill=p["obs_light"])

    # Layer 3: Obsidian Cuirass with Glowing Corruption Veins
    d.rectangle([(10, 19 + by), (21, 32 + by)], fill=p["obs_base"])
    d.rectangle([(12, 20 + by), (18, 27 + by)], fill=p["obs_light"])
    d.rectangle([(13, 21 + by), (17, 26 + by)], fill=p["obs_base"])
    # Glowing crimson corruption cracks
    for vx, vy in (
        (13, 22), (14, 23), (15, 24), (16, 24), (17, 25), (15, 25), (14, 26), (18, 22)
    ):
        put_px(body, vx, vy + by, p["blood"])
    # Heavy belt with crimson buckle
    d.rectangle([(10, 30 + by), (21, 32 + by)], fill=p["deep_shadow"])
    d.rectangle([(15, 30 + by), (16, 32 + by)], fill=p["blood"])

    # Left armored gauntlet (no shield)
    d.rectangle([(7, 24 + by), (10, 31 + by)], fill=p["obs_base"])
    d.line([(8, 24 + by), (8, 30 + by)], fill=p["obs_light"])
    put_px(body, 7, 28 + by, p["blood"])

    # Layer 4: Enlarged Spiked Pauldrons with Steel Fangs at Y=17
    for x0, x1, fangs in ((7, 12, (7, 9, 11)), (19, 24, (20, 22, 24))):
        d.rectangle([(x0, 18 + by), (x1, 24 + by)], fill=p["obs_base"])
        d.line([(x0, 18 + by), (x1, 18 + by)], fill=p["obs_light"])
        d.line([(x0 + 1, 21 + by), (x1 - 1, 21 + by)], fill=p["blood"])
        for fx in fangs:
            put_px(body, fx, 17 + by, p["obs_light"])
            put_px(body, fx, 16 + by, p["horn_tip"])

    # Layer 5: Horned Obsidian Helmet & Berserker Red Eyes
    d.rectangle([(12, 8 + by), (19, 18 + by)], fill=p["obs_base"])
    d.line([(13, 9 + by), (18, 9 + by)], fill=p["obs_light"])
    d.line([(13, 14 + by), (18, 14 + by)], fill=p["deep_shadow"])
    d.rectangle([(15, 14 + by), (16, 17 + by)], fill=p["deep_shadow"])
    # Glowing berserker eyes
    put_px(body, 14, 14 + by, p["eyes"])
    put_px(body, 17, 14 + by, p["eyes"])

    # Curved massive horns: [(8,4),(12,8)] and [(19,8),(23,4)]
    left_horn = [(12, 8), (11, 8), (10, 7), (9, 6), (8, 5), (8, 4)]
    right_horn = [(19, 8), (20, 8), (21, 7), (22, 6), (23, 5), (23, 4)]
    for hx, hy in left_horn + right_horn:
        col = p["horn_tip"] if hy <= 5 else p["obs_light"]
        put_px(body, hx, hy + by, col)
        put_px(body, hx, hy + 1 + by, p["obs_base"])

    # Layer 6: Brutal 4px-Wide Serrated Claymore [(21, 6), (25, 28)]
    sy = by + sdy
    d.rectangle([(21, 7 + sy), (25, 28 + sy)], fill=p["obs_light"])
    d.rectangle([(22, 7 + sy), (24, 28 + sy)], fill=p["obs_base"])
    d.line([(23, 8 + sy), (23, 27 + sy)], fill=p["obs_light"])
    # Pointed tip
    d.polygon([(21, 7 + sy), (25, 7 + sy), (23, 4 + sy)], fill=p["obs_light"])
    # Serrated notches along edges
    for ny in (10, 14, 18, 22):
        put_px(body, 25, ny + sy, (0, 0, 0, 0))
        put_px(body, 21, ny + 2 + sy, (0, 0, 0, 0))
    # Crimson blood drops on blade
    for bx, by_off in ((22, 11), (24, 15), (23, 19), (22, 24), (24, 25)):
        put_px(body, bx, by_off + sy, p["blood"])
    # Heavy crossguard & grip
    d.rectangle([(19, 28 + sy), (27, 29 + sy)], fill=p["deep_shadow"])
    d.line([(20, 28 + sy), (26, 28 + sy)], fill=p["obs_light"])
    d.rectangle([(22, 30 + sy), (24, 34 + sy)], fill=p["cape"])
    put_px(body, 23, 35 + sy, p["blood"])

    return compose_frame(shadow, body, None, p["outline"])


# ---------------------------------------------------------------------------
# Asset 1.3: Astral Inquisitor / Archmage (Кооп-Кастер)
# ---------------------------------------------------------------------------

def draw_archmage(params: dict) -> Image.Image:
    p = ARCHMAGE
    by = params["bob_y"]
    leg_l = params["leg_l"]
    leg_r = params["leg_r"]
    cdx = params["cape_dx"]
    sdy = params["sword_dy"]
    hexp = params["halo_exp"]

    shadow = new_canvas()
    body = new_canvas()
    glow = new_canvas()

    d_sh = ImageDraw.Draw(shadow)
    d = ImageDraw.Draw(body)
    d_gl = ImageDraw.Draw(glow)

    # Layer 0: Ground shadow
    d_sh.ellipse([(7, 43), (25, 47)], fill=SHADOW_GROUND)

    # Layer 1: Boots peeking beneath robe during walk
    d.rectangle([(12, 41 + leg_l), (14, 44 + leg_l)], fill=p["hood_shadow"])
    d.rectangle([(17, 41 + leg_r), (19, 44 + leg_r)], fill=p["hood_shadow"])

    # Layer 2: Flowing Indigo Robe & Gold Trim
    robe_poly = [
        (11, 18 + by),
        (20, 18 + by),
        (22 + cdx // 2, 42),
        (9 + cdx // 2, 42),
    ]
    d.polygon(robe_poly, fill=p["robe_base"])
    # Highlighted cloth folds
    d.polygon(
        [(13, 19 + by), (15, 19 + by), (14 + cdx // 2, 41), (11 + cdx // 2, 41)],
        fill=p["robe_light"],
    )
    d.polygon(
        [(17, 19 + by), (19, 19 + by), (20 + cdx // 2, 41), (18 + cdx // 2, 41)],
        fill=p["robe_light"],
    )
    # Central gold-trimmed stole & hem
    d.rectangle([(15, 19 + by), (16, 41)], fill=p["gold_trim"])
    d.line([(9 + cdx // 2, 42), (22 + cdx // 2, 42)], fill=p["gold_trim"])
    d.line([(11, 29 + by), (20, 29 + by)], fill=p["gold_trim"])

    # Wide wizard sleeves / shoulders
    d.rectangle([(8, 19 + by), (12, 25 + by)], fill=p["robe_base"])
    d.line([(8, 25 + by), (12, 25 + by)], fill=p["gold_trim"])
    d.rectangle([(19, 19 + by), (23, 25 + by)], fill=p["robe_base"])
    d.line([(19, 25 + by), (23, 25 + by)], fill=p["gold_trim"])

    # Layer 3: Deep Hood & Shadowed Face with Turquoise Eyes
    # Outer hood silhouette
    d.polygon(
        [(16, 5 + by), (21, 10 + by), (20, 18 + by), (11, 18 + by), (10, 10 + by)],
        fill=p["robe_base"],
    )
    # Hood highlight rim & gold trim
    d.line([(16, 5 + by), (11, 10 + by)], fill=p["robe_light"])
    d.line([(16, 5 + by), (20, 10 + by)], fill=p["robe_light"])
    d.line([(11, 18 + by), (20, 18 + by)], fill=p["gold_trim"])
    # Triangular fabric overhang [(12, 6), (19, 6), (16, 17)] framing shadow
    d.polygon(
        [(12, 9 + by), (19, 9 + by), (19, 17 + by), (12, 17 + by)],
        fill=p["hood_shadow"],
    )
    d.polygon(
        [(12, 6 + by), (19, 6 + by), (16, 11 + by)],
        fill=p["robe_light"],
    )
    # Two mystical turquoise astral eyes at (14, 13) & (17, 13)
    put_px(body, 14, 13 + by, p["crystal"])
    put_px(body, 17, 13 + by, p["crystal"])
    put_px(glow, 14, 13 + by, p["crystal_light"])
    put_px(glow, 17, 13 + by, p["crystal_light"])

    # Layer 4: Floating Open Tome of Time in Left Hand [(4, 22), (10, 28)]
    tome_y = by - hexp
    d.rectangle([(4, 22 + tome_y), (10, 28 + tome_y)], fill=p["robe_shadow"])
    d.rectangle([(5, 23 + tome_y), (9, 27 + tome_y)], fill=p["gold_light"])
    d.line([(7, 23 + tome_y), (7, 27 + tome_y)], fill=p["gold_trim"])
    put_px(body, 6, 25 + tome_y, p["crystal"])
    put_px(body, 8, 25 + tome_y, p["crystal"])

    # Layer 5: Silver Staff (X=24, Y=8..38) & Diamond Astral Crystal [(22, 4), (26, 8)]
    sy = by + sdy
    d.line([(24, 8 + sy), (24, 38 + sy)], fill=p["staff_silver"])
    d.line([(24, 20 + sy), (24, 38 + sy)], fill=p["staff_shadow"])
    put_px(body, 24, 10 + sy, p["gold_trim"])
    put_px(body, 24, 15 + sy, p["gold_trim"])

    # Diamond crystal at [(22, 4), (26, 8)]
    crystal_poly = [
        (24, 4 + sy - hexp),
        (26, 6 + sy - hexp),
        (24, 8 + sy - hexp),
        (22, 6 + sy - hexp),
    ]
    d.polygon(crystal_poly, fill=p["crystal"])
    put_px(body, 24, 6 + sy - hexp, p["crystal_light"])
    put_px(body, 24, 5 + sy - hexp, p["crystal_light"])
    # Translucent astral aura around crystal
    d_gl.ellipse(
        [(20, 2 + sy - hexp), (28, 10 + sy - hexp)],
        fill=p["crystal_aura"],
    )

    return compose_frame(shadow, body, glow, p["outline"])


# ---------------------------------------------------------------------------
# Asset 1.4: Shadow Ranger / Rogue (Кооп-Скорость)
# ---------------------------------------------------------------------------

def draw_rogue(params: dict) -> Image.Image:
    p = ROGUE
    by = params["bob_y"]
    leg_l = params["leg_l"]
    leg_r = params["leg_r"]
    cdx = params["cape_dx"]
    sdy = params["sword_dy"]

    shadow = new_canvas()
    body = new_canvas()
    d_sh = ImageDraw.Draw(shadow)
    d = ImageDraw.Draw(body)

    # Layer 0: Ground shadow
    d_sh.ellipse([(6, 43), (26, 47)], fill=SHADOW_GROUND)

    # Layer 1: Forest Camouflage Cloak (low crouch: starts 2px lower)
    cloak_poly = [
        (11, 21 + by),
        (20, 21 + by),
        (23 + cdx, 40),
        (8 + cdx, 40),
    ]
    d.polygon(cloak_poly, fill=p["cloak_base"])
    d.polygon(
        [(13, 22 + by), (17, 22 + by), (18 + cdx, 38), (12 + cdx, 38)],
        fill=p["cloak_light"],
    )
    d.line([(8 + cdx, 40), (23 + cdx, 40)], fill=p["cloak_shadow"])

    # Layer 2: Agile Bent Legs & Leather Boots
    d.rectangle([(11, 34 + leg_l), (14, 43 + leg_l)], fill=p["mask"])
    d.rectangle([(10, 41 + leg_l), (14, 44 + leg_l)], fill=p["leather_dark"])
    d.line([(11, 41 + leg_l), (13, 41 + leg_l)], fill=p["leather_light"])

    d.rectangle([(17, 34 + leg_r), (20, 43 + leg_r)], fill=p["mask"])
    d.rectangle([(17, 41 + leg_r), (21, 44 + leg_r)], fill=p["leather_dark"])
    d.line([(18, 41 + leg_r), (20, 41 + leg_r)], fill=p["leather_light"])

    # Layer 3: Leather Jerkin & Cross-Straps
    d.rectangle([(11, 21 + by), (20, 34 + by)], fill=p["leather_dark"])
    d.rectangle([(13, 22 + by), (18, 30 + by)], fill=p["leather_light"])
    # Diagonal bandolier strap
    d.line([(12, 22 + by), (19, 30 + by)], fill=p["mask"])
    # Belt
    d.rectangle([(11, 31 + by), (20, 33 + by)], fill=p["mask"])
    put_px(body, 15, 32 + by, p["poison"])
    put_px(body, 16, 32 + by, p["poison"])

    # Hooded scarf / shoulders
    d.rectangle([(9, 20 + by), (22, 23 + by)], fill=p["cloak_base"])
    d.line([(10, 20 + by), (21, 20 + by)], fill=p["cloak_light"])

    # Layer 4: Low Crouched Head (Y starts at 10+by) & Half-Face Silk Mask
    d.rectangle([(12, 10 + by), (19, 19 + by)], fill=p["cloak_base"])
    d.line([(13, 10 + by), (18, 10 + by)], fill=p["cloak_light"])
    # Upper face / eyes band
    d.rectangle([(13, 14 + by), (18, 15 + by)], fill=p["skin_shadow"])
    put_px(body, 14, 14 + by, p["eyes"])
    put_px(body, 17, 14 + by, p["eyes"])
    # Dark-gray silk half-mask over lower face
    d.rectangle([(12, 16 + by), (19, 19 + by)], fill=p["mask"])

    # Layer 5: Dual Curved Wind Daggers (point-down)
    # Left dagger: [(5, 24), (8, 33)]
    ly = by - sdy
    d.rectangle([(7, 23 + ly), (10, 26 + ly)], fill=p["leather_light"])  # hand/grip
    d.polygon(
        [(8, 25 + ly), (6, 25 + ly), (5, 30 + ly), (6, 33 + ly), (8, 29 + ly)],
        fill=p["blade_steel"],
    )
    d.line([(7, 26 + ly), (6, 29 + ly)], fill=p["blade_light"])
    # Emerald poison gradient toward tip
    d.polygon([(5, 30 + ly), (7, 30 + ly), (6, 33 + ly)], fill=p["poison"])
    put_px(body, 6, 33 + ly, p["poison_bright"])

    # Right dagger: [(23, 24), (26, 33)]
    ry = by + sdy
    d.rectangle([(21, 23 + ry), (24, 26 + ry)], fill=p["leather_light"])  # hand/grip
    d.polygon(
        [(23, 25 + ry), (25, 25 + ry), (26, 30 + ry), (25, 33 + ry), (23, 29 + ry)],
        fill=p["blade_steel"],
    )
    d.line([(24, 26 + ry), (25, 29 + ry)], fill=p["blade_light"])
    # Emerald poison gradient toward tip
    d.polygon([(24, 30 + ry), (26, 30 + ry), (25, 33 + ry)], fill=p["poison"])
    put_px(body, 25, 33 + ry, p["poison_bright"])

    return compose_frame(shadow, body, None, p["outline"])


# ---------------------------------------------------------------------------
# Asset 1.5: Downed Boss (Павший Босс / Ghost Shroud — 32x48)
# ---------------------------------------------------------------------------

def draw_boss_downed() -> Image.Image:
    p = DOWNED
    shadow = new_canvas()
    body = new_canvas()
    glow = new_canvas()

    d_sh = ImageDraw.Draw(shadow)
    d = ImageDraw.Draw(body)
    d_gl = ImageDraw.Draw(glow)

    # Ground shadow
    d_sh.ellipse([(5, 41), (27, 46)], fill=SHADOW_GROUND)

    # Split Paladin shield on ground [(6, 34), (16, 43)]
    # Left cracked half
    d.polygon([(6, 35), (10, 34), (11, 38), (9, 43), (6, 41)], fill=p["stone_base"])
    d.line([(7, 36), (7, 40)], fill=p["stone_light"])
    d.line([(9, 43), (6, 41)], fill=p["stone_dark"])
    # Right cracked half (separated by 1px jagged fissure)
    d.polygon([(12, 34), (16, 35), (16, 41), (13, 43), (11, 39)], fill=p["stone_base"])
    d.line([(15, 36), (15, 40)], fill=p["stone_dark"])
    d.line([(12, 35), (14, 36)], fill=p["stone_light"])

    # Broken sword thrust into ground at an angle [(14, 24), (20, 38)]
    # Angled blade (snapped off near top-right)
    d.polygon([(18, 25), (20, 26), (17, 38), (15, 38)], fill=p["blade"])
    d.line([(18, 26), (16, 37)], fill=p["blade_light"])
    # Jagged snapped tip
    put_px(body, 19, 24, p["blade_light"])
    # Tilted crossguard & hilt sticking out
    d.line([(16, 26), (22, 29)], fill=p["gold_dull"], width=2)
    d.line([(19, 25), (21, 22)], fill=p["stone_dark"], width=2)
    put_px(body, 21, 21, p["gold_dull"])

    # Ethereal soul orb centered at (16, 16), radius 6px, + outer rescue ring
    cx, cy = 16, 16
    # Outer semi-transparent rescue ring (#90CDF4, alpha 140)
    d_gl.ellipse([(cx - 8, cy - 8), (cx + 8, cy + 8)], outline=p["orb_ring"], width=2)
    # Core ethereal soul sphere (radius 6px, #63B3ED)
    d_gl.ellipse([(cx - 6, cy - 6), (cx + 6, cy + 6)], fill=p["orb_core"])
    # Inner radiant highlight
    d_gl.ellipse([(cx - 3, cy - 3), (cx + 3, cy + 3)], fill=p["orb_inner"])

    return compose_frame(shadow, body, glow, p["outline"])


# ---------------------------------------------------------------------------
# Batch Generation Pipeline
# ---------------------------------------------------------------------------

def generate_animated_asset(
    name: str,
    draw_fn: Callable[[dict], Image.Image],
    out_dirs: list[Path],
) -> list[Path]:
    saved: list[Path] = []
    for anim_name, keyframe_list in (("idle", IDLE_PARAMS), ("walk", WALK_PARAMS)):
        frames = [draw_fn(params) for params in keyframe_list]
        strip = make_strip(frames)

        for out_dir in out_dirs:
            out_dir.mkdir(parents=True, exist_ok=True)
            for idx, frame in enumerate(frames):
                path = out_dir / f"{name}_{anim_name}_{idx}.png"
                frame.save(path, format="PNG")
                saved.append(path)
            strip_path = out_dir / f"{name}_{anim_name}_strip.png"
            strip.save(strip_path, format="PNG")
            saved.append(strip_path)
    return saved


def generate_all(out_root: Path) -> list[Path]:
    heroes_dir = out_root / "assets" / "heroes"
    sprites_dir = out_root / "assets" / "sprites"

    heroes_dir.mkdir(parents=True, exist_ok=True)
    sprites_dir.mkdir(parents=True, exist_ok=True)

    saved: list[Path] = []

    # High Paladin saved to both assets/heroes/ and assets/sprites/
    saved.extend(generate_animated_asset("knight", draw_paladin, [heroes_dir, sprites_dir]))
    # Co-op Bosses saved to assets/heroes/
    saved.extend(generate_animated_asset("dark_knight", draw_dark_knight, [heroes_dir]))
    saved.extend(generate_animated_asset("archmage", draw_archmage, [heroes_dir]))
    saved.extend(generate_animated_asset("rogue", draw_rogue, [heroes_dir]))

    # Downed Boss Tombstone / Ghost Shroud
    downed_img = draw_boss_downed()
    downed_path = heroes_dir / "boss_downed.png"
    downed_img.save(downed_path, format="PNG")
    saved.append(downed_path)

    return saved


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Generate 32x48 RGBA pixel-art boss sprites and 128x48 strips."
    )
    parser.add_argument(
        "--out-root",
        type=Path,
        default=Path("."),
        help="Project root directory where assets/heroes/ and assets/sprites/ will be written.",
    )
    args = parser.parse_args()

    files = generate_all(args.out_root)
    print(f"Generated {len(files)} PNG files under {(args.out_root / 'assets').resolve()}")


if __name__ == "__main__":
    main()