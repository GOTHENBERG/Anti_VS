#!/usr/bin/env python3
"""Procedural pixel-art sprite generator for The High Legion (Tiers 9–16).

Generates all 8 elite High Legion archetypes (36x36 up to 72x72 Avatar of
Oblivion) in clean RGBA PNG format with contact shadows, 1px contrast outlines
(#0C0A12), and glowing ethereal auras, ready for Rust (`textures.rs`).

Usage:
    python scripts/generate_legion.py
    python scripts/generate_legion.py --out-root .
"""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Tuple

from PIL import Image, ImageDraw

RGBA = Tuple[int, int, int, int]

SHADOW_DEFAULT: RGBA = (0, 0, 0, 120)
OUTLINE_DARK: RGBA = (12, 10, 18, 255)      # #0C0A12


# ---------------------------------------------------------------------------
# Color Palettes (Tiers 9–16)
# ---------------------------------------------------------------------------

T9_PHALANX = {
    "steel": (30, 35, 42, 255),             # #1E232A
    "steel_light": (118, 134, 156, 255),    # #76869C
    "steel_dark": (16, 19, 24, 255),
    "gold": (200, 150, 62, 255),            # #C8963E
    "gold_bright": (255, 215, 96, 255),
    "bone": (244, 235, 217, 255),
    "soul": (6, 214, 160, 255),             # #06D6A0
    "soul_core": (224, 251, 252, 255),      # #E0FBFC
    "soul_glow": (6, 214, 160, 90),
    "wood": (78, 52, 34, 255),
}

T10_CERBERUS = {
    "fur": (22, 16, 25, 255),               # #161019
    "undercoat": (74, 21, 37, 255),         # #4A1525
    "lava_red": (217, 4, 41, 255),          # #D90429
    "lava_orange": (255, 158, 0, 255),      # #FF9E00
    "lava_yellow": (255, 224, 71, 255),
    "fang": (255, 255, 255, 255),
    "glow": (255, 120, 0, 95),
}

T11_VOID_CULTIST = {
    "robe_dark": (26, 11, 46, 255),         # #1A0B2E
    "robe_mid": (59, 20, 102, 255),         # #3B1466
    "robe_smoke": (42, 16, 74, 195),
    "gold": (212, 175, 55, 255),            # #D4AF37
    "crown": (255, 215, 0, 255),            # #FFD700
    "skull": (235, 212, 148, 255),
    "skull_shadow": (164, 134, 68, 255),
    "eye": (181, 23, 158, 255),             # #B5179E
    "rune": (247, 37, 133, 255),            # #F72585
    "rune_core": (255, 200, 230, 255),
    "aura": (181, 23, 158, 95),
    "star_core": (10, 6, 16, 255),
    "arc": (114, 9, 183, 255),
}

T12_COLOSSUS = {
    "obs": (18, 20, 26, 255),               # #12141A
    "facet": (75, 85, 107, 255),            # #4B556B
    "obs_deep": (9, 10, 14, 255),
    "magma": (255, 60, 0, 255),             # #FF3C00
    "magma_hot": (255, 160, 0, 255),        # #FFAA00
    "magma_core": (255, 245, 210, 255),
    "heat_glow": (255, 72, 0, 90),
}

T13_NUKE_BOMBER = {
    "wing": (28, 35, 26, 255),              # #1C231A
    "wing_rib": (54, 68, 50, 255),
    "bone": (214, 206, 188, 255),
    "iron": (44, 50, 58, 255),
    "iron_light": (108, 120, 136, 255),
    "acid": (56, 176, 0, 255),              # #38B000
    "lime": (112, 224, 0, 255),             # #70E000
    "neon": (204, 255, 51, 255),            # #CCFF33
    "cloud": (112, 224, 0, 80),
}

T14_STALKER = {
    "chitin": (15, 8, 29, 255),             # #0F081D
    "chitin_hi": (44, 22, 76, 255),
    "violet": (114, 9, 183, 255),           # #7209B7
    "magenta": (247, 37, 133, 255),         # #F72585
    "blade_core": (255, 190, 225, 255),
    "phase_glow": (247, 37, 133, 85),
    "shadow": (0, 0, 0, 85),
}

T15_SOLAR_ARCHON = {
    "obs": (20, 18, 24, 255),               # #141218
    "obs_hi": (48, 44, 58, 255),
    "gold_rim": (255, 170, 0, 255),         # #FFAA00
    "gold_armor": (212, 175, 55, 255),      # #D4AF37
    "gold_hi": (255, 232, 130, 255),
    "ruby": (158, 0, 37, 255),              # #9E0025
    "plasma_red": (255, 60, 0, 255),        # #FF3C00
    "plasma_gold": (255, 208, 0, 255),      # #FFD000
    "white": (255, 255, 255, 255),
    "halo_glow": (255, 170, 0, 100),
    "plasma_glow": (255, 96, 0, 95),
}

