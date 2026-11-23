#!/usr/bin/env python3
"""Procedural pixel-art generator for 20 Basic Weapons Icons (Group 7).

Generates 20 crisp 32x32 RGBA weapon icons in Dark Fantasy Gothic Pixel Art
style with transparent backgrounds, Tri-Layer Luminance contrast, and 1px
dark silhouette outlines.

Outputs are saved to `assets/icons/` and mirrored to `assets/icons/weapons/`.

Usage:
    python scripts/generate_weapons.py
    python scripts/generate_weapons.py --out-root .
"""

from __future__ import annotations

import argparse
import math
from pathlib import Path
from typing import Tuple

from PIL import Image, ImageDraw

RGBA = Tuple[int, int, int, int]

SIZE = 32


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def hex_rgba(h: str, a: int = 255) -> RGBA:
    h = h.lstrip("#")
    return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16), a)


def new_canvas() -> Image.Image:
    return Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))


def put_px(img: Image.Image, x: int, y: int, col: RGBA) -> None:
    if 0 <= x < SIZE and 0 <= y < SIZE:
        img.putpixel((x, y), col)


def apply_1px_outline(layer: Image.Image, outline_color: RGBA) -> Image.Image:
    """Apply a crisp 4-connected 1px dark outline around opaque pixels."""
    src = layer.load()
    out = layer.copy()
    dst = out.load()
    for y in range(SIZE):
        for x in range(SIZE):
            if src[x, y][3] > 0:
                continue
            for nx, ny in ((x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)):
                if 0 <= nx < SIZE and 0 <= ny < SIZE and src[nx, ny][3] >= 180:
                    dst[x, y] = outline_color
                    break
    return out


def finalize(
    body: Image.Image,
    outline_hex: str,
    glow: Image.Image | None = None,
    top: Image.Image | None = None,
) -> Image.Image:
    img = apply_1px_outline(body, hex_rgba(outline_hex))
    if glow is not None:
        img = Image.alpha_composite(glow, img)
    if top is not None:
        img = Image.alpha_composite(img, top)
    return img


# ---------------------------------------------------------------------------
# 1. icon_wind_blade.png — Wind Blade
# ---------------------------------------------------------------------------

def draw_wind_blade() -> Image.Image:
    body = new_canvas()
    top = new_canvas()
    d = ImageDraw.Draw(body)
    dt = ImageDraw.Draw(top)

    st_sh, st_b, st_r, st_hi = hex_rgba("#475569"), hex_rgba("#94A3B8"), hex_rgba("#CBD5E1"), hex_rgba("#F8FAFC")
    w_deep, w_jade, w_neon, w_foam = hex_rgba("#0F766E"), hex_rgba("#14B8A6"), hex_rgba("#2DD4BF"), hex_rgba("#F0FDFA")
    leather, brass, pommel = hex_rgba("#451A03"), hex_rgba("#D97706"), hex_rgba("#F59E0B")

    # Grip (6,25)->(11,20)
    d.line([(6, 25), (11, 20)], fill=leather, width=2)
    put_px(body, 6, 25, pommel)
    put_px(body, 5, 26, brass)
    # S-shaped brass guard (10,22)-(13,19)
    d.line([(9, 18), (11, 20), (13, 22), (14, 23)], fill=brass, width=2)
    put_px(body, 11, 20, pommel)

    # Curved moon-steel single-edged blade flaring toward tip (25,6)
    blade = [(12, 19), (16, 13), (22, 8), (26, 5), (24, 11), (19, 17), (13, 20)]
    d.polygon(blade, fill=st_b)
    d.line([(12, 19), (17, 13), (25, 6)], fill=st_hi, width=1)
    d.line([(13, 19), (18, 14), (24, 8)], fill=st_r, width=1)
    d.line([(14, 20), (19, 16), (24, 10)], fill=st_sh, width=1)

    # 2 Swirling emerald/neon wind vortex arcs along blade
    arc1 = [(9, 14), (14, 9), (20, 7), (26, 7)]
    arc2 = [(15, 22), (21, 18), (25, 13), (27, 9)]
    dt.line(arc1, fill=w_neon, width=1)
    dt.line([(14, 9), (20, 7)], fill=w_foam, width=1)
    dt.line(arc2, fill=w_jade, width=1)
    put_px(top, 21, 18, w_foam)
    put_px(top, 11, 12, w_deep)

    return finalize(body, "#0A0812", top=top)


# ---------------------------------------------------------------------------
# 2. icon_fire_aura.png — Fire Aura
# ---------------------------------------------------------------------------

