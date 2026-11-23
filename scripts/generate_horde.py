#!/usr/bin/env python3
"""Procedural pixel-art sprite generator for The Infernal Horde (Tiers 1–8).

Generates all 8 Horde archetypes (32x32, 36x24 + 144x24 4-frame gallop strip,
36x36, and 64x64 Reaper Titan) in RGBA PNG format with contact shadows,
1px contrast outlines, and translucent glow layers, ready for `textures.rs`.

Usage:
    python scripts/generate_horde.py
    python scripts/generate_horde.py --out-root .
"""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Iterable, Tuple

from PIL import Image, ImageDraw

RGBA = Tuple[int, int, int, int]

SHADOW_DEFAULT: RGBA = (0, 0, 0, 120)
OUTLINE_DARK: RGBA = (12, 10, 18, 255)      # #0C0A12 high-contrast 1px border


# ---------------------------------------------------------------------------
# Color Palettes (Tiers 1–8)
# ---------------------------------------------------------------------------

T1_SKELETON = {
    "bone": (244, 235, 217, 255),           # #F4EBD9
    "bone_shadow": (196, 182, 158, 255),
    "socket": (18, 18, 26, 255),            # #12121A
    "eye_spark": (255, 34, 51, 255),        # #FF2233
    "wood": (115, 74, 46, 255),
    "wood_dark": (74, 45, 26, 255),
    "iron": (148, 163, 184, 255),
    "iron_light": (226, 232, 240, 255),
}

T2_HOUND = {
    "fur": (26, 17, 24, 255),               # #1A1118
    "spine": (92, 58, 82, 255),             # #5C3A52
    "snarl": (255, 208, 0, 255),            # #FFD000
    "gums": (217, 4, 41, 255),              # #D90429
    "fang": (255, 255, 255, 255),           # #FFFFFF
    "outline": (10, 6, 10, 255),
}

T3_CULTIST = {
    "robe": (43, 30, 74, 255),              # #2B1E4A
    "robe_fold": (74, 50, 125, 255),        # #4A327D
    "robe_dark": (26, 17, 46, 255),
    "hood_shadow": (13, 8, 24, 255),
    "neon": (6, 214, 160, 255),             # #06D6A0
    "core": (224, 251, 252, 255),           # #E0FBFC
    "aura": (6, 214, 160, 95),
    "wood": (107, 68, 35, 255),
    "wood_light": (148, 98, 56, 255),
}

T4_GOLEM = {
    "basalt": (36, 40, 54, 255),            # #242836
    "basalt_light": (64, 72, 94, 255),
    "basalt_dark": (20, 23, 33, 255),
    "ice": (72, 202, 228, 255),             # #48CAE4
    "ice_mid": (144, 224, 239, 255),
    "ice_core": (224, 251, 252, 255),       # #E0FBFC
    "ice_glow": (72, 202, 228, 95),
}

T5_KAMIKAZE = {
    "imp": (230, 57, 70, 255),              # #E63946
    "imp_dark": (155, 34, 38, 255),
    "imp_light": (255, 107, 107, 255),
    "horn": (45, 35, 42, 255),
    "eye_white": (255, 255, 255, 255),
    "pupil": (18, 18, 26, 255),
    "iron": (26, 17, 24, 255),              # #1A1118
    "iron_spec": (58, 42, 56, 255),         # #3A2A38
    "fuse": (186, 152, 104, 255),
    "spark_outer": (255, 84, 0, 255),       # #FF5400
    "spark_core": (255, 230, 0, 255),       # #FFE600
    "spark_glow": (255, 84, 0, 120),
}

T6_SHADOW_DEMON = {
    "ether": (58, 12, 163, 235),            # #3A0CA3
    "bone_violet": (114, 9, 183, 255),      # #7209B7
    "smoke_dark": (29, 8, 66, 210),
    "wing_crimson": (255, 0, 84, 255),      # #FF0054
    "wing_glow": (255, 0, 84, 90),
    "mask": (36, 16, 62, 255),
    "mask_rim": (114, 9, 183, 255),
    "shadow": (0, 0, 0, 85),
}

T7_FIRE_ELEMENTAL = {
    "floor_glow": (244, 140, 6, 70),        # rgba(244, 140, 6, 70)
    "outer": (208, 0, 0, 255),              # #D00000
    "mid": (244, 140, 6, 255),              # #F48C06
    "inner": (255, 186, 8, 255),            # #FFBA08
    "core": (255, 243, 176, 255),           # #FFF3B0
    "eye": (255, 255, 255, 255),
    "aura": (255, 186, 8, 70),
    "outline": (92, 12, 12, 255),
}