T16_OBLIVION = {
    "void": (7, 4, 13, 255),                # #07040D
    "plate": (22, 17, 38, 255),
    "plate_hi": (54, 46, 86, 255),
    "violet": (114, 9, 183, 255),           # #7209B7
    "aqua": (76, 201, 240, 255),            # #4CC9F0
    "aqua_core": (224, 251, 252, 255),
    "mask": (210, 214, 226, 255),
    "black_hole": (0, 0, 0, 255),
    "disk_glow": (76, 201, 240, 100),
    "void_glow": (114, 9, 183, 95),
    "shadow": (0, 0, 0, 150),
}


# ---------------------------------------------------------------------------
# Shared Pixel-Art Helpers
# ---------------------------------------------------------------------------

def new_canvas(w: int, h: int) -> Image.Image:
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
    glow: Image.Image | None = None,
    outline_color: RGBA = OUTLINE_DARK,
) -> Image.Image:
    outlined = apply_1px_outline(body, outline_color)
    img = Image.alpha_composite(shadow, outlined)
    if glow is not None:
        img = Image.alpha_composite(img, glow)
    return img


# ---------------------------------------------------------------------------
# T9: Dread Phalanx (36x36)
# ---------------------------------------------------------------------------

def draw_tier09_dread_phalanx() -> Image.Image:
    p = T9_PHALANX
    shadow = new_canvas(36, 36)
    body = new_canvas(36, 36)
    glow = new_canvas(36, 36)
    d_sh = ImageDraw.Draw(shadow)
    d = ImageDraw.Draw(body)
    d_gl = ImageDraw.Draw(glow)

    # Layer 0: Wide contact shadow [(7, 30), (29, 34)]
    d_sh.ellipse([(7, 30), (29, 34)], fill=SHADOW_DEFAULT)

    # Layer 1: Greaves & Sabatons [(16, 25), (20, 32)] and [(22, 25), (26, 32)]
    for x0, x1 in ((16, 20), (22, 26)):
        d.rectangle([(x0, 25), (x1, 32)], fill=p["steel"])
        d.line([(x0 + 1, 25), (x0 + 1, 31)], fill=p["steel_light"])
        d.line([(x1, 25), (x1, 32)], fill=p["steel_dark"])
        d.line([(x0, 31), (x1, 31)], fill=p["gold"])

    # Heavy Dark-Steel Cuirass with Engraved Crossed Bones
    d.rectangle([(15, 13), (26, 25)], fill=p["steel"])
    d.line([(16, 14), (25, 14)], fill=p["steel_light"])
    d.line([(26, 13), (26, 25)], fill=p["steel_dark"])
    # Engraved crossed bones
    d.line([(17, 16), (24, 22)], fill=p["bone"])
    d.line([(24, 16), (17, 22)], fill=p["bone"])
    # Belt & right pauldron
    d.rectangle([(15, 23), (26, 24)], fill=p["gold"])
    d.rectangle([(24, 13), (28, 18)], fill=p["steel"])
    d.line([(24, 13), (28, 13)], fill=p["gold"])

    # Gladiator Helmet Dome [(17, 6), (25, 13)] + Golden Crest Plume
    d.rectangle([(17, 6), (25, 13)], fill=p["steel"])
    d.line([(18, 7), (23, 7)], fill=p["steel_light"])
    # Golden crest plume on top
    d.polygon([(19, 6), (20, 2), (24, 2), (26, 5), (23, 6)], fill=p["gold"])
    d.line([(20, 2), (24, 2)], fill=p["gold_bright"])
    # Thin horizontal visor slit with turquoise soul sparks
    d.line([(18, 10), (24, 10)], fill=p["steel_dark"])
    put_px(body, 19, 10, p["soul"])
    put_px(body, 23, 10, p["soul"])
    put_px(glow, 19, 10, p["soul_core"])
    put_px(glow, 23, 10, p["soul_core"])

    # Barbed Pike at X=28, Y=5..33
    d.line([(28, 7), (28, 33)], fill=p["wood"], width=2)
    # Three-edged spearhead + shield-hooking barbs
    d.polygon([(28, 1), (31, 7), (26, 7)], fill=p["steel_light"])
    d.line([(28, 2), (28, 7)], fill=p["soul_core"])
    d.line([(30, 7), (32, 9)], fill=p["steel_light"])
    d.line([(26, 8), (24, 10)], fill=p["steel_light"])

    # Layer 2: Tower Shield [(4, 10), (15, 32)]
    d.rectangle([(4, 10), (15, 32)], fill=p["steel"], outline=p["gold"])
    d.rectangle([(5, 11), (14, 31)], outline=p["steel_dark"])
    # Gold rivets along shield border
    for ry in (11, 16, 21, 26, 31):
        put_px(body, 4, ry, p["gold_bright"])
        put_px(body, 15, ry, p["gold_bright"])
    # Raised round umbo with skull & glowing turquoise sockets
    d.ellipse([(6, 17), (13, 25)], fill=p["steel_light"], outline=p["gold"])
    d.rectangle([(8, 19), (11, 23)], fill=p["bone"])
    put_px(body, 8, 20, p["soul"])
    put_px(body, 11, 20, p["soul"])
    d_gl.ellipse([(7, 19), (12, 22)], fill=p["soul_glow"])

    return compose(shadow, body, glow)