def draw_fire_aura() -> Image.Image:
    body = new_canvas()
    glow = new_canvas()
    d = ImageDraw.Draw(body)
    dg = ImageDraw.Draw(glow)

    ir_d, ir_m, ir_hi = hex_rgba("#1F2937"), hex_rgba("#374151"), hex_rgba("#6B7280")
    core, inner, mid, ember, spark = (
        hex_rgba("#FEF08A"), hex_rgba("#F59E0B"), hex_rgba("#DC2626"), hex_rgba("#7F1D1D"), hex_rgba("#FFFBEB")
    )

    # Soft heat glow
    dg.ellipse([(5, 5), (27, 27)], fill=hex_rgba("#DC2626", 60))

    # Forged iron ring (center 16,16, r=10) + 4 spikes at 0,90,180,270 deg
    d.ellipse([(6, 6), (26, 26)], outline=ir_m, width=3)
    d.ellipse([(7, 7), (25, 25)], outline=ir_hi, width=1)
    d.ellipse([(8, 8), (24, 24)], outline=ir_d, width=1)
    for pts in (
        [(14, 6), (18, 6), (16, 2)],
        [(14, 26), (18, 26), (16, 30)],
        [(6, 14), (6, 18), (2, 16)],
        [(26, 14), (26, 18), (30, 16)],
    ):
        d.polygon(pts, fill=ir_m)
        put_px(body, pts[2][0], pts[2][1], ir_hi)

    # Central blazing fire sphere (r=6) + 5 tongues bursting through ring
    d.ellipse([(10, 10), (22, 22)], fill=ember)
    d.ellipse([(11, 11), (21, 21)], fill=mid)
    d.ellipse([(12, 12), (20, 20)], fill=inner)
    d.rectangle([(15, 15), (17, 17)], fill=core)
    put_px(body, 16, 16, spark)

    tongues = [
        [(14, 11), (16, 4), (18, 11)],
        [(10, 13), (5, 7), (12, 15)],
        [(22, 13), (27, 8), (20, 16)],
        [(11, 20), (6, 24), (14, 21)],
        [(21, 20), (26, 23), (19, 21)],
    ]
    for t in tongues:
        d.polygon(t, fill=mid)
        d.line([t[0], t[1]], fill=inner)
        put_px(body, t[1][0], t[1][1], core)

    return finalize(body, "#080408", glow=glow)


# ---------------------------------------------------------------------------
# 3. icon_magic_wand.png — Magic Wand
# ---------------------------------------------------------------------------

def draw_magic_wand() -> Image.Image:
    body = new_canvas()
    top = new_canvas()
    d = ImageDraw.Draw(body)

    oak_d, oak_m, oak_l = hex_rgba("#1C130E"), hex_rgba("#3E2723"), hex_rgba("#5D4037")
    g_d, g_m, g_l = hex_rgba("#92400E"), hex_rgba("#D97706"), hex_rgba("#FDE68A")
    s_d, s_m, s_c, s_hi = hex_rgba("#1E3A8A"), hex_rgba("#2563EB"), hex_rgba("#60A5FA"), hex_rgba("#EFF6FF")

    # Spiral bog-oak shaft (6,26)->(19,13)
    d.line([(6, 26), (19, 13)], fill=oak_m, width=3)
    d.line([(6, 26), (19, 13)], fill=oak_d, width=1)
    for i in range(0, 13, 3):
        put_px(body, 6 + i, 25 - i, oak_l)

    # 4-petal golden crown mount at (18,14)-(21,11)
    d.polygon([(17, 13), (20, 10), (22, 12), (19, 15)], fill=g_m)
    d.line([(18, 12), (21, 11)], fill=g_l)
    put_px(body, 17, 11, g_d)
    put_px(body, 21, 15, g_d)

    # 7x9 faceted diamond sapphire at (23, 9)
    cx, cy = 23, 9
    d.polygon([(cx, cy - 5), (cx + 4, cy), (cx, cy + 4), (cx - 4, cy)], fill=s_m)
    d.polygon([(cx, cy - 5), (cx - 4, cy), (cx, cy)], fill=s_c)
    d.polygon([(cx, cy + 4), (cx + 4, cy), (cx, cy)], fill=s_d)
    put_px(body, cx - 1, cy - 2, s_hi)
    put_px(body, cx, cy - 3, s_hi)

    # 3 floating magic sparks
    for sx, sy in ((26, 6), (22, 5), (27, 11)):
        put_px(top, sx, sy, s_hi)

    return finalize(body, "#070614", top=top)


# ---------------------------------------------------------------------------
# 4. icon_holy_hammer.png — Holy Hammer
# ---------------------------------------------------------------------------

def draw_holy_hammer() -> Image.Image:
    body = new_canvas()
    glow = new_canvas()
    d = ImageDraw.Draw(body)
    dg = ImageDraw.Draw(glow)

    st_d, st_m, st_l = hex_rgba("#334155"), hex_rgba("#94A3B8"), hex_rgba("#E2E8F0")
    g_d, g_m, g_l, white = hex_rgba("#B45309"), hex_rgba("#F59E0B"), hex_rgba("#FEF08A"), hex_rgba("#FFFFFF")
    ash, silver = hex_rgba("#292524"), hex_rgba("#CBD5E1")

    dg.ellipse([(11, 3), (29, 21)], fill=hex_rgba("#FEF08A", 65))

    # Ash handle (5,27)->(17,15) with silver rings
    d.line([(5, 27), (18, 14)], fill=ash, width=3)
    for t in (2, 7, 11):
        put_px(body, 5 + t, 27 - t, silver)
        put_px(body, 6 + t, 27 - t, silver)
    d.rectangle([(4, 26), (6, 28)], fill=g_m)

    # Perpendicular 12x8 warhammer head centered at (20, 12)
    head = [(15, 7), (21, 5), (27, 11), (25, 17), (19, 19), (13, 13)]
    d.polygon(head, fill=st_m)
    d.line([(15, 7), (21, 5), (27, 11)], fill=st_l, width=2)
    d.line([(13, 13), (19, 19), (25, 17)], fill=st_d, width=2)
    # Sharp back beak spike
    d.polygon([(14, 10), (11, 7), (16, 8)], fill=st_l)
    # Engraved golden cross + white core on head face
    d.line([(18, 10), (22, 14)], fill=g_m, width=1)
    d.line([(22, 10), (18, 14)], fill=g_m, width=1)
    put_px(body, 20, 12, white)
    put_px(body, 20, 11, g_l)
    put_px(body, 21, 12, g_d)

    return finalize(body, "#0C0A10", glow=glow)


