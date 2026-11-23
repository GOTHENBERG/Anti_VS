#!/usr/bin/env python3
"""Procedural pixel-art generator for Structures, Altars, Obelisks & Drops (Group 5).

Generates all 8 tactical world objects & resource drops under Tri-Layer Luminance
with crisp 1px contrast outlines (#0C0A12) and alpha transparency:
  - structure_altar.png          (84x108)  Blood Altar
  - structure_obelisk.png        (128x128) Dormant Rift Obelisk
  - structure_obelisk_active.png (128x128) Active Rift Obelisk
  - structure_wall.png           (50x50)   Barricade Wall
  - crystal_soul.png             (32x32)   Soul Crystal (XP Gem)
  - soul_ethereal.png            (64x64)   Ethereal Soul Wisp
  - soul_ethereal_sm.png         (24x24)   Ethereal Soul Icon (UI / Projectile)
  - item_chest.png               (32x32)   Relic Chest

Usage:
    python scripts/generate_structures.py
    python scripts/generate_structures.py --out-root .
"""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Tuple

from PIL import Image, ImageDraw

RGBA = Tuple[int, int, int, int]

OUTLINE_DARK: RGBA = (12, 10, 18, 255)      # #0C0A12
SHADOW_HEAVY: RGBA = (0, 0, 0, 160)
SHADOW_LIGHT: RGBA = (0, 0, 0, 115)


# ---------------------------------------------------------------------------
# Shared Pixel-Art Helpers
# ---------------------------------------------------------------------------

def new_canvas(w: int, h: int) -> Image.Image:
    return Image.new("RGBA", (w, h), (0, 0, 0, 0))


def put_px(img: Image.Image, x: int, y: int, color: RGBA) -> None:
    if 0 <= x < img.width and 0 <= y < img.height:
        img.putpixel((x, y), color)


def apply_1px_outline(layer: Image.Image, outline_color: RGBA = OUTLINE_DARK) -> Image.Image:
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
    top_overlay: Image.Image | None = None,
    outline_color: RGBA = OUTLINE_DARK,
) -> Image.Image:
    outlined = apply_1px_outline(body, outline_color)
    img = Image.alpha_composite(shadow, outlined)
    if glow is not None:
        img = Image.alpha_composite(img, glow)
    if top_overlay is not None:
        img = Image.alpha_composite(img, top_overlay)
    return img


# ---------------------------------------------------------------------------
# 1. Blood Altar (structure_altar.png, 84x108)
# ---------------------------------------------------------------------------