# ---------------------------------------------------------------------------
# T10: Cerberus Hellhound (48x32)
# ---------------------------------------------------------------------------

def draw_tier10_hellhound() -> Image.Image:
    p = T10_CERBERUS
    shadow = new_canvas(48, 32)
    body = new_canvas(48, 32)
    glow = new_canvas(48, 32)
    d_sh = ImageDraw.Draw(shadow)
    d = ImageDraw.Draw(body)
    d_gl = ImageDraw.Draw(glow)

    # Layer 0: Elongated contact shadow [(6, 26), (42, 31)]
    d_sh.ellipse([(6, 26), (42, 31)], fill=SHADOW_DEFAULT)

    # Layer 1: Pointed Tail with Fiery Tuft
    d.line([(10, 15), (6, 11), (3, 12)], fill=p["fur"], width=2)
    d.polygon([(1, 11), (4, 9), (5, 13), (2, 14)], fill=p["lava_red"])
    put_px(body, 3, 11, p["lava_orange"])
    put_px(body, 2, 12, p["lava_yellow"])
    d_gl.ellipse([(0, 8), (6, 15)], fill=p["glow"])

    # Four Muscular Legs with Glowing Molten-Basalt Claws
    legs = [
        [(11, 18), (9, 24), (11, 28)],
        [(15, 18), (16, 24), (18, 28)],
        [(28, 18), (27, 24), (29, 28)],
        [(33, 18), (35, 24), (38, 28)],
    ]
    for leg in legs:
        d.line(leg, fill=p["fur"], width=3)
        d.line([leg[0], leg[1]], fill=p["undercoat"], width=1)
        cx, cy = leg[2]
        d.line([(cx - 1, cy), (cx + 2, cy)], fill=p["lava_orange"])
        put_px(body, cx + 2, cy, p["lava_yellow"])

    # Muscular Torso [(9, 15)..(36, 18)] with Dark-Crimson Undercoat
    torso = [(9, 15), (15, 10), (32, 10), (36, 15), (34, 20), (12, 19)]
    d.polygon(torso, fill=p["fur"])
    d.polygon([(13, 16), (33, 16), (32, 19), (14, 18)], fill=p["undercoat"])

    # Lava Spine Plates, Magmatic Cracks & Golden Spikes
    for sx in (13, 18, 23, 28):
        d.polygon([(sx, 11), (sx + 2, 7), (sx + 4, 11)], fill=p["lava_orange"])
        put_px(body, sx + 2, 7, p["lava_yellow"])
        d.line([(sx + 1, 11), (sx + 2, 14)], fill=p["lava_red"])

    # Layer 2: Three Ferocious Heads
    # 1) Upper head (reared up roaring near (34, 4))
    d.polygon([(29, 11), (32, 5), (38, 3), (40, 6), (35, 10)], fill=p["fur"])
    d.line([(36, 5), (39, 5)], fill=p["lava_orange"])
    put_px(body, 34, 6, p["lava_yellow"])
    put_px(body, 39, 4, p["fang"])

    # 2) Lower head (crouched low near (39, 15))
    d.polygon([(33, 16), (40, 15), (44, 19), (42, 22), (35, 20)], fill=p["fur"])
    d.line([(39, 19), (43, 20)], fill=p["lava_red"])
    put_px(body, 38, 17, p["lava_yellow"])
    put_px(body, 43, 19, p["fang"])
    put_px(body, 41, 21, p["fang"])

    # 3) Central dominant head [(32, 11), (46, 14)] with lava breath
    d.polygon([(31, 11), (36, 8), (45, 10), (46, 13), (35, 15)], fill=p["fur"])
    d.polygon([(35, 15), (45, 15), (44, 17), (34, 16)], fill=p["fur"])
    # Wide fiery jaws & lava breath
    d.rectangle([(39, 12), (45, 14)], fill=p["lava_red"])
    d.line([(40, 13), (46, 13)], fill=p["lava_orange"])
    put_px(body, 45, 13, p["lava_yellow"])
    d_gl.ellipse([(41, 10), (47, 16)], fill=p["glow"])
    # Glowing yellow eye & white fangs
    d.rectangle([(37, 10), (38, 11)], fill=p["lava_yellow"])
    put_px(body, 44, 12, p["fang"])
    put_px(body, 42, 12, p["fang"])
    put_px(body, 43, 15, p["fang"])

    return compose(shadow, body, glow)


# ---------------------------------------------------------------------------
# T11: Void Archcultist / Lich (36x40)
# ---------------------------------------------------------------------------