# ---------------------------------------------------------------------------
# 5. icon_ice_shards.png — Ice Shards
# ---------------------------------------------------------------------------

def draw_ice_shards() -> Image.Image:
    body = new_canvas()
    top = new_canvas()
    d = ImageDraw.Draw(body)

    c_d, c_m, c_b, c_l, frost = (
        hex_rgba("#0C4A6E"), hex_rgba("#0284C7"), hex_rgba("#38BDF8"), hex_rgba("#BAE6FD"), hex_rgba("#F0F9FF")
    )

    # Main central shard (8,24)->(25,7)
    d.polygon([(8, 24), (10, 20), (25, 7), (12, 22)], fill=c_b)
    d.polygon([(8, 24), (12, 22), (25, 7)], fill=c_m)
    d.line([(9, 23), (25, 7)], fill=frost, width=1)
    # Left flanking shard to (21,5)
    d.polygon([(9, 17), (11, 14), (21, 5), (12, 16)], fill=c_l)
    d.line([(10, 16), (21, 5)], fill=frost, width=1)
    # Right flanking shard to (27,13)
    d.polygon([(14, 24), (16, 21), (27, 13), (17, 23)], fill=c_d)
    d.line([(15, 23), (27, 13)], fill=c_l, width=1)

    # 4 micro snowflakes around fan
    for sx, sy in ((13, 9), (26, 6), (25, 21), (7, 19)):
        put_px(top, sx, sy, frost)

    return finalize(body, "#05101E", top=top)


# ---------------------------------------------------------------------------
# 6. icon_poison_orbs.png — Poison Orbs
# ---------------------------------------------------------------------------

def draw_poison_orbs() -> Image.Image:
    body = new_canvas()
    d = ImageDraw.Draw(body)

    t1, t2, t3, t4, t5 = (
        hex_rgba("#14532D"), hex_rgba("#4D7C0F"), hex_rgba("#84CC16"), hex_rgba("#BEF264"), hex_rgba("#ECFCCB")
    )

    # Main orb d=15 at (15,17) with radial gradient lit at (12,14)
    for y in range(9, 26):
        for x in range(7, 24):
            if math.hypot(x - 15, y - 17) <= 7.5:
                dl = math.hypot(x - 12, y - 14)
                if dl < 2.2:
                    col = t5
                elif dl < 4.5:
                    col = t4
                elif dl < 7.2:
                    col = t3
                elif dl < 9.5:
                    col = t2
                else:
                    col = t1
                put_px(body, x, y, col)

    # 3 protruding bubbles (d=3, 4, 2)
    d.ellipse([(16, 13), (19, 16)], fill=t4, outline=t2)
    put_px(body, 17, 14, t5)
    d.ellipse([(10, 18), (14, 22)], fill=t3, outline=t1)
    put_px(body, 11, 19, t5)
    d.rectangle([(18, 19), (19, 20)], fill=t4)

    # Satellite orb d=6 at (23, 9)
    d.ellipse([(20, 6), (26, 12)], fill=t2)
    d.ellipse([(21, 7), (24, 10)], fill=t4)
    put_px(body, 22, 8, t5)

    # Acid drips below orb at (13,26) and (18,25)
    d.line([(13, 25), (13, 28)], fill=t3)
    put_px(body, 13, 28, t4)
    d.line([(18, 24), (18, 27)], fill=t2)
    put_px(body, 18, 27, t3)

    return finalize(body, "#0A1305")


# ---------------------------------------------------------------------------
# 7. icon_executioner_axe.png — Executioner Axe
# ---------------------------------------------------------------------------

def draw_executioner_axe() -> Image.Image:
    body = new_canvas()
    d = ImageDraw.Draw(body)

    ir_d, ir_m, ir_l = hex_rgba("#1E293B"), hex_rgba("#475569"), hex_rgba("#94A3B8")
    bl_d, bl_m, bl_f = hex_rgba("#450A0A"), hex_rgba("#991B1B"), hex_rgba("#DC2626")
    oak, band = hex_rgba("#451A03"), hex_rgba("#64748B")

    # Oak haft (7,28)->(19,7)
    d.line([(7, 28), (19, 7)], fill=oak, width=3)
    d.line([(13, 17), (16, 12)], fill=band, width=3)

    # Massive semicircular cleaver blade (15,6)->(27,14)->(17,21)
    blade = [(16, 9), (15, 6), (23, 8), (27, 14), (24, 19), (17, 21), (14, 15)]
    d.polygon(blade, fill=ir_m)
    d.line([(16, 6), (23, 8), (27, 14), (24, 19), (17, 21)], fill=ir_l, width=2)
    d.line([(16, 10), (20, 14), (16, 18)], fill=ir_d, width=2)

    # Lower third covered in dark/fresh blood + 2 dripping drops
    d.polygon([(17, 16), (25, 16), (24, 19), (17, 21)], fill=bl_m)
    d.line([(18, 19), (24, 18)], fill=bl_d)
    d.line([(23, 20), (23, 24)], fill=bl_f)
    put_px(body, 23, 25, bl_f)
    d.line([(19, 22), (19, 26)], fill=bl_m)
    put_px(body, 19, 26, bl_f)

    # 2 wedge-shaped bone notches in blade edge
    put_px(body, 25, 11, (0, 0, 0, 0))
    put_px(body, 26, 11, (0, 0, 0, 0))
    put_px(body, 26, 16, (0, 0, 0, 0))

    return finalize(body, "#0D0709")