T8_REAPER = {
    "cloak": (11, 8, 19, 255),              # #0B0813
    "cloak_fold": (30, 22, 51, 255),
    "cloak_rim": (54, 42, 86, 255),
    "mask": (229, 222, 206, 255),           # #E5DECE
    "mask_shadow": (166, 156, 138, 255),
    "eye": (72, 202, 228, 255),             # #48CAE4
    "eye_core": (224, 251, 252, 255),
    "ebony": (28, 22, 34, 255),
    "ebony_light": (66, 56, 78, 255),
    "blade_steel": (52, 68, 88, 255),
    "blade_neon": (72, 202, 228, 255),      # #48CAE4
    "blade_core": (224, 251, 252, 255),
    "blade_glow": (72, 202, 228, 100),
    "shadow": (0, 0, 0, 140),
    "outline": (24, 18, 38, 255),
}


# ---------------------------------------------------------------------------
# Shared Pixel-Art Helpers
# ---------------------------------------------------------------------------

def new_canvas(w: int, h: int) -> Image.Image:
    return Image.new("RGBA", (w, h), (0, 0, 0, 0))


def put_px(img: Image.Image, x: int, y: int, color: RGBA) -> None:
    if 0 <= x < img.width and 0 <= y < img.height:
        img.putpixel((x, y), color)


def apply_1px_outline(layer: Image.Image, outline_color: RGBA) -> Image.Image:
    """Apply a crisp 4-connected 1px contrast outline around solid pixels."""
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
    glow: Image.Image | None = None,
    outline_color: RGBA = OUTLINE_DARK,
) -> Image.Image:
    outlined = apply_1px_outline(body, outline_color)
    img = Image.alpha_composite(shadow, outlined)
    if glow is not None:
        img = Image.alpha_composite(img, glow)
    return img


def make_strip(frames: Iterable[Image.Image]) -> Image.Image:
    frame_list = list(frames)
    w, h = frame_list[0].size
    strip = new_canvas(w * len(frame_list), h)
    for idx, frame in enumerate(frame_list):
        strip.paste(frame, (idx * w, 0))
    return strip


# ---------------------------------------------------------------------------
# T1: Skeleton Footman (32x32)
# ---------------------------------------------------------------------------

def draw_skeleton() -> Image.Image:
    p = T1_SKELETON
    shadow = new_canvas(32, 32)
    body = new_canvas(32, 32)
    d_sh = ImageDraw.Draw(shadow)
    d = ImageDraw.Draw(body)

    # Layer 0: Ground shadow
    d_sh.ellipse([(8, 28), (24, 31)], fill=SHADOW_DEFAULT)

    # Layer 1: Legs (2px thick vertical bones at X=12..13 and X=17..18, Y=21..28)
    d.rectangle([(12, 21), (13, 28)], fill=p["bone"])
    d.line([(13, 21), (13, 28)], fill=p["bone_shadow"])
    d.rectangle([(17, 21), (18, 28)], fill=p["bone"])
    d.line([(18, 21), (18, 28)], fill=p["bone_shadow"])
    # Pelvis
    d.rectangle([(12, 20), (18, 21)], fill=p["bone_shadow"])
    d.line([(13, 20), (17, 20)], fill=p["bone"])

    # Spine (X=13..16) & Ribs (step 2px)
    d.rectangle([(14, 13), (16, 20)], fill=p["bone_shadow"])
    d.line([(15, 13), (15, 20)], fill=p["bone"])
    for ry in (14, 16, 18):
        d.line([(12, ry), (18, ry)], fill=p["bone"])
    # Collarbone & bony arms
    d.line([(11, 13), (19, 13)], fill=p["bone"])
    d.line([(10, 14), (10, 18)], fill=p["bone_shadow"])
    d.line([(20, 14), (20, 19)], fill=p["bone"])

    # Skull monoblock [(11, 5), (18, 12)]
    d.rectangle([(11, 5), (18, 12)], fill=p["bone"])
    d.line([(11, 11), (18, 11)], fill=p["bone_shadow"])
    # Jaw teeth notches
    put_px(body, 11, 12, (0, 0, 0, 0))
    put_px(body, 18, 12, (0, 0, 0, 0))
    put_px(body, 14, 12, p["socket"])
    put_px(body, 16, 12, p["socket"])
    # Hollow eye sockets & red soul sparks
    d.rectangle([(12, 7), (14, 9)], fill=p["socket"])
    d.rectangle([(16, 7), (18, 9)], fill=p["socket"])
    put_px(body, 13, 8, p["eye_spark"])
    put_px(body, 17, 8, p["eye_spark"])
    # Nasal cavity
    put_px(body, 15, 10, p["socket"])

    # Layer 2: Cracked Wooden-Iron Shield [(6, 13), (11, 21)]
    d.rectangle([(6, 13), (11, 21)], fill=p["wood"], outline=p["iron"])
    d.line([(8, 14), (8, 20)], fill=p["wood_dark"])
    # Diagonal crack
    for cx, cy in ((7, 15), (8, 16), (9, 17), (10, 17)):
        put_px(body, cx, cy, p["socket"])

    # Short Serrated Sword [(19, 16), (25, 23)]
    d.line([(20, 21), (25, 16)], fill=p["iron"], width=2)
    d.line([(21, 20), (25, 16)], fill=p["iron_light"])
    # Serrated notch
    put_px(body, 23, 17, (0, 0, 0, 0))
    # Crossguard & hilt
    d.line([(19, 20), (22, 23)], fill=p["wood_dark"])
    put_px(body, 19, 22, p["wood"])

    return compose(shadow, body)