def draw_tier11_void_cultist() -> Image.Image:
    p = T11_VOID_CULTIST
    shadow = new_canvas(36, 40)
    body = new_canvas(36, 40)
    glow = new_canvas(36, 40)
    d_sh = ImageDraw.Draw(shadow)
    d = ImageDraw.Draw(body)
    d_gl = ImageDraw.Draw(glow)

    # Layer 0: Compact shadow [(10, 34), (26, 38)] showing 4px levitation
    d_sh.ellipse([(10, 34), (26, 38)], fill=SHADOW_DEFAULT)

    # Layer 1: Multi-layered Midnight & Royal Purple Robe with Smoky Tendrils
    d.polygon([(12, 14), (24, 14), (27, 30), (9, 30)], fill=p["robe_dark"])
    d.polygon([(14, 14), (22, 14), (24, 29), (12, 29)], fill=p["robe_mid"])
    # Dissolving smoky hem tendrils
    for tx, ty in ((10, 32), (14, 33), (18, 32), (22, 33), (25, 31)):
        d.line([(tx, 29), (tx, ty)], fill=p["robe_smoke"], width=2)
    # Sleeves
    d.polygon([(12, 15), (7, 20), (10, 22), (14, 19)], fill=p["robe_mid"])
    d.polygon([(24, 15), (29, 19), (28, 22), (23, 19)], fill=p["robe_mid"])

    # Golden Pallium on Chest (#D4AF37)
    d.rectangle([(16, 15), (20, 27)], fill=p["gold"])
    d.line([(13, 16), (23, 16)], fill=p["gold"])
    for ry in (18, 21, 24):
        put_px(body, 18, ry, p["rune"])

    # Exposed Gilded Skull & 3-Pronged Archmage Crown
    d.rectangle([(14, 7), (22, 14)], fill=p["skull"])
    d.line([(14, 13), (22, 13)], fill=p["skull_shadow"])
    # Neon-magenta burning eye sockets
    d.rectangle([(15, 9), (17, 11)], fill=p["star_core"])
    d.rectangle([(19, 9), (21, 11)], fill=p["star_core"])
    put_px(body, 16, 10, p["eye"])
    put_px(body, 20, 10, p["eye"])
    put_px(glow, 16, 10, p["rune"])
    put_px(glow, 20, 10, p["rune"])

    # 3-pronged golden crown with glowing gem
    d.rectangle([(13, 6), (23, 7)], fill=p["crown"])
    d.rectangle([(14, 3), (15, 6)], fill=p["crown"])
    d.rectangle([(17, 2), (19, 6)], fill=p["crown"])
    d.rectangle([(21, 3), (22, 6)], fill=p["crown"])
    put_px(body, 18, 5, p["rune"])

    # Layer 2: Floating Open Black Grimoire in Left Hand (X=2..10, Y=17..25)
    d_gl.ellipse([(1, 16), (11, 26)], fill=p["aura"])
    d.rectangle([(2, 17), (10, 25)], fill=p["star_core"], outline=p["gold"])
    d.line([(6, 18), (6, 24)], fill=p["gold"])
    # Pulsing bright magenta rune on pages
    d.line([(4, 19), (4, 23)], fill=p["rune"])
    d.line([(8, 19), (8, 23)], fill=p["rune"])
    d.line([(4, 21), (8, 21)], fill=p["rune_core"])

    # Tall Golden Staff & Floating Dark Star Crystal in Right Hand
    d.line([(30, 10), (30, 34)], fill=p["gold"], width=2)
    d.line([(28, 10), (32, 10)], fill=p["crown"])
    # Dark Star crystal
    d_gl.ellipse([(26, 2), (34, 10)], fill=p["aura"])
    d.polygon([(30, 3), (33, 6), (30, 9), (27, 6)], fill=p["star_core"], outline=p["arc"])
    put_px(body, 30, 6, p["rune"])
    # Arc discharges
    d.line([(27, 4), (25, 6)], fill=p["rune"])
    d.line([(33, 4), (34, 7)], fill=p["rune"])

    return compose(shadow, body, glow)


# ---------------------------------------------------------------------------
# T12: Obsidian Colossus (54x54)
# ---------------------------------------------------------------------------