def draw_structure_altar() -> Image.Image:
    shadow = new_canvas(84, 108)
    body = new_canvas(84, 108)
    glow = new_canvas(84, 108)
    d_sh = ImageDraw.Draw(shadow)
    d = ImageDraw.Draw(body)
    d_gl = ImageDraw.Draw(glow)

    basalt_dark = (24, 18, 32, 255)         # #181220
    basalt_mid = (44, 34, 58, 255)          # #2C223A
    basalt_hi = (78, 62, 100, 255)          # #4E3E64
    gold_base = (190, 140, 50, 255)         # #BE8C32
    gold_hi = (245, 205, 95, 255)           # #F5CD5F
    blood_dark = (110, 0, 18, 255)          # #6E0012
    blood_mid = (217, 4, 41, 255)           # #D90429
    blood_foam = (255, 77, 109, 255)        # #FF4D6D
    blood_glow = (217, 4, 41, 100)
    magenta_rim = (181, 23, 158, 255)       # #B5179E
    bone = (224, 214, 198, 255)
    white = (255, 255, 255, 255)

    # Layer 0: Contact shadow [(8, 92), (76, 105)]
    d_sh.ellipse([(8, 92), (76, 105)], fill=SHADOW_HEAVY)

    # Layer 1: Stepped Dais (Y: 82..102)
    # Bottom tier
    d.polygon([(10, 94), (74, 94), (78, 102), (6, 102)], fill=basalt_dark)
    d.line([(10, 94), (74, 94)], fill=basalt_hi, width=2)
    # Mid tier
    d.polygon([(16, 88), (68, 88), (72, 94), (12, 94)], fill=basalt_mid)
    d.line([(16, 88), (68, 88)], fill=basalt_hi)
    # Top plinth with red-gold trim
    d.polygon([(22, 82), (62, 82), (66, 88), (18, 88)], fill=basalt_dark)
    d.line([(22, 82), (62, 82)], fill=gold_base, width=2)
    d.line([(24, 82), (60, 82)], fill=gold_hi, width=1)

    # Layer 2: Central Column (24..60, 42..82) & Fluted Gargoyle Pilasters
    d.rectangle([(24, 42), (60, 82)], fill=basalt_mid)
    d.rectangle([(29, 44), (55, 80)], fill=basalt_dark)

    # Left & Right fluted pilasters [(18,44),(24,80)] and [(60,44),(66,80)]
    for x0, x1 in ((18, 24), (60, 66)):
        d.rectangle([(x0, 44), (x1, 80)], fill=basalt_mid)
        d.line([(x0 + 1, 44), (x0 + 1, 78)], fill=basalt_hi)
        d.line([(x0 + 3, 45), (x0 + 3, 78)], fill=basalt_dark)
        # Gargoyle clawed feet at base
        d.polygon([(x0 - 2, 82), (x0, 77), (x1, 77), (x1 + 2, 82)], fill=basalt_hi)
        for cx in (x0 - 1, (x0 + x1) // 2, x1 + 1):
            put_px(body, cx, 82, bone)

    # Occult Bas-Relief Demon Skull [(38, 54), (46, 64)] in Magenta Ritual Frame
    d.ellipse([(33, 50), (51, 68)], outline=magenta_rim, width=2)
    d.polygon([(38, 54), (46, 54), (47, 61), (44, 65), (40, 65), (37, 61)], fill=bone)
    d.rectangle([(39, 57), (41, 59)], fill=basalt_dark)
    d.rectangle([(43, 57), (45, 59)], fill=basalt_dark)
    put_px(body, 40, 58, blood_foam)
    put_px(body, 44, 58, blood_foam)
    put_px(glow, 40, 58, white)
    put_px(glow, 44, 58, white)

    # Blood Drainage Channels at X=26 and X=58 + Blood Pools at Plinth
    for bx in (26, 57):
        d.rectangle([(bx, 40), (bx + 1, 82)], fill=blood_dark)
        d.line([(bx, 42), (bx, 81)], fill=blood_mid)
        d.ellipse([(bx - 4, 83), (bx + 5, 87)], fill=blood_dark)
        d.ellipse([(bx - 2, 84), (bx + 3, 86)], fill=blood_mid)

    # Layer 3: Sacred Blood Font (Y: 20..42)
    # 3 Corbel support brackets
    for cx in (28, 42, 56):
        d.polygon([(cx - 3, 42), (cx + 3, 42), (cx, 48)], fill=basalt_hi)
    # Massive bowl [(14, 26), (70, 26), (62, 42), (22, 42)]
    d.polygon([(14, 26), (70, 26), (62, 42), (22, 42)], fill=basalt_mid)
    d.line([(16, 30), (68, 30)], fill=gold_base, width=2)
    d.line([(18, 30), (66, 30)], fill=gold_hi, width=1)
    d.line([(20, 36), (64, 36)], fill=gold_base, width=2)

    # Boiling Blood Basin [(18, 22), (66, 34)]
    d_gl.ellipse([(12, 12), (72, 36)], fill=blood_glow)
    d.ellipse([(18, 22), (66, 34)], fill=blood_dark)
    d.ellipse([(22, 24), (62, 32)], fill=blood_mid)
    # 5 foaming boiling bubbles with white glints
    for bx, by, br in ((28, 27, 2), (36, 25, 2), (43, 29, 3), (51, 26, 2), (57, 28, 2)):
        d.ellipse([(bx - br, by - br), (bx + br, by + br)], fill=blood_foam)
        put_px(body, bx - 1, by - 1, white)

    # Layer 4: Curved Stone Altar Horns to (12, 8) and (72, 8) + Gold Orb Finials
    left_horn = [(16, 27), (10, 20), (10, 12), (12, 8), (15, 13), (17, 21), (22, 26)]
    right_horn = [(68, 27), (74, 20), (74, 12), (72, 8), (69, 13), (67, 21), (62, 26)]
    for horn, tip in ((left_horn, (12, 8)), (right_horn, (72, 8))):
        d.polygon(horn, fill=basalt_dark)
        d.line([horn[0], horn[1], horn[2], tip], fill=basalt_hi, width=2)
        tx, ty = tip
        d.ellipse([(tx - 3, ty - 3), (tx + 3, ty + 3)], fill=gold_base)
        put_px(body, tx - 1, ty - 1, gold_hi)

    return compose(shadow, body, glow)


# ---------------------------------------------------------------------------
# 2. Dormant Rift Obelisk (structure_obelisk.png, 128x128)
# ---------------------------------------------------------------------------

def draw_structure_obelisk() -> Image.Image:
    shadow = new_canvas(128, 128)
    body = new_canvas(128, 128)
    d_sh = ImageDraw.Draw(shadow)
    d = ImageDraw.Draw(body)

    obs_dark = (16, 12, 24, 255)            # #100C18
    obs_mid = (34, 26, 48, 255)             # #221A30
    obs_lit = (68, 54, 92, 255)             # #44365C
    rune_dim = (85, 25, 115, 255)           # #551973
    rune_dot = (135, 45, 175, 255)          # #872DAF
    chain_iron = (90, 85, 105, 255)         # #5A5569
    chain_hi = (150, 145, 170, 255)         # #9691AA

    # Layer 0: Contact shadow [(16, 108), (112, 124)]
    d_sh.ellipse([(16, 108), (112, 124)], fill=SHADOW_HEAVY)

    # Layer 1: Three-Tier Stepped Obsidian Pedestal (Y: 92..116)
    tiers = [
        (18, 106, 110, 115),
        (26, 98, 102, 106),
        (34, 92, 94, 98),
    ]
    for x0, y0, x1, y1 in tiers:
        d.rectangle([(x0, y0), (x1, y1)], fill=obs_dark)
        d.line([(x0, y0), (x1, y0)], fill=obs_lit, width=2)
        d.line([(x0, y0), (x0, y1)], fill=obs_mid, width=2)

    # Layer 2: Monolithic Tapering Spire (Y: 10..92, apex (64, 10))
    d.polygon([(64, 10), (52, 24), (44, 92), (64, 92)], fill=obs_mid)
    d.polygon([(64, 10), (52, 24), (64, 24)], fill=obs_lit)
    d.line([(52, 24), (44, 92)], fill=obs_lit, width=2)
    d.polygon([(64, 10), (76, 24), (84, 92), (64, 92)], fill=obs_dark)
    d.line([(64, 10), (64, 92)], fill=obs_lit, width=2)

    # Dormant Void Runes along central axis
    for ry in (30, 48, 66, 82):
        d.polygon([(64, ry - 4), (68, ry), (64, ry + 4), (60, ry)], outline=rune_dim)
        put_px(body, 64, ry, rune_dot)
        put_px(body, 58, ry, rune_dim)
        put_px(body, 70, ry, rune_dim)

    # Layer 3: Three Heavy Iron Binding Chains at Y=40, 58, 76
    for cy, half_w in ((40, 16), (58, 19), (76, 22)):
        d.arc([(64 - half_w, cy - 5), (64 + half_w, cy + 5)], start=0, end=180, fill=chain_iron, width=3)
        for lx in range(64 - half_w + 2, 64 + half_w - 1, 5):
            put_px(body, lx, cy + 3, chain_hi)
            put_px(body, lx + 1, cy + 2, chain_hi)
        d.line([(64 - half_w, cy), (64 - half_w - 2, cy + 8)], fill=chain_iron, width=2)
        d.line([(64 + half_w, cy), (64 + half_w + 2, cy + 8)], fill=chain_iron, width=2)
        put_px(body, 64 - half_w - 2, cy + 7, chain_hi)
        put_px(body, 64 + half_w + 2, cy + 7, chain_hi)

    return compose(shadow, body)


# ---------------------------------------------------------------------------
# 3. Active Rift Obelisk (structure_obelisk_active.png, 128x128)
# ---------------------------------------------------------------------------

def draw_structure_obelisk_active() -> Image.Image:
    shadow = new_canvas(128, 128)
    body = new_canvas(128, 128)
    glow = new_canvas(128, 128)
    top = new_canvas(128, 128)
    d_sh = ImageDraw.Draw(shadow)
    d = ImageDraw.Draw(body)
    d_gl = ImageDraw.Draw(glow)
    d_top = ImageDraw.Draw(top)

    obs_dark = (16, 12, 24, 255)
    obs_mid = (34, 26, 48, 255)
    obs_lit = (68, 54, 92, 255)
    chain_iron = (90, 85, 105, 255)
    chain_hi = (150, 145, 170, 255)
    violet = (114, 9, 183, 255)            # #7209B7
    violet_pool = (114, 9, 183, 85)
    magenta = (247, 37, 133, 255)           # #F72585
    aqua = (76, 201, 240, 255)              # #4CC9F0
    core_white = (224, 251, 252, 255)       # #E0FBFC
    gold = (255, 208, 0, 255)               # #FFD000
    gold_glow = (255, 208, 0, 120)

    # Layer 0: Cosmic Breach Ring & Void Pool
    d_gl.ellipse([(10, 103), (118, 126)], fill=violet_pool)
    d_sh.ellipse([(16, 108), (112, 124)], fill=SHADOW_HEAVY)
    d_gl.ellipse([(16, 107), (112, 124)], outline=gold, width=3)

    # Layer 1: Shattered Dais + Plasma Rift Chasm
    for x0, x1 in ((18, 58), (70, 110)):
        d.rectangle([(x0, 106), (x1, 115)], fill=obs_dark)
        d.line([(x0, 106), (x1, 106)], fill=obs_lit, width=2)
    for x0, x1 in ((26, 57), (71, 102)):
        d.rectangle([(x0, 98), (x1, 106)], fill=obs_mid)
        d.line([(x0, 98), (x1, 98)], fill=obs_lit, width=2)
    for x0, x1 in ((34, 56), (72, 94)):
        d.rectangle([(x0, 92), (x1, 98)], fill=obs_dark)
        d.line([(x0, 92), (x1, 92)], fill=obs_lit, width=2)

    d.polygon([(58, 92), (70, 92), (72, 116), (56, 116)], fill=magenta)
    d.line([(64, 92), (63, 104), (65, 116)], fill=aqua, width=3)
    d.line([(64, 94), (63, 104), (65, 114)], fill=core_white, width=1)

    # Layer 2: Split Floating Obelisk Halves
    left_slab = [(52, 12), (40, 28), (28, 86), (48, 88), (54, 48)]
    d.polygon(left_slab, fill=obs_mid)
    d.line([(52, 12), (40, 28), (28, 86)], fill=obs_lit, width=2)
    d.line([(52, 12), (54, 48), (48, 88)], fill=magenta, width=2)

    right_slab = [(76, 12), (88, 28), (100, 86), (80, 88), (74, 48)]
    d.polygon(right_slab, fill=obs_dark)
    d.line([(76, 12), (88, 28), (100, 86)], fill=obs_lit, width=2)
    d.line([(76, 12), (74, 48), (80, 88)], fill=magenta, width=2)

    for (sx, sy), dx in (
        ((35, 42), -8), ((31, 62), -10),
        ((93, 42), 8), ((97, 62), 10),
    ):
        d.line([(sx, sy), (sx + dx, sy + 14)], fill=chain_iron, width=3)
        put_px(body, sx + dx // 2, sy + 7, chain_hi)
        put_px(body, sx + dx, sy + 14, chain_hi)

    # Layer 3: Void Wormhole & Rift Lightning (center 64, 52)
    cx, cy = 64, 52
    d_gl.ellipse([(cx - 36, cy - 36), (cx + 36, cy + 36)], fill=violet_pool)
    d_gl.ellipse([(cx - 34, cy - 34), (cx + 34, cy + 34)], outline=gold_glow, width=3)
    d_gl.ellipse([(cx - 24, cy - 24), (cx + 24, cy + 24)], outline=magenta, width=2)

    d_top.polygon([(cx, 10), (cx + 15, cy), (cx, 98), (cx - 15, cy)], fill=violet)
    d_top.polygon([(cx, 16), (cx + 11, cy), (cx, 92), (cx - 11, cy)], fill=magenta)
    d_top.polygon([(cx, 24), (cx + 6, cy), (cx, 84), (cx - 6, cy)], fill=aqua)
    d_top.polygon([(cx, 32), (cx + 3, cy), (cx, 74), (cx - 3, cy)], fill=core_white)

    arcs = [
        [(cx - 8, 42), (50, 36), (44, 30), (38, 34)],
        [(cx - 10, 62), (48, 68), (40, 74), (34, 72)],
        [(cx + 8, 42), (78, 36), (84, 30), (90, 34)],
        [(cx + 10, 62), (80, 68), (88, 74), (94, 72)],
    ]
    for arc in arcs:
        d_top.line(arc, fill=aqua, width=3)
        d_top.line(arc, fill=core_white, width=1)

    return compose(shadow, body, glow, top_overlay=top)


# ---------------------------------------------------------------------------
# 4. Barricade Wall (structure_wall.png, 50x50)
# ---------------------------------------------------------------------------

def draw_structure_wall() -> Image.Image:
    shadow = new_canvas(50, 50)
    body = new_canvas(50, 50)
    d_sh = ImageDraw.Draw(shadow)
    d = ImageDraw.Draw(body)

    stone = (52, 46, 64, 255)               # #342E40
    bevel_hi = (88, 80, 108, 255)           # #58506C
    bevel_lo = (32, 28, 40, 255)            # #201C28
    crack = (10, 8, 14, 255)                # #0A080E
    iron = (22, 20, 26, 255)                # #16141A
    rivet = (180, 175, 195, 255)            # #B4AFC3

    d_sh.ellipse([(2, 44), (48, 49)], fill=SHADOW_HEAVY)

    courses = [
        (10, 22, [4, 18, 34, 46]),
        (22, 34, [4, 12, 26, 39, 46]),
        (34, 46, [4, 20, 36, 46]),
    ]
    for y0, y1, xs in courses:
        for x0, x1 in zip(xs[:-1], xs[1:]):
            d.rectangle([(x0, y0), (x1, y1)], fill=stone)
            d.line([(x0 + 1, y0 + 1), (x1 - 1, y0 + 1)], fill=bevel_hi)
            d.line([(x0 + 1, y0 + 1), (x0 + 1, y1 - 1)], fill=bevel_hi)
            d.line([(x0 + 1, y1 - 1), (x1 - 1, y1 - 1)], fill=bevel_lo)
            d.line([(x1 - 1, y0 + 1), (x1 - 1, y1 - 1)], fill=bevel_lo)
            d.line([(x0, y0), (x1, y0)], fill=crack)
            d.line([(x0, y0), (x0, y1)], fill=crack)

    d.line([(16, 13), (23, 21), (26, 29), (33, 41)], fill=crack, width=2)
    d.line([(23, 21), (19, 27)], fill=crack, width=1)

    corners = [
        [(4, 10, 12, 13), (4, 10, 7, 18), (6, 12)],
        [(38, 10, 46, 13), (43, 10, 46, 18), (44, 12)],
        [(4, 43, 12, 46), (4, 38, 7, 46), (6, 44)],
        [(38, 43, 46, 46), (43, 38, 46, 46), (44, 44)],
    ]
    for r1, r2, rv in corners:
        d.rectangle([(r1[0], r1[1]), (r1[2], r1[3])], fill=iron)
        d.rectangle([(r2[0], r2[1]), (r2[2], r2[3])], fill=iron)
        d.rectangle([(rv[0] - 1, rv[1] - 1), (rv[0], rv[1])], fill=rivet)

    d.rectangle([(4, 21), (46, 24)], fill=iron)
    for rx in (12, 25, 38):
        d.rectangle([(rx - 1, 22), (rx, 23)], fill=rivet)

    spikes = [(8, 2), (18, 3), (25, 1), (32, 3), (42, 2)]
    for tx, ty in spikes:
        d.polygon([(tx, ty), (tx - 3, 10), (tx + 3, 10)], fill=iron)
        d.line([(tx, ty), (tx - 1, 10)], fill=rivet)

    return compose(shadow, body)


# ---------------------------------------------------------------------------
# 5. Soul Crystal (crystal_soul.png, 32x32)
# ---------------------------------------------------------------------------

def draw_crystal_soul() -> Image.Image:
    shadow = new_canvas(32, 32)
    body = new_canvas(32, 32)
    glow = new_canvas(32, 32)
    d_sh = ImageDraw.Draw(shadow)
    d = ImageDraw.Draw(body)
    d_gl = ImageDraw.Draw(glow)

    f1 = (224, 251, 252, 255)               # #E0FBFC
    f2 = (72, 202, 228, 255)                # #48CAE4
    f3 = (0, 119, 182, 255)                 # #0077B6
    f4 = (2, 62, 138, 255)                  # #023E8A
    aura = (72, 202, 228, 80)
    white = (255, 255, 255, 255)

    d_sh.ellipse([(8, 26), (24, 30)], fill=SHADOW_LIGHT)
    d_gl.ellipse([(5, 4), (27, 26)], fill=aura)

    top, bot, left, mid, right = (16, 4), (16, 26), (7, 15), (16, 16), (25, 15)
    d.polygon([top, left, mid], fill=f1)
    d.polygon([top, right, mid], fill=f2)
    d.polygon([bot, left, mid], fill=f3)
    d.polygon([bot, right, mid], fill=f4)

    d.line([top, bot], fill=white)
    d.line([left, mid, right], fill=f1)
    d_gl.line([(16, 0), (16, 8)], fill=white, width=1)
    d_gl.line([(12, 4), (20, 4)], fill=white, width=1)
    put_px(glow, 16, 4, white)

    return compose(shadow, body, glow)


# ---------------------------------------------------------------------------
# 6. Ethereal Soul (soul_ethereal.png 64x64 & soul_ethereal_sm.png 24x24)
# ---------------------------------------------------------------------------

def draw_soul_ethereal(size: int) -> Image.Image:
    shadow = new_canvas(size, size)
    body = new_canvas(size, size)
    glow = new_canvas(size, size)
    d = ImageDraw.Draw(body)
    d_gl = ImageDraw.Draw(glow)

    s = size / 64.0
    def pt(x: float, y: float) -> Tuple[int, int]:
        return (int(round(x * s)), int(round(y * s)))

    corona = (76, 201, 240, 255)            # #4CC9F0
    corona_glow = (76, 201, 240, 95)
    core = (224, 251, 252, 255)             # #E0FBFC
    white = (255, 255, 255, 255)
    smoke_hi = (114, 9, 183, 215)           # #7209B7
    smoke_lo = (58, 12, 163, 185)           # #3A0CA3
    eye_dark = (14, 10, 26, 255)

    tail_outer = [pt(36, 32), pt(28, 44), pt(16, 52), pt(10, 46), pt(18, 40), pt(22, 30)]
    d.polygon(tail_outer, fill=smoke_lo)
    tail_inner = [pt(34, 30), pt(26, 41), pt(17, 46), pt(22, 36)]
    d.polygon(tail_inner, fill=smoke_hi)

    d_gl.ellipse([pt(16, 6), pt(56, 46)], fill=corona_glow)
    d.ellipse([pt(20, 10), pt(52, 42)], fill=corona)
    d.ellipse([pt(25, 15), pt(47, 37)], fill=core)
    d.ellipse([pt(31, 21), pt(41, 31)], fill=white)

    d.polygon([pt(30, 12), pt(38, 4), pt(42, 14)], fill=corona)
    d.polygon([pt(34, 12), pt(38, 7), pt(40, 14)], fill=core)

    eye_w = max(1, int(round(2 * s)))
    d.line([pt(29, 25), pt(34, 27)], fill=eye_dark, width=eye_w)
    d.line([pt(43, 25), pt(38, 27)], fill=eye_dark, width=eye_w)

    return compose(shadow, body, glow)


# ---------------------------------------------------------------------------
# 7. Relic Chest (item_chest.png, 32x32)
# ---------------------------------------------------------------------------

def draw_item_chest() -> Image.Image:
    shadow = new_canvas(32, 32)
    body = new_canvas(32, 32)
    glow = new_canvas(32, 32)
    d_sh = ImageDraw.Draw(shadow)
    d = ImageDraw.Draw(body)
    d_gl = ImageDraw.Draw(glow)

    oak = (78, 48, 32, 255)                 # #4E3020
    oak_dark = (48, 30, 20, 255)            # #301E14
    oak_hi = (118, 76, 50, 255)             # #764C32
    iron = (36, 38, 46, 255)
    iron_hi = (118, 126, 142, 255)
    gold = (212, 168, 55, 255)              # #D4A837
    gold_beam = (255, 215, 0, 115)          # #FFD700
    gold_bright = (255, 215, 0, 255)
    white_beam = (255, 255, 235, 175)

    d_sh.ellipse([(4, 25), (28, 29)], fill=SHADOW_LIGHT)

    d_gl.polygon([(8, 14), (24, 14), (29, 1), (3, 1)], fill=gold_beam)
    d_gl.polygon([(12, 14), (20, 14), (23, 2), (9, 2)], fill=white_beam)

    d.rectangle([(5, 15), (27, 26)], fill=oak)
    d.line([(6, 16), (26, 16)], fill=oak_hi)
    for py in (19, 23):
        d.line([(6, py), (26, py)], fill=oak_dark)
    for x0, x1 in ((5, 7), (25, 27)):
        d.rectangle([(x0, 15), (x1, 26)], fill=iron)
        put_px(body, x0 + 1, 17, gold_bright)
        put_px(body, x0 + 1, 24, gold_bright)
    d.line([(5, 26), (27, 26)], fill=iron)

    d.line([(6, 14), (26, 14)], fill=gold_bright)
    d.line([(10, 14), (22, 14)], fill=(255, 255, 255, 255))

    d.polygon([(5, 13), (7, 8), (11, 6), (21, 6), (25, 8), (27, 13)], fill=oak)
    d.line([(10, 7), (22, 7)], fill=oak_hi)
    d.line([(6, 10), (26, 10)], fill=oak_dark)
    for rx in (10, 22):
        d.line([(rx, 6), (rx, 13)], fill=iron, width=2)
        put_px(body, rx, 7, iron_hi)
        put_px(body, rx, 12, gold_bright)

    d.rectangle([(13, 13), (19, 20)], fill=gold, outline=oak_dark)
    d.rectangle([(14, 14), (18, 17)], fill=gold_bright)
    put_px(body, 15, 15, oak_dark)
    put_px(body, 17, 15, oak_dark)
    d.line([(16, 17), (16, 19)], fill=oak_dark)

    return compose(shadow, body, glow)


# ---------------------------------------------------------------------------
# Batch Generation Pipeline
# ---------------------------------------------------------------------------

GENERATORS = [
    ("structure_altar.png", draw_structure_altar),
    ("structure_obelisk.png", draw_structure_obelisk),
    ("structure_obelisk_active.png", draw_structure_obelisk_active),
    ("structure_wall.png", draw_structure_wall),
    ("crystal_soul.png", draw_crystal_soul),
    ("soul_ethereal.png", lambda: draw_soul_ethereal(64)),
    ("soul_ethereal_sm.png", lambda: draw_soul_ethereal(24)),
    ("item_chest.png", draw_item_chest),
]


def generate_all(out_root: Path) -> list[Path]:
    out_dirs = [
        out_root / "assets" / "structures",
        out_root / "assets" / "environment",
        out_root / "assets" / "sprites",
    ]
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
        description="Generate Group 5 Structures, Altars, Obelisks & Drops (RGBA)."
    )
    parser.add_argument(
        "--out-root",
        type=Path,
        default=Path("."),
        help="Project root directory where assets/structures/ and assets/sprites/ will be written.",
    )
    args = parser.parse_args()

    files = generate_all(args.out_root)
    print(f"Generated {len(files)} PNG files under {(args.out_root / 'assets').resolve()}")


if __name__ == "__main__":
    main()