# ---------------------------------------------------------------------------
# T2: Infernal Hound (36x24, 4-frame gallop strip 144x24)
# ---------------------------------------------------------------------------

HOUND_GALLOP = [
    {"body_dy": 0, "fl": (-2, 0), "fr": (2, -1), "bl": (2, -1), "br": (-2, 0), "tail_dy": -1},
    {"body_dy": -1, "fl": (1, -2), "fr": (-1, -1), "bl": (-1, -2), "br": (1, -1), "tail_dy": 0},
    {"body_dy": 0, "fl": (2, 0), "fr": (-2, -1), "bl": (-2, -1), "br": (2, 0), "tail_dy": 1},
    {"body_dy": 1, "fl": (-1, -1), "fr": (1, 0), "bl": (1, 0), "br": (-1, -1), "tail_dy": 0},
]


def draw_hound(frame_idx: int) -> Image.Image:
    p = T2_HOUND
    st = HOUND_GALLOP[frame_idx % len(HOUND_GALLOP)]
    dy = st["body_dy"]
    fl_x, fl_y = st["fl"]
    fr_x, fr_y = st["fr"]
    bl_x, bl_y = st["bl"]
    br_x, br_y = st["br"]
    tdy = st["tail_dy"]

    shadow = new_canvas(36, 24)
    body = new_canvas(36, 24)
    d_sh = ImageDraw.Draw(shadow)
    d = ImageDraw.Draw(body)

    # Layer 0: Elongated ground shadow
    d_sh.ellipse([(5, 20), (31, 23)], fill=SHADOW_DEFAULT)

    # Layer 1: Whip tail (8, 14) -> (2, 9)
    d.line([(9, 13 + dy), (5, 11 + dy + tdy), (2, 9 + tdy)], fill=p["fur"], width=2)
    d.line([(8, 12 + dy), (3, 9 + tdy)], fill=p["spine"])

    # Back legs (powerful angled thighs)
    d.line([(11, 14 + dy), (9 + bl_x, 18 + bl_y), (11 + bl_x, 21 + bl_y)], fill=p["fur"], width=2)
    d.line([(13, 14 + dy), (11 + br_x, 18 + br_y), (13 + br_x, 21 + br_y)], fill=p["fur"], width=2)

    # Front legs (alternating thrust ±2px)
    d.line([(23, 14 + dy), (25 + fl_x, 18 + fl_y), (27 + fl_x, 21 + fl_y)], fill=p["fur"], width=2)
    d.line([(25, 14 + dy), (27 + fr_x, 18 + fr_y), (29 + fr_x, 21 + fr_y)], fill=p["fur"], width=2)

    # Curved wedge torso (8, 13) .. (27, 16)
    torso = [
        (8, 13 + dy),
        (14, 9 + dy),
        (24, 9 + dy),
        (27, 13 + dy),
        (26, 16 + dy),
        (11, 15 + dy),
    ]
    d.polygon(torso, fill=p["fur"])
    # Spine highlight #5C3A52 for readability
    d.line([(10, 11 + dy), (15, 9 + dy), (23, 9 + dy), (26, 11 + dy)], fill=p["spine"])
    # Muscular rib ridge
    d.line([(16, 11 + dy), (22, 11 + dy)], fill=p["spine"])

    # Layer 2: Predatory Wedge Head, Jaws, Golden Snarl & Fangs
    head = [
        (24, 9 + dy),
        (28, 6 + dy),   # swept-back ear
        (29, 8 + dy),
        (34, 11 + dy),  # upper snout tip
        (33, 13 + dy),
        (27, 14 + dy),
    ]
    d.polygon(head, fill=p["fur"])
    d.line([(26, 8 + dy), (28, 6 + dy)], fill=p["spine"])
    # Lower jaw
    d.polygon([(27, 14 + dy), (33, 15 + dy), (32, 16 + dy), (26, 15 + dy)], fill=p["fur"])
    # Bloodied gums & bright yellow snarl/eye
    d.line([(29, 13 + dy), (32, 14 + dy)], fill=p["gums"])
    put_px(body, 29, 10 + dy, p["snarl"])
    put_px(body, 30, 10 + dy, p["snarl"])
    put_px(body, 31, 12 + dy, p["snarl"])
    # White fangs
    put_px(body, 33, 13 + dy, p["fang"])
    put_px(body, 31, 13 + dy, p["fang"])
    put_px(body, 32, 14 + dy, p["fang"])

    return compose(shadow, body, outline_color=p["outline"])