def draw_tier12_obsidian_colossus() -> Image.Image:
    p = T12_COLOSSUS
    shadow = new_canvas(54, 54)
    body = new_canvas(54, 54)
    glow = new_canvas(54, 54)
    d_sh = ImageDraw.Draw(shadow)
    d = ImageDraw.Draw(body)
    d_gl = ImageDraw.Draw(glow)

    # Layer 0: Massive shadow [(8, 47), (46, 52)]
    d_sh.ellipse([(8, 47), (46, 52)], fill=SHADOW_DEFAULT)

    # Layer 1: 10px-Wide Pillar Legs with Faceted Highlights
    for x0, x1 in ((12, 22), (32, 42)):
        d.rectangle([(x0, 35), (x1, 49)], fill=p["obs"])
        d.polygon([(x0, 35), (x0 + 4, 35), (x0 + 2, 48), (x0, 48)], fill=p["facet"])
        d.line([(x1, 35), (x1, 49)], fill=p["obs_deep"])
        d.line([(x0 + 5, 38), (x0 + 7, 44)], fill=p["magma"])

    # Monolithic Torso (36px wide: X=9..45, Y=15..36)
    d.rectangle([(9, 15), (45, 36)], fill=p["obs"])
    d.polygon([(9, 15), (26, 15), (20, 22), (9, 26)], fill=p["facet"])
    d.line([(45, 15), (45, 36)], fill=p["obs_deep"])
    d.line([(9, 36), (45, 36)], fill=p["obs_deep"])

    # Solid Obsidian Hammer-Boulder Arms
    for x0, x1, hi_x in ((2, 11, 2), (43, 52, 44)):
        d.rectangle([(x0, 19), (x1, 41)], fill=p["obs"])
        d.line([(hi_x, 19), (hi_x, 39)], fill=p["facet"], width=2)
        d.rectangle([(x0, 33), (x1, 41)], fill=p["obs_deep"])
        d.line([(x0 + 3, 25), (x1 - 3, 31)], fill=p["magma"], width=2)

    # Zigzag Magma Fissures across Chest (#FF3C00, #FFA000, white core)
    fissure1 = [(27, 17), (22, 21), (30, 26), (24, 31), (28, 35)]
    fissure2 = [(27, 24), (35, 21), (39, 25)]
    fissure3 = [(24, 24), (16, 22), (14, 27)]
    for path in (fissure1, fissure2, fissure3):
        d_gl.line(path, fill=p["heat_glow"], width=6)
        d.line(path, fill=p["magma"], width=3)
        d.line(path, fill=p["magma_hot"], width=1)
    d.ellipse([(25, 23), (29, 27)], fill=p["magma_core"])

    # Layer 2: Sharp Obsidian Crystal Prisms on Shoulders
    for pts, hi in (
        ([(6, 16), (10, 5), (16, 16)], [(10, 5), (10, 15)]),
        ([(38, 16), (44, 5), (48, 16)], [(44, 5), (44, 15)]),
    ):
        d.polygon(pts, fill=p["obs"])
        d.line(hi, fill=p["facet"], width=2)
        d.line([pts[0], pts[1]], fill=p["facet"])

    # Recessed Monolithic Head Slab with Vertical Magmatic Cyclopean Eye
    d.rectangle([(19, 8), (35, 16)], fill=p["obs"])
    d.line([(19, 8), (35, 8)], fill=p["facet"])
    d_gl.ellipse([(22, 8), (32, 16)], fill=p["heat_glow"])
    d.rectangle([(26, 9), (28, 15)], fill=p["magma"])
    d.line([(27, 10), (27, 14)], fill=p["magma_hot"])
    put_px(body, 27, 12, p["magma_core"])

    return compose(shadow, body, glow)


# ---------------------------------------------------------------------------
# T13: Nuke Bomber / Tormentor (36x36)
# ---------------------------------------------------------------------------

def draw_tier13_nuke_bomber() -> Image.Image:
    p = T13_NUKE_BOMBER
    shadow = new_canvas(36, 36)
    body = new_canvas(36, 36)
    glow = new_canvas(36, 36)
    d_sh = ImageDraw.Draw(shadow)
    d = ImageDraw.Draw(body)
    d_gl = ImageDraw.Draw(glow)

    # Layer 0: Shadow [(9, 31), (27, 35)]
    d_sh.ellipse([(9, 31), (27, 35)], fill=SHADOW_DEFAULT)

    # Layer 1: Wide Leathery Gargoyle Wings (#1C231A) with Bone Joint Claws
    left_wing = [(13, 14), (4, 5), (2, 15), (7, 21), (13, 18)]
    right_wing = [(23, 14), (32, 5), (34, 15), (29, 21), (23, 18)]
    d.polygon(left_wing, fill=p["wing"])
    d.polygon(right_wing, fill=p["wing"])
    for a, b in (
        ((13, 14), (4, 5)), ((13, 15), (2, 15)), ((12, 17), (7, 21)),
        ((23, 14), (32, 5)), ((23, 15), (34, 15)), ((24, 17), (29, 21)),
    ):
        d.line([a, b], fill=p["wing_rib"])
    put_px(body, 4, 4, p["bone"])
    put_px(body, 32, 4, p["bone"])

    # Taloned Birdlike Legs & Hunched Torso
    d.line([(13, 28), (11, 33)], fill=p["wing_rib"], width=2)
    d.line([(23, 28), (25, 33)], fill=p["wing_rib"], width=2)
    d.line([(9, 33), (12, 33)], fill=p["bone"])
    d.line([(24, 33), (27, 33)], fill=p["bone"])
    d.rectangle([(12, 12), (24, 21)], fill=p["wing"])

    # Head in Iron Plague Mask with Glowing Green Goggles
    d.ellipse([(13, 6), (23, 15)], fill=p["iron"])
    d.polygon([(16, 12), (20, 12), (18, 17)], fill=p["iron_light"])
    d.rectangle([(14, 9), (16, 11)], fill=p["lime"])
    d.rectangle([(20, 9), (22, 11)], fill=p["lime"])
    put_px(body, 15, 10, p["neon"])
    put_px(body, 21, 10, p["neon"])

    # Layer 2: Radioactive Blight Tank [(11, 17), (25, 31)] strapped with chains
    d_gl.ellipse([(8, 15), (28, 33)], fill=p["cloud"])
    d.ellipse([(11, 17), (25, 31)], fill=p["acid"], outline=p["iron_light"])
    d.ellipse([(13, 20), (23, 29)], fill=p["lime"])
    d.ellipse([(14, 21), (17, 24)], fill=p["neon"])
    d.ellipse([(19, 24), (21, 26)], fill=p["neon"])
    put_px(body, 18, 20, p["neon"])
    # Iron chains
    d.line([(11, 24), (25, 24)], fill=p["iron"], width=2)
    d.line([(18, 17), (18, 31)], fill=p["iron"], width=2)
    for cx in (13, 16, 20, 23):
        put_px(body, cx, 24, p["iron_light"])

    return compose(shadow, body, glow)