# ---------------------------------------------------------------------------
# 8. icon_celestial_bow.png — Celestial Bow
# ---------------------------------------------------------------------------

def draw_celestial_bow() -> Image.Image:
    body = new_canvas()
    top = new_canvas()
    d = ImageDraw.Draw(body)
    dt = ImageDraw.Draw(top)

    s_sh, s_m, s_p, white = hex_rgba("#334155"), hex_rgba("#94A3B8"), hex_rgba("#E2E8F0"), hex_rgba("#FFFFFF")
    g_d, g_m, g_l = hex_rgba("#D97706"), hex_rgba("#FBBF24"), hex_rgba("#FEF9C3")
    em_d, em_l = hex_rgba("#059669"), hex_rgba("#34D399")

    # Star-silver elven bow arc (7,7) -> grip (11,20) -> (24,25)
    bow_pts = [(7, 7), (6, 13), (8, 18), (11, 20), (16, 24), (24, 25)]
    d.line(bow_pts, fill=s_m, width=3)
    d.line(bow_pts, fill=s_p, width=1)
    put_px(body, 7, 7, white)
    put_px(body, 24, 25, white)
    # Spiral tip curls
    put_px(body, 9, 6, s_sh)
    put_px(body, 25, 23, s_sh)
    # Emerald gem in grip
    d.rectangle([(10, 19), (12, 21)], fill=em_d)
    put_px(body, 11, 20, em_l)

    # Taut golden light bowstring (7,7)->(24,25)
    dt.line([(7, 7), (24, 25)], fill=g_d, width=1)

    # Charged radiant light arrow aiming at (27,5)
    d.line([(12, 20), (26, 6)], fill=g_m, width=2)
    d.line([(13, 19), (26, 6)], fill=g_l, width=1)
    d.polygon([(27, 5), (22, 6), (26, 10)], fill=white)
    # Light fletching
    d.line([(11, 18), (14, 21)], fill=g_l)

    return finalize(body, "#080B14", top=top)


# ---------------------------------------------------------------------------
# 9. icon_thunder.png — Thunder Orb
# ---------------------------------------------------------------------------

def draw_thunder() -> Image.Image:
    body = new_canvas()
    glow = new_canvas()
    d = ImageDraw.Draw(body)
    dg = ImageDraw.Draw(glow)

    p1, p2, p3, p4, white = (
        hex_rgba("#3B0764"), hex_rgba("#581C87"), hex_rgba("#8B5CF6"), hex_rgba("#C4B5FD"), hex_rgba("#FFFFFF")
    )

    dg.ellipse([(6, 6), (26, 26)], fill=hex_rgba("#8B5CF6", 70))

    # Central r=7 plasma sphere at (16,16)
    d.ellipse([(9, 9), (23, 23)], fill=p1)
    d.ellipse([(10, 10), (22, 22)], fill=p2)
    d.ellipse([(12, 12), (20, 20)], fill=p3)
    # 3 crossing white arcs inside sphere
    d.line([(12, 14), (16, 16), (20, 13)], fill=white)
    d.line([(13, 19), (16, 16), (19, 19)], fill=white)
    d.line([(16, 12), (15, 16), (17, 20)], fill=p4)

    # 4 jagged lightning bolts bursting toward corners
    bolts = [
        [(16, 10), (19, 7), (21, 8), (24, 4)],
        [(21, 19), (25, 20), (24, 23), (28, 24)],
        [(11, 20), (8, 21), (8, 24), (4, 26)],
        [(10, 12), (7, 12), (6, 9), (4, 8)],
    ]
    for b in bolts:
        d.line(b, fill=p4, width=1)
        put_px(body, b[-1][0], b[-1][1], white)

    return finalize(body, "#080516", glow=glow)


# ---------------------------------------------------------------------------
# 10. icon_blood_daggers.png — Blood Daggers
# ---------------------------------------------------------------------------

def draw_blood_daggers() -> Image.Image:
    body = new_canvas()
    d = ImageDraw.Draw(body)

    b_d, b_m, b_h, b_e = hex_rgba("#7F1D1D"), hex_rgba("#B91C1C"), hex_rgba("#EF4444"), hex_rgba("#FCA5A5")
    gold, obs = hex_rgba("#78350F"), hex_rgba("#18181B")

    # Dagger 1: (5,27)->(26,6) wavy flamberge blade
    d.line([(5, 27), (10, 22)], fill=obs, width=2)
    d.line([(8, 20), (12, 24)], fill=gold, width=2)
    blade1 = [(10, 21), (13, 17), (15, 18), (19, 13), (21, 14), (26, 6), (21, 9), (18, 11), (15, 14), (11, 19)]
    d.polygon(blade1, fill=b_m)
    d.line([(11, 20), (16, 15), (26, 6)], fill=b_h, width=1)
    put_px(body, 25, 7, b_e)

    # Dagger 2: (26,27)->(5,6) wavy flamberge blade crossing at (16,16)
    d.line([(26, 27), (21, 22)], fill=obs, width=2)
    d.line([(23, 20), (19, 24)], fill=gold, width=2)
    blade2 = [(21, 21), (18, 17), (16, 18), (12, 13), (10, 14), (5, 6), (10, 9), (13, 11), (16, 14), (20, 19)]
    d.polygon(blade2, fill=b_d)
    d.line([(20, 20), (15, 15), (5, 6)], fill=b_h, width=1)
    put_px(body, 6, 7, b_e)

    # Crossing highlight & dripping crimson drops
    put_px(body, 16, 16, b_e)
    for dx, dy in ((14, 21), (18, 22), (22, 13)):
        put_px(body, dx, dy, b_h)
        put_px(body, dx, dy + 1, b_d)

    return finalize(body, "#120406")