# ---------------------------------------------------------------------------
# T3: Occult Cultist (32x32)
# ---------------------------------------------------------------------------

def draw_cultist() -> Image.Image:
    p = T3_CULTIST
    shadow = new_canvas(32, 32)
    body = new_canvas(32, 32)
    glow = new_canvas(32, 32)
    d_sh = ImageDraw.Draw(shadow)
    d = ImageDraw.Draw(body)
    d_gl = ImageDraw.Draw(glow)

    # Layer 0: Compact shadow showing 2px levitation
    d_sh.ellipse([(9, 28), (23, 31)], fill=SHADOW_DEFAULT)

    # Layer 1: Trapezoidal Indigo Robe (12, 14) .. (9, 27)-(23, 27)
    d.polygon([(12, 14), (20, 14), (23, 27), (9, 27)], fill=p["robe"])
    # Central fold #4A327D & side shadow
    d.polygon([(15, 14), (17, 14), (19, 27), (13, 27)], fill=p["robe_fold"])
    d.line([(20, 14), (23, 27)], fill=p["robe_dark"])
    # Sleeves
    d.polygon([(12, 15), (8, 19), (10, 21), (13, 18)], fill=p["robe"])
    d.polygon([(20, 15), (24, 18), (23, 20), (19, 18)], fill=p["robe"])

    # Pointed Hood with apex (16, 4) & deep shadow [(13, 9), (19, 14)]
    d.polygon([(16, 4), (21, 11), (20, 15), (12, 15), (11, 11)], fill=p["robe"])
    d.line([(16, 4), (11, 11)], fill=p["robe_fold"])
    d.line([(16, 4), (16, 8)], fill=p["robe_fold"])
    d.rectangle([(13, 9), (19, 14)], fill=p["hood_shadow"])
    # Twin turquoise abyss eyes
    put_px(body, 14, 11, p["neon"])
    put_px(body, 18, 11, p["neon"])
    put_px(glow, 14, 11, p["core"])
    put_px(glow, 18, 11, p["core"])

    # Layer 2: Wooden Staff & Hovering Pulsing Neon Orb
    d.line([(25, 13), (25, 27)], fill=p["wood"], width=2)
    d.line([(25, 13), (25, 26)], fill=p["wood_light"])
    # Staff cradle prongs
    put_px(body, 24, 12, p["wood_light"])
    put_px(body, 27, 12, p["wood_light"])

    # Hovering magic sphere #06D6A0 + white core #E0FBFC + translucent aura
    d_gl.ellipse([(21, 4), (29, 12)], fill=p["aura"])
    d.ellipse([(23, 6), (27, 10)], fill=p["neon"])
    d.rectangle([(25, 7), (25, 8)], fill=p["core"])
    put_px(body, 24, 7, p["core"])

    return compose(shadow, body, glow)


# ---------------------------------------------------------------------------
# T4: Ice Golem (36x36)
# ---------------------------------------------------------------------------