# ---------------------------------------------------------------------------
# T14: Shadow Stalker (40x40)
# ---------------------------------------------------------------------------

def draw_tier14_shadow_stalker() -> Image.Image:
    p = T14_STALKER
    shadow = new_canvas(40, 40)
    body = new_canvas(40, 40)
    glow = new_canvas(40, 40)
    d_sh = ImageDraw.Draw(shadow)
    d = ImageDraw.Draw(body)
    d_gl = ImageDraw.Draw(glow)

    # Layer 0: Ghostly diffused shadow [(10, 34), (30, 38)]
    d_sh.ellipse([(10, 34), (30, 38)], fill=p["shadow"])

    # Layer 1: 4 Articulated Spider Legs (#0F081D) with Violet Spines (#7209B7)
    spider_legs = [
        [(16, 22), (10, 25), (12, 34)],
        [(17, 24), (13, 29), (16, 35)],
        [(24, 22), (30, 25), (28, 34)],
        [(23, 24), (27, 29), (24, 35)],
    ]
    for leg in spider_legs:
        d.line(leg, fill=p["chitin"], width=2)
        put_px(body, leg[1][0], leg[1][1] - 1, p["violet"])

    # Narrow Segmented Thorax & Dissolving Smoky Tail
    d.polygon([(16, 13), (24, 13), (22, 26), (18, 26)], fill=p["chitin"])
    for sy in (16, 19, 22):
        d.line([(17, sy), (23, sy)], fill=p["chitin_hi"])
    d.polygon([(18, 26), (22, 26), (21, 31), (19, 33)], fill=p["violet"])
    put_px(body, 20, 34, p["chitin_hi"])

    # Predatory Mantis Mask with 4 Faceted Electric-Magenta Eyes (#F72585)
    d.polygon([(16, 6), (24, 6), (23, 12), (20, 14), (17, 12)], fill=p["chitin"])
    d.line([(16, 6), (14, 3)], fill=p["violet"])
    d.line([(24, 6), (26, 3)], fill=p["violet"])
    for ex, ey in ((17, 9), (19, 10), (21, 10), (23, 9)):
        put_px(body, ex, ey, p["magenta"])
        put_px(glow, ex, ey, p["blade_core"])

    # Layer 2: Four Curved Phase Scythe-Blades
    upper_l = [(16, 15), (9, 8), (2, 16), (6, 13), (15, 17)]
    upper_r = [(24, 15), (31, 8), (38, 16), (34, 13), (25, 17)]
    for poly, edge in (
        (upper_l, [(15, 15), (9, 8), (2, 16)]),
        (upper_r, [(25, 15), (31, 8), (38, 16)]),
    ):
        d_gl.line(edge, fill=p["phase_glow"], width=4)
        d.polygon(poly, fill=p["violet"])
        d.line(edge, fill=p["magenta"], width=1)
        put_px(body, edge[2][0], edge[2][1], p["blade_core"])

    lower_l = [(16, 19), (7, 22), (5, 27), (10, 23), (16, 21)]
    lower_r = [(24, 19), (33, 22), (35, 27), (30, 23), (24, 21)]
    for poly, tip in ((lower_l, (5, 27)), (lower_r, (35, 27))):
        d.polygon(poly, fill=p["violet"])
        d.line([poly[0], poly[1], tip], fill=p["magenta"])

    return compose(shadow, body, glow)


# ---------------------------------------------------------------------------
# T15: Solar Archon / Fallen Seraph (48x54)
# ---------------------------------------------------------------------------