# ---------------------------------------------------------------------------
# 11. icon_singularity.png — Singularity Orb
# ---------------------------------------------------------------------------

def draw_singularity() -> Image.Image:
    body = new_canvas()
    glow = new_canvas()
    d = ImageDraw.Draw(body)
    dg = ImageDraw.Draw(glow)

    black, horizon = hex_rgba("#000000"), hex_rgba("#0F051D")
    p_d, p_f, p_m, p_hi = hex_rgba("#581C87"), hex_rgba("#A21CAF"), hex_rgba("#E879F9"), hex_rgba("#FDF4FF")

    dg.ellipse([(4, 4), (28, 28)], fill=hex_rgba("#581C87", 80))

    # Dual counterclockwise spiral accretion arms up to radius 12
    for arm_offset in (0.0, math.pi):
        for step in range(42):
            t = step / 41.0
            r = 5.5 + t * 7.0
            ang = arm_offset + t * 2.5
            x = int(round(16 + math.cos(ang) * r))
            y = int(round(16 - math.sin(ang) * r))
            col = p_m if t < 0.35 else (p_f if t < 0.7 else p_d)
            d.rectangle([(x - 1, y - 1), (x, y)], fill=col)

    # Event horizon pure black r=5 at (16,16) + 1px photon rim
    d.ellipse([(10, 10), (22, 22)], fill=p_m)
    d.ellipse([(11, 11), (21, 21)], fill=black)
    d.arc([(10, 10), (22, 22)], start=200, end=340, fill=p_hi, width=1)
    put_px(body, 16, 16, black)

    # Gravitationally bent peripheral star specks
    for sx, sy in ((5, 6), (27, 7), (26, 26), (6, 25)):
        put_px(glow, sx, sy, p_hi)

    return finalize(body, "#0F051D", glow=glow)


# ---------------------------------------------------------------------------
# 12. icon_chain_frost.png — Chain Frost
# ---------------------------------------------------------------------------

def draw_chain_frost() -> Image.Image:
    body = new_canvas()
    top = new_canvas()
    d = ImageDraw.Draw(body)
    dt = ImageDraw.Draw(top)

    c1, c2, c3, c4, c5 = (
        hex_rgba("#0369A1"), hex_rgba("#0284C7"), hex_rgba("#38BDF8"), hex_rgba("#BAE6FD"), hex_rgba("#F0F9FF")
    )

    # 6 pyramidal ice spikes radiating from central sphere
    for i in range(6):
        ang = i * (math.pi / 3.0)
        x_tip = int(round(16 + math.cos(ang) * 11))
        y_tip = int(round(16 + math.sin(ang) * 11))
        x_a = int(round(16 + math.cos(ang + 0.35) * 5))
        y_a = int(round(16 + math.sin(ang + 0.35) * 5))
        x_b = int(round(16 + math.cos(ang - 0.35) * 5))
        y_b = int(round(16 + math.sin(ang - 0.35) * 5))
        d.polygon([(x_a, y_a), (x_tip, y_tip), (x_b, y_b)], fill=c3)
        put_px(body, x_tip, y_tip, c5)

    # Central d=12 icy orb at (16,16)
    d.ellipse([(10, 10), (22, 22)], fill=c1)
    d.ellipse([(11, 11), (20, 20)], fill=c2)
    d.ellipse([(12, 12), (18, 18)], fill=c4)
    d.rectangle([(13, 13), (15, 15)], fill=c5)

    # Tilted elliptical frost chain ring + sparks
    for i in range(16):
        ang = i * (2 * math.pi / 16.0)
        ex = 13.0 * math.cos(ang)
        ey = 5.5 * math.sin(ang)
        # Rotate -25 deg
        rx = int(round(16 + ex * 0.90 - ey * 0.42))
        ry = int(round(16 + ex * 0.42 + ey * 0.90))
        if i % 2 == 0:
            dt.rectangle([(rx, ry), (rx + 1, ry)], fill=c4)
        else:
            put_px(top, rx, ry, c5)

    return finalize(body, "#040E1A", top=top)


# ---------------------------------------------------------------------------
# 13. icon_brimstone.png — Brimstone Ray
# ---------------------------------------------------------------------------