def draw_golem() -> Image.Image:
    p = T4_GOLEM
    shadow = new_canvas(36, 36)
    body = new_canvas(36, 36)
    glow = new_canvas(36, 36)
    d_sh = ImageDraw.Draw(shadow)
    d = ImageDraw.Draw(body)
    d_gl = ImageDraw.Draw(glow)

    # Layer 0: Wide contact shadow
    d_sh.ellipse([(6, 32), (30, 35)], fill=SHADOW_DEFAULT)

    # Layer 1: Basalt Leg Pillars [(9, 25), (15, 33)] & [(21, 25), (27, 33)]
    for x0, x1 in ((9, 15), (21, 27)):
        d.rectangle([(x0, 25), (x1, 33)], fill=p["basalt"])
        d.line([(x0, 25), (x0, 32)], fill=p["basalt_light"])
        d.line([(x1, 25), (x1, 33)], fill=p["basalt_dark"])

    # Monolithic Basalt Torso [(8, 13), (28, 26)]
    d.rectangle([(8, 13), (28, 26)], fill=p["basalt"])
    d.line([(8, 13), (28, 13)], fill=p["basalt_light"])
    d.line([(8, 13), (8, 25)], fill=p["basalt_light"])
    d.line([(28, 14), (28, 26)], fill=p["basalt_dark"])
    d.line([(8, 26), (28, 26)], fill=p["basalt_dark"])

    # Heavy basalt boulder fists
    d.rectangle([(3, 16), (8, 27)], fill=p["basalt"])
    d.line([(3, 16), (3, 26)], fill=p["basalt_light"])
    d.rectangle([(28, 16), (33, 27)], fill=p["basalt"])
    d.line([(33, 16), (33, 27)], fill=p["basalt_dark"])

    # Zigzag icy chest crack (#48CAE4)
    crack_pts = [(18, 14), (15, 17), (20, 20), (16, 23), (19, 25)]
    d.line(crack_pts, fill=p["ice"], width=2)
    d.line([(18, 15), (16, 17), (19, 20), (17, 23)], fill=p["ice_core"])

    # Layer 2: Shoulder Ice Stalagmites
    # Left: (6, 13) -> (9, 6) -> (12, 13); Right: mirrored (24, 13) -> (27, 6) -> (30, 13)
    d.polygon([(6, 13), (9, 6), (12, 13)], fill=p["ice"])
    d.polygon([(8, 13), (9, 7), (10, 13)], fill=p["ice_mid"])
    put_px(body, 9, 7, p["ice_core"])
    d_gl.polygon([(5, 13), (9, 5), (13, 13)], fill=p["ice_glow"])

    d.polygon([(24, 13), (27, 6), (30, 13)], fill=p["ice"])
    d.polygon([(26, 13), (27, 7), (28, 13)], fill=p["ice_mid"])
    put_px(body, 27, 7, p["ice_core"])
    d_gl.polygon([(23, 13), (27, 5), (31, 13)], fill=p["ice_glow"])

    # Basalt Cube Head with Cyclopean Ice Eye
    d.rectangle([(13, 7), (23, 14)], fill=p["basalt"])
    d.line([(13, 7), (23, 7)], fill=p["basalt_light"])
    d.line([(23, 8), (23, 14)], fill=p["basalt_dark"])
    # Wide cyclopean eye #48CAE4 + white core #E0FBFC
    d.rectangle([(15, 9), (21, 11)], fill=p["ice"])
    d.rectangle([(17, 9), (19, 11)], fill=p["ice_core"])
    d_gl.ellipse([(14, 8), (22, 12)], fill=p["ice_glow"])

    return compose(shadow, body, glow)


# ---------------------------------------------------------------------------
# T5: Kamikaze Imp (32x32)
# ---------------------------------------------------------------------------

def draw_kamikaze() -> Image.Image:
    p = T5_KAMIKAZE
    shadow = new_canvas(32, 32)
    body = new_canvas(32, 32)
    glow = new_canvas(32, 32)
    d_sh = ImageDraw.Draw(shadow)
    d = ImageDraw.Draw(body)
    d_gl = ImageDraw.Draw(glow)

    # Layer 0: Shifted ground shadow
    d_sh.ellipse([(8, 28), (24, 31)], fill=SHADOW_DEFAULT)

    # Layer 1: Cast-Iron Bomb on back [(7, 8), (21, 22)]
    d.ellipse([(7, 8), (21, 22)], fill=p["iron"])
    # Specular iron highlight #3A2A38
    d.arc([(9, 10), (19, 20)], start=190, end=290, fill=p["iron_spec"], width=2)
    d.rectangle([(12, 7), (16, 9)], fill=p["iron_spec"])  # fuse collar

    # Curved hemp fuse: (14, 8) -> (17, 3) -> (20, 4)
    d.line([(14, 8), (15, 5), (17, 3), (20, 4)], fill=p["fuse"], width=1)

    # Sparkling fuse fire at (20, 4): #FF5400 radial + #FFE600 core
    d_gl.ellipse([(17, 1), (24, 8)], fill=p["spark_glow"])
    for sx, sy in ((20, 2), (20, 6), (18, 4), (22, 4), (19, 3), (21, 5), (21, 3), (19, 5)):
        put_px(body, sx, sy, p["spark_outer"])
    d.rectangle([(20, 3), (21, 4)], fill=p["spark_core"])
    put_px(glow, 20, 4, (255, 255, 255, 255))

    # Layer 2: Forward-leaning Crimson Imp (#E63946)
    # Fast scurrying legs
    d.line([(13, 24), (10, 28)], fill=p["imp_dark"], width=2)
    d.line([(17, 24), (21, 28)], fill=p["imp"], width=2)
    # Hunched crimson torso leaning right
    d.polygon([(12, 20), (20, 17), (23, 23), (14, 25)], fill=p["imp"])
    d.line([(14, 24), (22, 23)], fill=p["imp_dark"])
    # Arms clutching bomb strap
    d.line([(18, 19), (14, 17)], fill=p["imp_light"], width=2)
    # Horned head leaning forward
    d.ellipse([(18, 13), (26, 20)], fill=p["imp"])
    d.line([(19, 14), (22, 13)], fill=p["imp_light"])
    # Imp horns
    d.line([(19, 13), (17, 10)], fill=p["horn"], width=2)
    d.line([(23, 13), (25, 10)], fill=p["horn"], width=2)
    # Bulging crazed eyes
    d.rectangle([(20, 15), (22, 17)], fill=p["eye_white"])
    d.rectangle([(24, 15), (26, 17)], fill=p["eye_white"])
    put_px(body, 21, 16, p["pupil"])
    put_px(body, 25, 16, p["pupil"])
    # Manic grin
    d.line([(21, 19), (25, 19)], fill=p["pupil"])

    return compose(shadow, body, glow)