def draw_tier15_solar_archon() -> Image.Image:
    p = T15_SOLAR_ARCHON
    shadow = new_canvas(48, 54)
    body = new_canvas(48, 54)
    glow = new_canvas(48, 54)
    d_sh = ImageDraw.Draw(shadow)
    d = ImageDraw.Draw(body)
    d_gl = ImageDraw.Draw(glow)

    # Layer 0: Shadow far below [(16, 48), (32, 52)]
    d_sh.ellipse([(16, 48), (32, 52)], fill=SHADOW_DEFAULT)

    # Layer 1: Six Obsidian-Gold Seraph Wings
    wings = [
        ([(20, 18), (9, 4), (4, 12), (14, 20)], [(20, 18), (9, 4), (4, 12)]),
        ([(28, 18), (39, 4), (44, 12), (34, 20)], [(28, 18), (39, 4), (44, 12)]),
        ([(19, 23), (2, 18), (4, 28), (18, 27)], [(19, 23), (2, 18), (4, 28)]),
        ([(29, 23), (46, 18), (44, 28), (30, 27)], [(29, 23), (46, 18), (44, 28)]),
        ([(20, 29), (10, 34), (13, 44), (21, 36)], [(10, 34), (13, 44)]),
        ([(28, 29), (38, 34), (35, 44), (27, 36)], [(38, 34), (35, 44)]),
    ]
    for poly, rim in wings:
        d.polygon(poly, fill=p["obs"])
        d.line(rim, fill=p["gold_rim"], width=1)

    # Ruby Cloak Lining & Gold Cuirass
    d.polygon([(19, 20), (29, 20), (31, 42), (17, 42)], fill=p["ruby"])
    d.rectangle([(19, 19), (29, 32)], fill=p["gold_armor"])
    d.rectangle([(21, 20), (27, 28)], fill=p["gold_hi"])
    d.line([(24, 20), (24, 31)], fill=p["ruby"])
    # Armored legs
    d.rectangle([(20, 33), (23, 43)], fill=p["obs"])
    d.line([(21, 33), (21, 42)], fill=p["gold_armor"])
    d.rectangle([(25, 33), (28, 43)], fill=p["obs"])
    d.line([(26, 33), (26, 42)], fill=p["gold_armor"])
    d.rectangle([(15, 18), (20, 24)], fill=p["gold_armor"])
    d.rectangle([(28, 18), (33, 24)], fill=p["gold_armor"])

    # Faceless Helm with White Light Slit
    d.polygon([(20, 10), (28, 10), (29, 18), (24, 20), (19, 18)], fill=p["gold_armor"])
    d.rectangle([(21, 11), (27, 17)], fill=p["obs"])
    d.line([(21, 14), (27, 14)], fill=p["white"])
    d_gl.line([(20, 14), (28, 14)], fill=p["halo_glow"], width=3)

    # Layer 2: Broken Thorny Solar Crown-Halo
    d_gl.ellipse([(15, 2), (33, 9)], outline=p["halo_glow"], width=3)
    d.arc([(16, 3), (32, 9)], start=200, end=340, fill=p["gold_rim"], width=2)
    for tx, ty in ((18, 2), (24, 1), (30, 2)):
        put_px(body, tx, ty, p["white"])

    # Two-Handed Plasma Broadsword
    d_gl.polygon([(36, 7), (43, 7), (43, 33), (36, 33)], fill=p["plasma_glow"])
    d.polygon([(39, 6), (42, 10), (42, 31), (37, 31), (37, 10)], fill=p["plasma_red"])
    d.rectangle([(38, 9), (41, 30)], fill=p["plasma_gold"])
    d.line([(39, 8), (39, 30)], fill=p["white"], width=2)
    d.rectangle([(34, 31), (44, 33)], fill=p["gold_armor"])
    d.rectangle([(38, 33), (40, 39)], fill=p["obs_hi"])
    put_px(body, 39, 40, p["gold_rim"])

    return compose(shadow, body, glow)


# ---------------------------------------------------------------------------
# T16: Avatar of Oblivion (72x72)
# ---------------------------------------------------------------------------