def draw_brimstone() -> Image.Image:
    body = new_canvas()
    glow = new_canvas()
    d = ImageDraw.Draw(body)
    dg = ImageDraw.Draw(glow)

    s1, s2, s3, s4 = hex_rgba("#7F1D1D"), hex_rgba("#DC2626"), hex_rgba("#F97316"), hex_rgba("#FEF08A")

    dg.ellipse([(5, 4), (27, 28)], fill=hex_rgba("#DC2626", 55))

    # Upper double alchemical cross (vertical x=15..16, y=4..17; bars y=6, y=11)
    d.rectangle([(14, 4), (17, 18)], fill=s2)
    d.rectangle([(10, 6), (21, 8)], fill=s2)
    d.rectangle([(9, 11), (22, 13)], fill=s2)
    # Lower infinity loop (y=18..27, x=7..25)
    d.ellipse([(7, 18), (17, 27)], outline=s2, width=3)
    d.ellipse([(15, 18), (25, 27)], outline=s2, width=3)

    # Bright orange mid & 1px yellow plasma core
    d.line([(15, 5), (15, 17)], fill=s3, width=2)
    d.line([(16, 5), (16, 17)], fill=s4, width=1)
    d.line([(11, 7), (20, 7)], fill=s4, width=1)
    d.line([(10, 12), (21, 12)], fill=s4, width=1)
    d.ellipse([(8, 19), (16, 26)], outline=s3, width=1)
    d.ellipse([(16, 19), (24, 26)], outline=s3, width=1)
    put_px(body, 16, 22, s4)
    put_px(body, 10, 22, s4)
    put_px(body, 22, 22, s4)

    # Brimstone decay sparks
    for sx, sy in ((8, 5), (24, 9), (6, 25), (26, 21)):
        put_px(glow, sx, sy, s3)
        put_px(glow, sx, sy - 1, s1)

    return finalize(body, "#120508", glow=glow)


# ---------------------------------------------------------------------------
# 14. icon_missiles.png — Missile Swarm
# ---------------------------------------------------------------------------

def draw_missiles() -> Image.Image:
    body = new_canvas()
    d = ImageDraw.Draw(body)

    st, copper, brass = hex_rgba("#334155"), hex_rgba("#B45309"), hex_rgba("#F59E0B")
    f_r, f_o, f_y, smoke = hex_rgba("#DC2626"), hex_rgba("#EA580C"), hex_rgba("#FEF08A"), hex_rgba("#64748B")

    def draw_rocket(tx: int, ty: int, length: int) -> None:
        bx, by = tx - length, ty + length
        # Exhaust flame plume & smoke
        d.line([(bx, by), (bx - 5, by + 5)], fill=f_r, width=3)
        d.line([(bx, by), (bx - 4, by + 4)], fill=f_o, width=2)
        d.line([(bx, by), (bx - 2, by + 2)], fill=f_y, width=1)
        put_px(body, bx - 6, by + 6, smoke)
        # Tail stabilizer fins
        d.polygon([(bx + 1, by - 1), (bx - 2, by - 2), (bx + 2, by + 2)], fill=st)
        # Copper cylindrical body + steel nosecone
        d.line([(bx + 1, by - 1), (tx - 2, ty + 2)], fill=copper, width=4)
        d.line([(bx + 2, by - 2), (tx - 2, ty + 2)], fill=brass, width=1)
        d.polygon([(tx, ty), (tx - 3, ty + 1), (tx - 1, ty + 3)], fill=st)
        put_px(body, tx, ty, brass)

    # Two wingmen behind lead
    draw_rocket(15, 11, 5)
    draw_rocket(23, 19, 5)
    # Lead missile centered flying up-right to (26,6)
    draw_rocket(26, 6, 8)

    return finalize(body, "#0D0C14")


# ---------------------------------------------------------------------------
# 15. icon_chain_lightning.png — Chain Lightning
# ---------------------------------------------------------------------------

def draw_chain_lightning() -> Image.Image:
    body = new_canvas()
    glow = new_canvas()
    d = ImageDraw.Draw(body)
    dg = ImageDraw.Draw(glow)

    c_d, c_m, c_l, white = hex_rgba("#0E7490"), hex_rgba("#06B6D4"), hex_rgba("#67E8F9"), hex_rgba("#FFFFFF")

    main = [(5, 27), (10, 23), (12, 19), (15, 18), (17, 14), (22, 11), (27, 4)]
    forks = [
        [(12, 19), (9, 15), (6, 11)],
        [(15, 18), (18, 21), (20, 24)],
        [(22, 11), (24, 14), (26, 16)],
    ]

    dg.line(main, fill=hex_rgba("#06B6D4", 90), width=6)
    d.line(main, fill=c_d, width=3)
    d.line(main, fill=c_l, width=2)
    d.line(main, fill=white, width=1)

    for f in forks:
        d.line(f, fill=c_m, width=2)
        d.line(f, fill=c_l, width=1)
        put_px(body, f[-1][0], f[-1][1], white)

    return finalize(body, "#040D18", glow=glow)


# ---------------------------------------------------------------------------
# 16. icon_divine_bell.png — Divine Bell
# ---------------------------------------------------------------------------

def draw_divine_bell() -> Image.Image:
    body = new_canvas()
    top = new_canvas()
    d = ImageDraw.Draw(body)
    dt = ImageDraw.Draw(top)

    b_d, b_m, b_l, b_hi = hex_rgba("#451A03"), hex_rgba("#92400E"), hex_rgba("#D97706"), hex_rgba("#FDE68A")
    wave1, wave2 = hex_rgba("#FBBF24", 180), hex_rgba("#FBBF24", 110)

    # Top forged suspension loop (14,5)-(17,7)
    d.rectangle([(14, 4), (17, 7)], outline=b_l, width=1)
    put_px(body, 15, 4, b_hi)

    # Clapper below skirt (15,24)-(16,26)
    d.ellipse([(14, 23), (17, 26)], fill=b_m)
    put_px(body, 15, 24, b_hi)

    # Classic domed bell body flaring from top (13..18, y=8) to skirt (9..22, y=23)
    bell = [(13, 8), (18, 8), (20, 14), (23, 22), (22, 23), (9, 23), (8, 22), (11, 14)]
    d.polygon(bell, fill=b_l)
    d.polygon([(14, 9), (16, 9), (13, 21), (11, 21)], fill=b_hi)
    d.polygon([(18, 9), (22, 22), (19, 22), (17, 10)], fill=b_m)
    d.line([(9, 23), (22, 23)], fill=b_d)
    # Engraved crenellation along skirt
    for x in range(10, 22, 2):
        put_px(body, x, 21, b_d)

    # 2 concentric golden sound-wave arcs on each side
    dt.arc([(4, 11), (12, 23)], start=130, end=230, fill=wave1, width=1)
    dt.arc([(1, 9), (11, 25)], start=135, end=225, fill=wave2, width=1)
    dt.arc([(20, 11), (28, 23)], start=310, end=50, fill=wave1, width=1)
    dt.arc([(21, 9), (31, 25)], start=315, end=45, fill=wave2, width=1)

    return finalize(body, "#0F0C06", top=top)