# ---------------------------------------------------------------------------
# T6: Shadow Demon (32x32)
# ---------------------------------------------------------------------------

def draw_shadow_demon() -> Image.Image:
    p = T6_SHADOW_DEMON
    shadow = new_canvas(32, 32)
    body = new_canvas(32, 32)
    glow = new_canvas(32, 32)
    d_sh = ImageDraw.Draw(shadow)
    d = ImageDraw.Draw(body)
    d_gl = ImageDraw.Draw(glow)

    # Layer 0: Light diffused shadow
    d_sh.ellipse([(9, 27), (23, 30)], fill=p["shadow"])

    # Layer 1: Membranous Dark-Ether Wings with Purple Bone Rays & Crimson Accent
    # Left wing
    left_wing = [(13, 13), (4, 6), (2, 14), (6, 19), (12, 17)]
    d.polygon(left_wing, fill=p["ether"])
    d.line([(13, 13), (4, 6)], fill=p["bone_violet"], width=1)
    d.line([(13, 14), (2, 14)], fill=p["bone_violet"], width=1)
    d.line([(12, 16), (6, 19)], fill=p["bone_violet"], width=1)
    d.line([(4, 6), (2, 14), (6, 19)], fill=p["wing_crimson"])
    d_gl.line([(4, 6), (2, 14), (6, 19)], fill=p["wing_glow"], width=2)

    # Right wing
    right_wing = [(19, 13), (28, 6), (30, 14), (26, 19), (20, 17)]
    d.polygon(right_wing, fill=p["ether"])
    d.line([(19, 13), (28, 6)], fill=p["bone_violet"], width=1)
    d.line([(19, 14), (30, 14)], fill=p["bone_violet"], width=1)
    d.line([(20, 16), (26, 19)], fill=p["bone_violet"], width=1)
    d.line([(28, 6), (30, 14), (26, 19)], fill=p["wing_crimson"])
    d_gl.line([(28, 6), (30, 14), (26, 19)], fill=p["wing_glow"], width=2)

    # Floating torso tapering into a curling smoky tail (above ground)
    d.polygon([(12, 12), (20, 12), (18, 21), (14, 21)], fill=p["smoke_dark"])
    d.polygon([(14, 20), (18, 20), (19, 24), (15, 26), (12, 24)], fill=p["ether"])
    put_px(body, 14, 26, p["bone_violet"])
    put_px(body, 17, 25, p["smoke_dark"])

    # Layer 2: Thin Horned Skull Mask & Slanted Crimson Eyes (#FF0054)
    d.polygon([(13, 6), (19, 6), (18, 13), (16, 15), (14, 13)], fill=p["mask"])
    d.line([(13, 6), (11, 3)], fill=p["mask_rim"])
    d.line([(19, 6), (21, 3)], fill=p["mask_rim"])
    # Slanted crimson eye slits
    d.line([(13, 9), (15, 10)], fill=p["wing_crimson"])
    d.line([(19, 9), (17, 10)], fill=p["wing_crimson"])

    return compose(shadow, body, glow)


# ---------------------------------------------------------------------------
# T7: Fire Elemental (32x32)
# ---------------------------------------------------------------------------

def draw_fire_elemental() -> Image.Image:
    p = T7_FIRE_ELEMENTAL
    shadow = new_canvas(32, 32)
    body = new_canvas(32, 32)
    glow = new_canvas(32, 32)
    d_sh = ImageDraw.Draw(shadow)
    d = ImageDraw.Draw(body)
    d_gl = ImageDraw.Draw(glow)

    # Layer 0: Warm thermal orange glow on floor
    d_sh.ellipse([(7, 26), (25, 31)], fill=p["floor_glow"])

    # Ambient fire aura
    d_gl.ellipse([(6, 6), (26, 28)], fill=p["aura"])

    # Layer 1: Outer Crimson-Red Flame Teardrop (#D00000) with flickering tongues
    outer_poly = [
        (16, 3),
        (20, 9),
        (23, 7),   # side flame lick
        (24, 15),
        (25, 22),
        (21, 27),
        (11, 27),
        (7, 22),
        (8, 14),
        (9, 8),    # left flame lick
        (12, 10),
    ]
    d.polygon(outer_poly, fill=p["outer"])
    # Floating ember sparks
    put_px(glow, 11, 4, p["mid"])
    put_px(glow, 22, 4, p["inner"])

    # Layer 2: Mid Bright Orange Flame (#F48C06)
    mid_poly = [
        (16, 7),
        (20, 13),
        (22, 20),
        (19, 25),
        (13, 25),
        (10, 20),
        (12, 13),
    ]
    d.polygon(mid_poly, fill=p["mid"])

    # Inner Golden Flame (#FFBA08) & White-Gold Hot Core (#FFF3B0)
    d.ellipse([(12, 13), (20, 24)], fill=p["inner"])
    d.ellipse([(13, 16), (19, 23)], fill=p["core"])

    # Two floating bright white eyes inside the flame
    d.rectangle([(13, 15), (14, 16)], fill=p["eye"])
    d.rectangle([(18, 15), (19, 16)], fill=p["eye"])

    return compose(shadow, body, glow, outline_color=p["outline"])