def draw_tier16_avatar_oblivion() -> Image.Image:
    p = T16_OBLIVION
    shadow = new_canvas(72, 72)
    body = new_canvas(72, 72)
    glow = new_canvas(72, 72)
    d_sh = ImageDraw.Draw(shadow)
    d = ImageDraw.Draw(body)
    d_gl = ImageDraw.Draw(glow)

    # Layer 0: Huge void stain [(14, 63), (58, 70)]
    d_sh.ellipse([(14, 63), (58, 70)], fill=p["shadow"])
    d_gl.ellipse([(16, 62), (56, 69)], outline=p["void_glow"], width=2)

    # Layer 1: Cyclonic Void Vortex & 4 Tendrils
    vortex = [(24, 42), (48, 42), (44, 56), (36, 66), (28, 56)]
    d.polygon(vortex, fill=p["void"])
    d.arc([(24, 46), (48, 56)], start=10, end=170, fill=p["violet"], width=2)
    d.arc([(27, 53), (45, 61)], start=10, end=170, fill=p["aqua"], width=1)

    tentacles = [
        [(26, 46), (16, 48), (10, 56), (6, 52)],
        [(28, 52), (18, 58), (14, 64), (10, 62)],
        [(46, 46), (56, 48), (62, 56), (66, 52)],
        [(44, 52), (54, 58), (58, 64), (62, 62)],
    ]
    for t in tentacles:
        d.line(t, fill=p["void"], width=4)
        d.line(t, fill=p["plate_hi"], width=1)
        for sx, sy in t[1:3]:
            put_px(body, sx, sy - 2, p["aqua"])
            put_px(body, sx, sy + 2, p["violet"])

    # Cracked Astral Plate Torso
    d.polygon([(20, 22), (52, 22), (47, 44), (25, 44)], fill=p["plate"])
    d.line([(20, 22), (52, 22)], fill=p["plate_hi"], width=2)
    for ry in (26, 31, 36, 40):
        d.line([(22, ry), (29, ry + 1)], fill=p["violet"], width=2)
        d.line([(43, ry + 1), (50, ry)], fill=p["violet"], width=2)

    # Floating Segmented Stone Gauntlets with Aquamarine Claws
    for gx0, gx1, claw_dir in ((7, 17, -1), (55, 65, 1)):
        d.rectangle([(gx0, 26), (gx1, 36)], fill=p["plate"])
        d.line([(gx0, 26), (gx1, 26)], fill=p["plate_hi"])
        d.line([(gx0 + 2, 30), (gx1 - 2, 30)], fill=p["violet"])
        for cy in (37, 39):
            for cx in range(gx0 + 1, gx1, 3):
                d.line([(cx, 36), (cx + claw_dir, cy + 2)], fill=p["aqua"])

    # Layer 2: Chest Black Hole Singularity & Accretion Disk
    d_gl.ellipse([(24, 24), (48, 44)], fill=p["void_glow"])
    d_gl.ellipse([(27, 27), (45, 41)], outline=p["disk_glow"], width=3)
    d_gl.line([(21, 34), (51, 34)], fill=p["aqua"], width=2)
    d.ellipse([(29, 27), (43, 41)], fill=p["violet"])
    d.ellipse([(30, 28), (42, 40)], fill=p["aqua"])
    d.ellipse([(32, 30), (40, 38)], fill=p["black_hole"])
    d.arc([(31, 29), (41, 39)], start=210, end=330, fill=p["aqua_core"], width=1)

    # Death Mask & Crown of 5 Spires
    d.polygon([(29, 13), (43, 13), (41, 22), (36, 25), (31, 22)], fill=p["mask"])
    d.rectangle([(31, 16), (34, 19)], fill=p["void"])
    d.rectangle([(38, 16), (41, 19)], fill=p["void"])
    put_px(body, 32, 17, p["aqua"])
    put_px(body, 39, 17, p["aqua"])
    put_px(glow, 32, 17, p["aqua_core"])
    put_px(glow, 39, 17, p["aqua_core"])

    spires = [
        (26, 8, 29, 14),
        (31, 5, 33, 13),
        (36, 2, 36, 13),
        (41, 5, 39, 13),
        (46, 8, 43, 14),
    ]
    for tip_x, tip_y, base_x, base_y in spires:
        d.polygon([(tip_x, tip_y), (base_x - 2, base_y), (base_x + 2, base_y)], fill=p["void"])
        d.line([(tip_x, tip_y), (base_x, base_y)], fill=p["plate_hi"])
        put_px(body, tip_x, tip_y, p["aqua"])

    return compose(shadow, body, glow)


# ---------------------------------------------------------------------------
# Batch Generation Pipeline
# ---------------------------------------------------------------------------

GENERATORS = [
    ("mob_tier09_dread_phalanx.png", draw_tier09_dread_phalanx),
    ("mob_tier10_hellhound.png", draw_tier10_hellhound),
    ("mob_tier11_void_cultist.png", draw_tier11_void_cultist),
    ("mob_tier12_obsidian_colossus.png", draw_tier12_obsidian_colossus),
    ("mob_tier13_nuke_bomber.png", draw_tier13_nuke_bomber),
    ("mob_tier14_shadow_stalker.png", draw_tier14_shadow_stalker),
    ("mob_tier15_solar_archon.png", draw_tier15_solar_archon),
    ("mob_tier16_avatar_oblivion.png", draw_tier16_avatar_oblivion),
]


def generate_all(out_root: Path) -> list[Path]:
    mobs_dir = out_root / "assets" / "mobs"
    sprites_dir = out_root / "assets" / "sprites"
    legion_dir = out_root / "assets" / "monsters" / "legion"
    out_dirs = [mobs_dir, sprites_dir, legion_dir]
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
        description="Generate High Legion (Tiers 9–16) RGBA pixel-art sprites."
    )
    parser.add_argument(
        "--out-root",
        type=Path,
        default=Path("."),
        help="Project root directory where assets/mobs/, assets/monsters/legion/ and assets/sprites/ will be written.",
    )
    args = parser.parse_args()

    files = generate_all(args.out_root)
    print(f"Generated {len(files)} PNG files under {(args.out_root / 'assets').resolve()}")


if __name__ == "__main__":
    main()