# ---------------------------------------------------------------------------
# 17. icon_soul_phantoms.png — Soul Phantoms
# ---------------------------------------------------------------------------

def draw_soul_phantoms() -> Image.Image:
    body = new_canvas()
    glow = new_canvas()
    d = ImageDraw.Draw(body)
    dg = ImageDraw.Draw(glow)

    e1, e2, e3, e4, e5 = (
        hex_rgba("#064E3B"), hex_rgba("#059669"), hex_rgba("#34D399"), hex_rgba("#A7F3D0"), hex_rgba("#ECFDF5")
    )
    void = hex_rgba("#02140D")

    dg.ellipse([(4, 4), (28, 28)], fill=hex_rgba("#059669", 55))

    # Smaller secondary phantom at (10,20) tail to (16,28)
    d.polygon([(8, 22), (12, 22), (16, 28), (11, 27)], fill=e2)
    d.ellipse([(7, 17), (13, 23)], fill=e3)
    d.ellipse([(8, 18), (12, 21)], fill=e4)
    put_px(body, 9, 19, void)
    put_px(body, 11, 19, void)

    # Lead screaming spectral skull at (19,10) d=8, tail curving back to (9,21)
    d.polygon([(17, 13), (23, 14), (16, 19), (9, 21), (13, 16)], fill=e2)
    d.line([(18, 14), (10, 20)], fill=e3, width=2)
    d.ellipse([(15, 6), (24, 15)], fill=e3)
    d.ellipse([(16, 7), (23, 13)], fill=e4)
    d.rectangle([(18, 7), (21, 9)], fill=e5)
    # Hollow black eyes & gaping screaming jaw
    d.rectangle([(17, 9), (18, 10)], fill=void)
    d.rectangle([(21, 9), (22, 10)], fill=void)
    d.rectangle([(19, 12), (21, 14)], fill=void)

    put_px(glow, 13, 12, e1)
    put_px(glow, 25, 16, e4)

    return finalize(body, "#02140D", glow=glow)


# ---------------------------------------------------------------------------
# 18. icon_aegis_orbiters.png — Aegis Orbiters
# ---------------------------------------------------------------------------

def draw_aegis_orbiters() -> Image.Image:
    body = new_canvas()
    glow = new_canvas()
    d = ImageDraw.Draw(body)
    dg = ImageDraw.Draw(glow)

    st_d, st_m, st_l = hex_rgba("#1E293B"), hex_rgba("#94A3B8"), hex_rgba("#E2E8F0")
    f_d, f_m, f_n, f_l = hex_rgba("#1D4ED8"), hex_rgba("#3B82F6"), hex_rgba("#60A5FA"), hex_rgba("#DBEAFE")

    # Elliptical energy orbit connecting the shields
    dg.ellipse([(3, 8), (29, 24)], outline=f_m, width=3)
    d.ellipse([(3, 8), (29, 24)], outline=f_n, width=1)
    for nx, ny in ((16, 8), (16, 24)):
        d.rectangle([(nx - 1, ny - 1), (nx + 1, ny + 1)], fill=f_l)

    # Two symmetrical 8x12 teardrop shields at (6,10) and (18,10)
    for x0 in (6, 18):
        poly = [(x0, 10), (x0 + 8, 10), (x0 + 8, 17), (x0 + 4, 22), (x0, 17)]
        d.polygon(poly, fill=st_d, outline=st_l)
        d.polygon([(x0 + 1, 11), (x0 + 7, 11), (x0 + 7, 16), (x0 + 4, 20), (x0 + 1, 16)], fill=st_m)
        # Silver/azure heraldic cross
        d.line([(x0 + 4, 12), (x0 + 4, 19)], fill=f_l, width=1)
        d.line([(x0 + 2, 14), (x0 + 6, 14)], fill=f_l, width=1)
        put_px(body, x0 + 4, 14, f_d)

    return finalize(body, "#070A14", glow=glow)


# ---------------------------------------------------------------------------
# 19. icon_chaos_meteor.png — Chaos Meteor
# ---------------------------------------------------------------------------