# ---------------------------------------------------------------------------
# T8: Reaper Titan / Bringer of Death (64x64)
# ---------------------------------------------------------------------------

def draw_bringer_death() -> Image.Image:
    p = T8_REAPER
    shadow = new_canvas(64, 64)
    body = new_canvas(64, 64)
    glow = new_canvas(64, 64)
    d_sh = ImageDraw.Draw(shadow)
    d = ImageDraw.Draw(body)
    d_gl = ImageDraw.Draw(glow)

    # Layer 0: Huge contact shadow [(16, 56), (48, 62)]
    d_sh.ellipse([(16, 56), (48, 62)], fill=p["shadow"])

    # Layer 1: Monumental Void Mantle (#0B0813, ~44px tall: Y=12..56)
    cloak_poly = [
        (22, 18),
        (42, 18),
        (48, 55),
        (16, 55),
    ]
    d.polygon(cloak_poly, fill=p["cloak"])
    # Subtle monolithic folds
    d.polygon([(26, 20), (30, 20), (26, 53), (21, 53)], fill=p["cloak_fold"])
    d.polygon([(34, 20), (38, 20), (42, 53), (37, 53)], fill=p["cloak_fold"])
    d.line([(22, 18), (16, 54)], fill=p["cloak_rim"])
    d.line([(42, 18), (48, 54)], fill=p["cloak_rim"])

    # Wide shoulder mantle & arms reaching toward scythe
    d.polygon([(14, 22), (22, 17), (42, 17), (50, 22), (47, 32), (17, 32)], fill=p["cloak"])
    d.line([(15, 22), (22, 17), (42, 17), (49, 22)], fill=p["cloak_rim"])
    # Right sleeve gripping scythe shaft at X=44
    d.polygon([(40, 24), (47, 28), (46, 35), (38, 32)], fill=p["cloak_fold"])
    d.rectangle([(42, 29), (46, 33)], fill=p["mask_shadow"])  # skeletal hand

    # Tattered hem with pixel dithering dissolve (Y=51..57)
    for x in range(16, 49):
        for y in range(52, 56):
            if (x + y) % 2 == 0 and y >= 54:
                put_px(body, x, y, (0, 0, 0, 0))
            elif (x * 3 + y) % 5 == 0 and y >= 52:
                put_px(body, x, y, (0, 0, 0, 0))
        if x % 3 == 0:
            put_px(body, x, 56, p["cloak"])

    # Deep Hood & Bone Mask (#E5DECE)
    d.polygon([(32, 6), (42, 14), (40, 24), (24, 24), (22, 14)], fill=p["cloak"])
    d.line([(32, 6), (22, 14)], fill=p["cloak_rim"])
    d.line([(32, 6), (42, 14)], fill=p["cloak_rim"])
    # Bone mask protruding from hood
    d.polygon([(27, 13), (37, 13), (36, 21), (32, 24), (28, 21)], fill=p["mask"])
    d.line([(28, 21), (32, 24), (36, 21)], fill=p["mask_shadow"])
    # Eye sockets & chilling cyan eyes (#48CAE4)
    d.rectangle([(28, 15), (31, 18)], fill=p["cloak"])
    d.rectangle([(33, 15), (36, 18)], fill=p["cloak"])
    d.rectangle([(29, 16), (30, 17)], fill=p["eye"])
    d.rectangle([(34, 16), (35, 17)], fill=p["eye"])
    put_px(glow, 29, 16, p["eye_core"])
    put_px(glow, 34, 16, p["eye_core"])

    # Layer 2: Scythe of Death
    # Ebony shaft from (44, 58) to (44, 10), 3px thick (X=43..45)
    d.rectangle([(43, 10), (45, 58)], fill=p["ebony"])
    d.line([(43, 10), (43, 58)], fill=p["ebony_light"])
    # Ferrule cap & collar
    d.rectangle([(42, 9), (46, 12)], fill=p["blade_steel"])
    d.rectangle([(42, 56), (46, 58)], fill=p["blade_steel"])

    # Colossal crescent moon blade from (44, 14) curving to (13, 14) and sweeping tip
    blade_outer = [
        (44, 10),
        (36, 7),
        (25, 7),
        (16, 11),
        (10, 18),
        (13, 14),
        (21, 12),
        (32, 12),
        (43, 15),
    ]
    d.polygon(blade_outer, fill=p["blade_steel"])

    # Neon-cyan soul-flame cutting edge (#48CAE4 + #E0FBFC)
    cutting_edge = [(43, 15), (32, 12), (21, 12), (13, 14), (10, 18)]
    d.line(cutting_edge, fill=p["blade_neon"], width=2)
    d.line([(42, 14), (32, 11), (21, 11), (13, 14)], fill=p["blade_core"], width=1)

    # Translucent neon glow trail (d_gl) along crescent edge
    d_gl.line(cutting_edge, fill=p["blade_glow"], width=5)
    d_gl.ellipse([(8, 15), (15, 21)], fill=p["blade_glow"])

    return compose(shadow, body, glow, outline_color=p["outline"])


# ---------------------------------------------------------------------------
# Batch Generation Pipeline
# ---------------------------------------------------------------------------

def save_to_dirs(img: Image.Image, filename: str, dirs: list[Path]) -> list[Path]:
    paths: list[Path] = []
    for d in dirs:
        d.mkdir(parents=True, exist_ok=True)
        path = d / filename
        img.save(path, format="PNG")
        paths.append(path)
    return paths


def generate_all(out_root: Path) -> list[Path]:
    mobs_dir = out_root / "assets" / "mobs"
    sprites_dir = out_root / "assets" / "sprites"
    horde_dir = out_root / "assets" / "monsters" / "horde"
    out_dirs = [mobs_dir, sprites_dir, horde_dir]

    saved: list[Path] = []

    # T1: Skeleton Footman (32x32)
    saved.extend(save_to_dirs(draw_skeleton(), "mob_skeleton.png", out_dirs))
    saved.extend(save_to_dirs(draw_skeleton(), "mob_skeleton_white.png", out_dirs))
    saved.extend(save_to_dirs(draw_skeleton(), "mob_tier01_skeleton.png", out_dirs))

    # T2: Infernal Hound (4 frames 36x24 + strip 144x24)
    hound_frames = [draw_hound(i) for i in range(4)]
    for idx, frame in enumerate(hound_frames):
        saved.extend(save_to_dirs(frame, f"mob_hound_{idx}.png", out_dirs))
    saved.extend(save_to_dirs(hound_frames[0], "mob_hound.png", out_dirs))
    saved.extend(save_to_dirs(hound_frames[0], "mob_tier02_hound.png", out_dirs))
    saved.extend(save_to_dirs(make_strip(hound_frames), "mob_hound_strip.png", out_dirs))

    # T3–T8 static archetypes
    saved.extend(save_to_dirs(draw_cultist(), "mob_cultist.png", out_dirs))
    saved.extend(save_to_dirs(draw_cultist(), "mob_tier03_cultist.png", out_dirs))

    saved.extend(save_to_dirs(draw_golem(), "mob_golem.png", out_dirs))
    saved.extend(save_to_dirs(draw_golem(), "mob_tier04_ice_golem.png", out_dirs))

    saved.extend(save_to_dirs(draw_kamikaze(), "mob_kamikaze.png", out_dirs))
    saved.extend(save_to_dirs(draw_kamikaze(), "mob_tier05_kamikaze.png", out_dirs))

    saved.extend(save_to_dirs(draw_shadow_demon(), "mob_shadow_demon.png", out_dirs))
    saved.extend(save_to_dirs(draw_shadow_demon(), "mob_tier06_shadow_demon.png", out_dirs))

    saved.extend(save_to_dirs(draw_fire_elemental(), "mob_fire_elemental.png", out_dirs))
    saved.extend(save_to_dirs(draw_fire_elemental(), "mob_tier07_fire_elemental.png", out_dirs))

    saved.extend(save_to_dirs(draw_bringer_death(), "mob_bringer_death.png", out_dirs))
    saved.extend(save_to_dirs(draw_bringer_death(), "mob_tier08_bringer_death.png", out_dirs))

    return saved


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Generate Infernal Horde (Tiers 1–8) RGBA pixel-art sprites."
    )
    parser.add_argument(
        "--out-root",
        type=Path,
        default=Path("."),
        help="Project root directory where assets/mobs/ and assets/sprites/ will be written.",
    )
    args = parser.parse_args()

    files = generate_all(args.out_root)
    print(f"Generated {len(files)} PNG files under {(args.out_root / 'assets').resolve()}")


if __name__ == "__main__":
    main()