def draw_chaos_meteor() -> Image.Image:
    body = new_canvas()
    glow = new_canvas()
    d = ImageDraw.Draw(body)
    dg = ImageDraw.Draw(glow)

    rock_d, rock_l = hex_rgba("#1C1917"), hex_rgba("#44403C")
    m1, m2, m3, m4 = hex_rgba("#7F1D1D"), hex_rgba("#DC2626"), hex_rgba("#F97316"), hex_rgba("#FEF08A")

    # Conical fire-and-ash tail trailing up-left to (5,5)
    dg.polygon([(24, 16), (16, 24), (4, 8), (8, 4)], fill=hex_rgba("#DC2626", 85))
    d.polygon([(22, 16), (16, 22), (5, 7), (7, 5)], fill=m2)
    d.polygon([(20, 17), (17, 20), (7, 8), (8, 7)], fill=m3)
    d.line([(18, 18), (9, 9)], fill=m4, width=2)

    # Smoke & ember sparks along tail
    for sx, sy, col in ((5, 11, rock_l), (11, 5, rock_l), (4, 5, m4), (12, 8, m4), (8, 13, m3)):
        put_px(body, sx, sy, col)

    # Irregular d=11 charred basalt asteroid centered at (20,20)
    rock = [(16, 15), (22, 14), (26, 18), (25, 24), (20, 26), (15, 23), (14, 18)]
    d.polygon(rock, fill=rock_d)
    d.polygon([(16, 16), (21, 15), (23, 18), (17, 19)], fill=rock_l)
    # Glowing magma cracks across asteroid surface
    d.line([(17, 17), (20, 20), (24, 19)], fill=m3, width=2)
    d.line([(20, 20), (19, 25)], fill=m3, width=1)
    d.line([(18, 18), (21, 20)], fill=m4, width=1)
    put_px(body, 22, 23, m1)

    return finalize(body, "#100505", glow=glow)


# ---------------------------------------------------------------------------
# 20. icon_void_rift.png — Void Rift
# ---------------------------------------------------------------------------

def draw_void_rift() -> Image.Image:
    body = new_canvas()
    glow = new_canvas()
    d = ImageDraw.Draw(body)
    dg = ImageDraw.Draw(glow)

    black, ether = hex_rgba("#000000"), hex_rgba("#2E1065")
    v1, v2, v3, v4 = hex_rgba("#581C87"), hex_rgba("#9333EA"), hex_rgba("#C084FC"), hex_rgba("#F5D0FE")

    # Outer ultraviolet spatial halo
    dg.ellipse([(7, 4), (25, 28)], fill=hex_rgba("#9333EA", 75))

    # 2 ghostly Void tentacles writhing outward from rift
    d.line([(14, 14), (9, 12), (5, 15), (4, 13)], fill=v2, width=2)
    d.line([(14, 14), (9, 12), (5, 15)], fill=v3, width=1)
    d.line([(18, 19), (23, 21), (27, 18), (28, 20)], fill=v2, width=2)
    d.line([(18, 19), (23, 21), (27, 18)], fill=v3, width=1)

    # Jagged vertical spindle rift (16,4)->(16,28), max width 8px at (16,16)
    outer = [(16, 3), (19, 9), (21, 13), (20, 16), (21, 20), (18, 24), (16, 29), (14, 24), (11, 20), (12, 16), (11, 12), (13, 8)]
    d.polygon(outer, fill=v2)
    d.polygon([(16, 4), (19, 11), (20, 16), (19, 21), (16, 28), (13, 21), (12, 16), (13, 11)], fill=v3)
    d.polygon([(16, 6), (18, 12), (19, 16), (18, 20), (16, 26), (14, 20), (13, 16), (14, 12)], fill=ether)
    d.polygon([(16, 8), (18, 16), (16, 24), (14, 16)], fill=black)

    # 4 starry micro-stars inside pitch-black rift
    for sx, sy in ((16, 11), (15, 15), (17, 18), (16, 21)):
        put_px(body, sx, sy, v4)
    put_px(body, 16, 4, v4)
    put_px(body, 16, 28, v4)

    return finalize(body, "#581C87", glow=glow)


# ---------------------------------------------------------------------------
# Batch Generation Pipeline
# ---------------------------------------------------------------------------

GENERATORS = [
    ("icon_wind_blade.png", draw_wind_blade),
    ("icon_fire_aura.png", draw_fire_aura),
    ("icon_magic_wand.png", draw_magic_wand),
    ("icon_holy_hammer.png", draw_holy_hammer),
    ("icon_ice_shards.png", draw_ice_shards),
    ("icon_poison_orbs.png", draw_poison_orbs),
    ("icon_executioner_axe.png", draw_executioner_axe),
    ("icon_celestial_bow.png", draw_celestial_bow),
    ("icon_thunder.png", draw_thunder),
    ("icon_blood_daggers.png", draw_blood_daggers),
    ("icon_singularity.png", draw_singularity),
    ("icon_chain_frost.png", draw_chain_frost),
    ("icon_brimstone.png", draw_brimstone),
    ("icon_missiles.png", draw_missiles),
    ("icon_chain_lightning.png", draw_chain_lightning),
    ("icon_divine_bell.png", draw_divine_bell),
    ("icon_soul_phantoms.png", draw_soul_phantoms),
    ("icon_aegis_orbiters.png", draw_aegis_orbiters),
    ("icon_chaos_meteor.png", draw_chaos_meteor),
    ("icon_void_rift.png", draw_void_rift),
]


def generate_all(out_root: Path) -> list[Path]:
    out_dirs = [out_root / "assets" / "icons", out_root / "assets" / "icons" / "weapons"]
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
        description="Generate 20 Basic Weapons 32x32 RGBA pixel-art icons (Group 7)."
    )
    parser.add_argument(
        "--out-root",
        type=Path,
        default=Path("."),
        help="Project root directory where assets/icons/ and assets/icons/weapons/ will be written.",
    )
    args = parser.parse_args()

    files = generate_all(args.out_root)
    print(f"Generated {len(files)} PNG files under {(args.out_root / 'assets' / 'icons').resolve()}")


if __name__ == "__main__":
    main()
