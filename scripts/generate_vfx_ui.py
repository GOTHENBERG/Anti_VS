#!/usr/bin/env python3
"""Procedural pixel-art generator for Projectiles, Hazard Zones & 9-Slice UI (Group 10).

Generates the final combat & UI asset kit (19 RGBA PNGs) in Dark Fantasy Gothic
Pixel Art style:
  - Part 1: 11 Weapon Projectiles (32x32)  -> assets/sprites/
  - Part 2:  4 Hazard Zones/Pools (64x64)  -> assets/sprites/ & assets/tiles/
  - Part 3:  4 9-Slice UI Frames           -> assets/ui/ & assets/sprites/

Usage:
    python scripts/generate_vfx_ui.py
    python scripts/generate_vfx_ui.py --out-root .
"""

from __future__ import annotations

import argparse
import math
from pathlib import Path
from typing import Callable, Tuple

from PIL import Image, ImageDraw

RGBA = Tuple[int, int, int, int]


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def hex_rgba(h: str, a: int = 255) -> RGBA:
    h = h.lstrip("#")
    return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16), a)


def new_canvas(w: int, h: int) -> Image.Image:
    return Image.new("RGBA", (w, h), (0, 0, 0, 0))


def put_px(img: Image.Image, x: int, y: int, col: RGBA) -> None:
    if 0 <= x < img.width and 0 <= y < img.height:
        img.putpixel((x, y), col)


def apply_1px_outline(layer: Image.Image, outline_color: RGBA) -> Image.Image:
    """Apply a crisp 4-connected 1px dark outline around opaque pixels."""
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


def finalize_proj(
    body: Image.Image,
    outline_hex: str = "#0A0812",
    glow: Image.Image | None = None,
    top: Image.Image | None = None,
) -> Image.Image:
    img = apply_1px_outline(body, hex_rgba(outline_hex))
    if glow is not None:
        img = Image.alpha_composite(glow, img)
    if top is not None:
        img = Image.alpha_composite(img, top)
    return img


# ===========================================================================
# PART 1: 11 Weapon Projectiles (32x32)
# ===========================================================================

def draw_proj_wind() -> Image.Image:
    body, glow = new_canvas(32, 32), new_canvas(32, 32)
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    c1, c2, c3, c4 = hex_rgba("#0F766E"), hex_rgba("#14B8A6"), hex_rgba("#2DD4BF"), hex_rgba("#F0FDFA")

    dg.ellipse([(5, 5), (27, 27)], fill=hex_rgba("#14B8A6", 65))
    # Crescent wind blade sweeping up-right (width ~24px)
    outer = [(6, 12), (14, 5), (24, 7), (27, 15), (22, 24), (13, 27), (18, 20), (20, 14), (15, 10)]
    d.polygon(outer, fill=c2)
    d.line([(6, 12), (14, 5), (24, 7), (27, 15), (22, 24), (13, 27)], fill=c3, width=2)
    d.line([(9, 9), (15, 5), (24, 7), (27, 15), (23, 22)], fill=c4, width=1)
    d.line([(15, 10), (20, 14), (18, 20)], fill=c1, width=1)
    # Vortex micro-sparks at crescent tips
    put_px(glow, 4, 13, c4)
    put_px(glow, 11, 28, c4)
    return finalize_proj(body, "#041410", glow=glow)


def draw_proj_fire() -> Image.Image:
    body, glow = new_canvas(32, 32), new_canvas(32, 32)
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    f1, f2, f3, f4, white = (
        hex_rgba("#7F1D1D"), hex_rgba("#DC2626"), hex_rgba("#F97316"), hex_rgba("#FEF08A"), hex_rgba("#FFFFFF")
    )

    dg.ellipse([(4, 7), (28, 25)], fill=hex_rgba("#F97316", 75))
    # Tapered conical flame tail stretching back to (4,16)
    d.polygon([(20, 10), (26, 16), (20, 22), (4, 16)], fill=f2)
    d.polygon([(20, 11), (25, 16), (20, 21), (8, 16)], fill=f3)
    d.polygon([(20, 13), (24, 16), (20, 19), (13, 16)], fill=f4)
    # Core sphere d=10 at (20,16)
    d.ellipse([(15, 11), (25, 21)], fill=f3)
    d.ellipse([(17, 13), (24, 19)], fill=f4)
    d.ellipse([(19, 14), (23, 18)], fill=white)
    # Trailing embers & licks
    put_px(body, 6, 12, f1)
    put_px(body, 7, 20, f2)
    put_px(glow, 3, 14, f4)
    put_px(glow, 5, 19, f3)
    return finalize_proj(body, "#140204", glow=glow)


def draw_proj_magic() -> Image.Image:
    body, glow = new_canvas(32, 32), new_canvas(32, 32)
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    b1, b2, b3, b4 = hex_rgba("#1E3A8A"), hex_rgba("#2563EB"), hex_rgba("#60A5FA"), hex_rgba("#EFF6FF")

    dg.ellipse([(5, 9), (27, 23)], fill=hex_rgba("#2563EB", 75))
    # Two spiral arcane trails around axis
    d.line([(4, 13), (9, 11), (15, 14), (19, 12)], fill=b3, width=1)
    d.line([(4, 19), (9, 21), (15, 18), (19, 20)], fill=b3, width=1)
    # Horizontal diamond crystal bolt (7,16) -> (25,16)
    d.polygon([(7, 16), (17, 11), (26, 16), (17, 21)], fill=b2)
    d.polygon([(9, 16), (17, 12), (25, 16), (17, 16)], fill=b3)
    d.polygon([(17, 16), (25, 16), (17, 20)], fill=b1)
    d.line([(12, 16), (25, 16)], fill=b4, width=1)
    put_px(body, 25, 16, b4)
    return finalize_proj(body, "#060618", glow=glow)


def draw_proj_holy() -> Image.Image:
    body, glow = new_canvas(32, 32), new_canvas(32, 32)
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    g1, g2, g3, white = hex_rgba("#B45309"), hex_rgba("#F59E0B"), hex_rgba("#FEF08A"), hex_rgba("#FFFFFF")

    dg.ellipse([(5, 5), (27, 27)], fill=hex_rgba("#FEF08A", 75))
    # Symmetric 4-pointed star cross d=18 with thick center
    d.polygon([(16, 5), (19, 13), (27, 16), (19, 19), (16, 27), (13, 19), (5, 16), (13, 13)], fill=g2)
    d.polygon([(16, 7), (18, 14), (25, 16), (18, 18), (16, 25), (14, 18), (7, 16), (14, 14)], fill=g3)
    d.line([(16, 7), (16, 25)], fill=white, width=1)
    d.line([(7, 16), (25, 16)], fill=white, width=1)
    d.ellipse([(14, 14), (18, 18)], fill=white)
    # Golden corner impulse sparks
    for sx, sy in ((9, 9), (23, 9), (9, 23), (23, 23)):
        put_px(glow, sx, sy, g1)
        put_px(glow, sx + (1 if sx < 16 else -1), sy + (1 if sy < 16 else -1), g3)
    return finalize_proj(body, "#0E0C04", glow=glow)


def draw_proj_ice() -> Image.Image:
    body, glow = new_canvas(32, 32), new_canvas(32, 32)
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    i1, i2, i3, i4, i5 = (
        hex_rgba("#0C4A6E"), hex_rgba("#0284C7"), hex_rgba("#38BDF8"), hex_rgba("#BAE6FD"), hex_rgba("#F0F9FF")
    )

    dg.line([(5, 27), (27, 5)], fill=hex_rgba("#38BDF8", 80), width=8)
    # Diagonal faceted crystal spike (5,27)->(27,5), length 22, width 8
    d.polygon([(5, 27), (11, 15), (27, 5), (17, 21)], fill=i2)
    d.polygon([(5, 27), (11, 15), (27, 5)], fill=i3)
    d.polygon([(5, 27), (17, 21), (27, 5)], fill=i1)
    d.line([(6, 26), (27, 5)], fill=i5, width=1)
    d.line([(11, 15), (15, 17)], fill=i4)
    # Side frost micro-crystal spurs
    d.polygon([(12, 14), (10, 10), (15, 12)], fill=i4)
    d.polygon([(18, 20), (22, 22), (20, 17)], fill=i3)
    return finalize_proj(body, "#030F1C", glow=glow)


def draw_proj_poison() -> Image.Image:
    body, glow = new_canvas(32, 32), new_canvas(32, 32)
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    p1, p2, p3, p4, p5 = (
        hex_rgba("#14532D"), hex_rgba("#4D7C0F"), hex_rgba("#84CC16"), hex_rgba("#BEF264"), hex_rgba("#ECFCCB")
    )

    dg.ellipse([(6, 6), (27, 26)], fill=hex_rgba("#84CC16", 65))
    # Organic acid teardrop flying right with trailing tail
    d.polygon([(4, 16), (17, 9), (24, 12), (26, 16), (24, 20), (17, 23)], fill=p2)
    d.ellipse([(12, 9), (26, 23)], fill=p3)
    d.ellipse([(14, 11), (24, 21)], fill=p4)
    d.arc([(13, 10), (25, 22)], start=10, end=160, fill=p1, width=2)
    # 2 bright gas bubbles inside & falling drop
    d.ellipse([(16, 12), (19, 15)], fill=p5)
    d.ellipse([(20, 17), (22, 19)], fill=p5)
    d.ellipse([(16, 25), (18, 27)], fill=p3)
    put_px(body, 16, 25, p4)
    return finalize_proj(body, "#0A1405", glow=glow)


def draw_proj_axe() -> Image.Image:
    body = new_canvas(32, 32)
    d = ImageDraw.Draw(body)
    s1, s2, s3 = hex_rgba("#1E293B"), hex_rgba("#475569"), hex_rgba("#94A3B8")
    blood, oak = hex_rgba("#991B1B"), hex_rgba("#451A03")

    # Short vertical haft centered at (16,16)
    d.rectangle([(15, 6), (17, 26)], fill=oak)
    # Symmetric double-sided crescent blades centered at (16,16)
    left_b = [(15, 12), (7, 7), (4, 16), (7, 25), (15, 20)]
    right_b = [(17, 12), (25, 7), (28, 16), (25, 25), (17, 20)]
    for poly, edge, bx in (
        (left_b, [(7, 7), (4, 16), (7, 25)], 6),
        (right_b, [(25, 7), (28, 16), (25, 25)], 26),
    ):
        d.polygon(poly, fill=s2)
        d.line(edge, fill=s3, width=2)
        d.line([(bx, 18), (bx, 23)], fill=blood, width=2)
    d.rectangle([(14, 13), (18, 19)], fill=s1)
    put_px(body, 16, 16, s3)
    return finalize_proj(body, "#100508")


def draw_proj_arrow() -> Image.Image:
    body, glow = new_canvas(32, 32), new_canvas(32, 32)
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    shaft = hex_rgba("#CBD5E1")
    tip1, tip2, white = hex_rgba("#0284C7"), hex_rgba("#38BDF8"), hex_rgba("#FFFFFF")
    f1, f2 = hex_rgba("#F59E0B"), hex_rgba("#FEF08A")

    dg.line([(3, 16), (29, 16)], fill=hex_rgba("#38BDF8", 75), width=6)
    # Golden feather fletching on left (3,16)
    d.polygon([(3, 12), (10, 15), (10, 17), (3, 20), (6, 16)], fill=f1)
    d.line([(5, 14), (9, 16)], fill=f2)
    d.line([(5, 18), (9, 16)], fill=f2)
    # Platinum horizontal shaft (3,16)->(29,16)
    d.line([(5, 16), (25, 16)], fill=shaft, width=2)
    # Three-edged diamond azure arrowhead
    d.polygon([(21, 12), (29, 16), (21, 20), (23, 16)], fill=tip1)
    d.polygon([(22, 13), (29, 16), (23, 16)], fill=tip2)
    d.line([(23, 16), (29, 16)], fill=white)
    return finalize_proj(body, "#080B14", glow=glow)


def draw_proj_lightning() -> Image.Image:
    body, glow = new_canvas(32, 32), new_canvas(32, 32)
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    c1, c2, c3, white = hex_rgba("#0E7490"), hex_rgba("#06B6D4"), hex_rgba("#67E8F9"), hex_rgba("#FFFFFF")

    bolt = [(5, 16), (10, 11), (15, 20), (21, 12), (27, 16)]
    dg.line(bolt, fill=hex_rgba("#06B6D4", 95), width=6)
    d.line(bolt, fill=c1, width=3)
    d.line(bolt, fill=c3, width=2)
    d.line(bolt, fill=white, width=1)
    # 2 side micro-forks
    d.line([(10, 11), (13, 6)], fill=c2, width=1)
    d.line([(15, 20), (19, 25)], fill=c2, width=1)
    put_px(body, 13, 6, white)
    put_px(body, 19, 25, white)
    return finalize_proj(body, "#040D18", glow=glow)


def draw_proj_dagger() -> Image.Image:
    body = new_canvas(32, 32)
    d = ImageDraw.Draw(body)
    st, r1, r2, r3, gold = (
        hex_rgba("#18181B"), hex_rgba("#7F1D1D"), hex_rgba("#B91C1C"), hex_rgba("#EF4444"), hex_rgba("#D97706")
    )

    # Diagonal double-edged blood dagger (6,26)->(26,6)
    d.line([(6, 26), (11, 21)], fill=st, width=2)
    d.line([(9, 19), (13, 23)], fill=gold, width=2)
    d.polygon([(11, 19), (26, 6), (13, 21)], fill=st)
    d.polygon([(12, 18), (26, 6), (14, 20)], fill=r2)
    d.line([(13, 19), (26, 6)], fill=r3, width=1)
    put_px(body, 11, 21, r1)
    put_px(body, 25, 7, hex_rgba("#FCA5A5"))
    return finalize_proj(body, "#140306")


def draw_proj_scythe() -> Image.Image:
    body, glow = new_canvas(32, 32), new_canvas(32, 32)
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    e1, e2, e3, e4, e5 = (
        hex_rgba("#064E3B"), hex_rgba("#059669"), hex_rgba("#10B981"), hex_rgba("#6EE7B7"), hex_rgba("#ECFDF5")
    )

    dg.ellipse([(5, 5), (27, 27)], fill=hex_rgba("#10B981", 65))
    # Crescent spectral scythe blade (6,24) -> (25,20) -> (22,6) with inner serrations
    blade = [(6, 24), (16, 24), (25, 20), (27, 12), (22, 6), (22, 13), (18, 18), (11, 20)]
    d.polygon(blade, fill=e2)
    d.line([(6, 24), (16, 24), (25, 20), (27, 12), (22, 6)], fill=e4, width=2)
    d.line([(15, 24), (25, 20), (27, 12), (22, 6)], fill=e5, width=1)
    d.line([(11, 20), (18, 18), (22, 13)], fill=e1, width=1)
    # Inner serration notches
    for nx, ny in ((15, 19), (20, 16), (22, 11)):
        put_px(body, nx, ny, e3)
    return finalize_proj(body, "#03140C", glow=glow)


# ===========================================================================
# PART 2: 4 Hazard Zones & Pools (64x64)
# ===========================================================================

def draw_hazard_poison_pool() -> Image.Image:
    img = new_canvas(64, 64)
    d = ImageDraw.Draw(img)
    deep = hex_rgba("#0D2E14", 200)
    mid = hex_rgba("#22C55E", 220)
    hi = hex_rgba("#4ADE80", 230)
    bub = hex_rgba("#A3E635", 245)
    glint = hex_rgba("#F0FDF4", 255)

    cx, cy = 32.0, 32.0
    px = img.load()
    for y in range(6, 58):
        for x in range(6, 58):
            dx, dy = (x - cx), (y - cy) * 1.15
            ang = math.atan2(dy, dx)
            dist = math.hypot(dx, dy)
            rad = 22.0 + 3.0 * math.sin(ang * 3 + 0.4) + 2.0 * math.cos(ang * 5 - 0.7)
            if dist <= rad:
                if dist > rad - 2.5:
                    px[x, y] = deep
                elif dist > rad - 6.0:
                    px[x, y] = mid
                elif dist > rad - 9.0:
                    px[x, y] = hi
                else:
                    px[x, y] = mid

    # Concentric ripple & 6 boiling bubbles (3–5px)
    d.ellipse([(18, 21), (46, 43)], outline=hi, width=1)
    for bx, by, r in ((22, 27, 2), (34, 24, 3), (42, 31, 2), (28, 38, 3), (38, 40, 2), (24, 34, 1)):
        d.ellipse([(bx - r, by - r), (bx + r, by + r)], fill=bub, outline=deep)
        put_px(img, bx - 1, by - 1, glint)
    return img


def draw_hazard_miasma_pool() -> Image.Image:
    img = new_canvas(64, 64)
    d = ImageDraw.Draw(img)
    deep = hex_rgba("#2E1065", 200)
    mid = hex_rgba("#7E22CE", 220)
    hi = hex_rgba("#C084FC", 225)
    spore = hex_rgba("#E879F9", 245)
    ether = hex_rgba("#FDF4FF", 255)

    cx, cy = 31.0, 32.0
    px = img.load()
    for y in range(8, 56):
        for x in range(8, 56):
            dx, dy = (x - cx), (y - cy) * 1.1
            ang = math.atan2(dy, dx)
            dist = math.hypot(dx, dy)
            rad = 19.0 + 3.5 * math.sin(ang * 4 - 0.5) + 1.8 * math.cos(ang * 2 + 1.1)
            if dist <= rad:
                if dist > rad - 2.5:
                    px[x, y] = deep
                elif dist > rad - 7.0:
                    px[x, y] = mid
                else:
                    px[x, y] = hi if int(dist) % 4 == 0 else mid

    # 4 satellite miasma puddles around main reservoir
    for sx, sy, sr in ((11, 17, 5), (52, 19, 4), (50, 47, 5), (14, 49, 4)):
        d.ellipse([(sx - sr, sy - sr), (sx + sr, sy + sr)], fill=deep)
        d.ellipse([(sx - sr + 1, sy - sr + 1), (sx + sr - 1, sy + sr - 1)], fill=mid)
        put_px(img, sx, sy, spore)

    # Swirling micro-vortex & popping spores in center
    d.arc([(22, 24), (40, 40)], start=20, end=250, fill=spore, width=1)
    for bx, by in ((27, 29), (35, 34), (31, 26), (37, 28)):
        d.ellipse([(bx - 1, by - 1), (bx + 1, by + 1)], fill=deep, outline=spore)
        put_px(img, bx, by, ether)
    return img


def draw_hazard_fire_pool() -> Image.Image:
    img = new_canvas(64, 64)
    d = ImageDraw.Draw(img)
    rock = hex_rgba("#1C1917", 240)
    ash = hex_rgba("#44403C", 230)
    m1 = hex_rgba("#991B1B", 240)
    m2 = hex_rgba("#EA580C", 245)
    m3 = hex_rgba("#F59E0B", 250)
    m4 = hex_rgba("#FEF08A", 255)

    # Charred basalt crater d=48
    d.ellipse([(8, 8), (56, 56)], fill=rock)
    d.ellipse([(11, 11), (53, 53)], fill=ash)
    d.ellipse([(14, 14), (50, 50)], fill=rock)

    # Network of intersecting magma cracks radiating from core
    cracks = [
        [(32, 32), (18, 16)], [(32, 32), (48, 18)], [(32, 32), (50, 42)],
        [(32, 32), (16, 46)], [(32, 32), (32, 11)], [(32, 32), (34, 52)],
    ]
    for path in cracks:
        d.line(path, fill=m1, width=4)
        d.line(path, fill=m2, width=2)
        d.line(path, fill=m3, width=1)

    # Boiling liquid fire lake in crater core
    d.ellipse([(20, 20), (44, 44)], fill=m1)
    d.ellipse([(22, 22), (42, 42)], fill=m2)
    d.ellipse([(25, 25), (39, 39)], fill=m3)
    d.ellipse([(28, 28), (36, 36)], fill=m4)
    return img


def draw_hazard_danger_circle() -> Image.Image:
    img = new_canvas(64, 64)
    d = ImageDraw.Draw(img)
    fill_outer = hex_rgba("#7F1D1D", 45)
    fill_inner = hex_rgba("#7F1D1D", 80)
    ring_red = hex_rgba("#EF4444", 240)
    ring_hi = hex_rgba("#F87171", 255)
    mark = hex_rgba("#FCA5A5", 240)

    # Translucent crimson telegraph gradient fill (diameter 56 centered at 32,32)
    d.ellipse([(4, 4), (60, 60)], fill=fill_outer)
    d.ellipse([(14, 14), (50, 50)], fill=fill_inner)

    # 16-sector dashed gothic outer border (width 2px)
    for i in range(16):
        start_deg = i * 22.5 + 3
        end_deg = (i + 1) * 22.5 - 4
        d.arc([(4, 4), (60, 60)], start=start_deg, end=end_deg, fill=ring_red, width=3)
        d.arc([(5, 5), (59, 59)], start=start_deg + 1, end=end_deg - 1, fill=ring_hi, width=1)

    # Inner targeting ring & central exclamation/crosshair warning rune
    d.ellipse([(20, 20), (44, 44)], outline=hex_rgba("#EF4444", 150), width=1)
    d.line([(32, 16), (32, 22)], fill=mark, width=1)
    d.line([(32, 42), (32, 48)], fill=mark, width=1)
    d.line([(16, 32), (22, 32)], fill=mark, width=1)
    d.line([(42, 32), (48, 32)], fill=mark, width=1)
    # Crisp exclamation mark (!)
    d.rectangle([(31, 25), (33, 34)], fill=mark)
    d.rectangle([(31, 37), (33, 39)], fill=mark)
    return img


# ===========================================================================
# PART 3: 4 Gothic 9-Slice UI Frames
# ===========================================================================

def draw_ui_frame_gothic_card() -> Image.Image:
    """48x48 9-slice gothic card (corners 12x12, edges 24x12, center 24x24)."""
    img = new_canvas(48, 48)
    d = ImageDraw.Draw(img)
    outline = hex_rgba("#0A0812")
    bg = hex_rgba("#12101C", 245)
    g_dark, g_mid, g_hi = hex_rgba("#92400E"), hex_rgba("#D97706"), hex_rgba("#FDE68A")
    ruby = hex_rgba("#EF4444")

    # Outer frame & dark slate center
    d.rectangle([(0, 0), (47, 47)], fill=bg, outline=outline)
    # Double metallic edge profile (uniform along stretch spans x=12..35 and y=12..35)
    d.rectangle([(1, 1), (46, 46)], outline=g_dark)
    d.rectangle([(2, 2), (45, 45)], outline=g_mid)
    d.rectangle([(3, 3), (44, 44)], outline=outline)
    d.rectangle([(4, 4), (43, 43)], outline=g_dark)
    # Top-left light bevel & bottom-right shadow bevel
    d.line([(2, 2), (45, 2)], fill=g_hi)
    d.line([(2, 2), (2, 45)], fill=g_hi)
    d.line([(2, 45), (45, 45)], fill=g_dark)
    d.line([(45, 2), (45, 45)], fill=g_dark)

    # Ornate 12x12 gothic filigree corners with 2x2 ruby rivets
    corners = [(0, 0, 1, 1), (47, 0, -1, 1), (0, 47, 1, -1), (47, 47, -1, -1)]
    for cx, cy, sx, sy in corners:
        # Filigree corner bracket
        bx0, bx1 = sorted([cx + sx * 1, cx + sx * 9])
        by0, by1 = sorted([cy + sy * 1, cy + sy * 9])
        d.rectangle([(bx0, by0), (bx1, by1)], outline=g_mid)
        d.line([(cx + sx * 5, cy + sy * 9), (cx + sx * 9, cy + sy * 5)], fill=g_hi)
        # 2x2 ruby rivet inside corner
        rx0, rx1 = sorted([cx + sx * 4, cx + sx * 5])
        ry0, ry1 = sorted([cy + sy * 4, cy + sy * 5])
        d.rectangle([(rx0, ry0), (rx1, ry1)], fill=ruby)
        put_px(img, rx0, ry0, g_hi)
        # Cut 1px outer pixel for subtle gothic silhouette
        put_px(img, cx, cy, (0, 0, 0, 0))
    return img


def draw_ui_frame_slot() -> Image.Image:
    """32x32 9-slice HUD slot (corners 8x8, edges 16x8, center 16x16)."""
    img = new_canvas(32, 32)
    d = ImageDraw.Draw(img)
    bg = hex_rgba("#0C0A14", 245)
    steel = hex_rgba("#1E293B")
    silver = hex_rgba("#64748B")
    gold = hex_rgba("#F59E0B")
    outline = hex_rgba("#0A0812")

    d.rectangle([(0, 0), (31, 31)], fill=bg, outline=outline)
    d.rectangle([(1, 1), (30, 30)], outline=steel, width=2)
    d.rectangle([(3, 3), (28, 28)], outline=outline)
    # Recessed inner bevel (dark top-left shadow, silver bottom-right rim)
    d.line([(1, 1), (30, 1)], fill=silver)
    d.line([(1, 1), (1, 30)], fill=silver)
    d.line([(4, 4), (27, 4)], fill=hex_rgba("#05040A"))
    d.line([(4, 4), (4, 27)], fill=hex_rgba("#05040A"))

    # 4 corner 1px gold rivets inside 8x8 corners
    for rx, ry in ((2, 2), (29, 2), (2, 29), (29, 29)):
        put_px(img, rx, ry, gold)
    return img


def draw_ui_frame_tooltip() -> Image.Image:
    """48x48 9-slice tooltip frame (corners 10x10, edges 28x10, center 28x28)."""
    img = new_canvas(48, 48)
    d = ImageDraw.Draw(img)
    outline = hex_rgba("#080612")
    bg = hex_rgba("#0B0916", 248)
    am_dark = hex_rgba("#7C3AED")
    am_hi = hex_rgba("#C084FC")
    glow_c = hex_rgba("#7C3AED", 90)

    d.rectangle([(0, 0), (47, 47)], fill=bg, outline=outline)
    # Clean 2px amethyst-silver border
    d.rectangle([(1, 1), (46, 46)], outline=am_dark)
    d.rectangle([(2, 2), (45, 45)], outline=am_hi)
    d.rectangle([(3, 3), (44, 44)], outline=outline)

    # Subtle violet corner glow accents inside 10x10 corners
    for cx, cy, sx, sy in ((0, 0, 1, 1), (47, 0, -1, 1), (0, 47, 1, -1), (47, 47, -1, -1)):
        x0, x1 = sorted([cx + sx * 4, cx + sx * 7])
        y0, y1 = sorted([cy + sy * 4, cy + sy * 7])
        d.rectangle([(x0, y0), (x1, y1)], fill=glow_c)
        put_px(img, cx + sx * 2, cy + sy * 2, hex_rgba("#FDF4FF"))
        put_px(img, cx, cy, (0, 0, 0, 0))
    return img


def draw_ui_button_gothic() -> Image.Image:
    """48x32 9-slice chamfered gothic button (corners 8x8, x-edge 32, y-edge 16)."""
    img = new_canvas(48, 32)
    d = ImageDraw.Draw(img)
    outline = hex_rgba("#0B0914")
    base = hex_rgba("#1F2937")
    press = hex_rgba("#374151")
    g_mid = hex_rgba("#D97706")
    g_hi = hex_rgba("#FDE68A")
    g_dark = hex_rgba("#92400E")

    # Chamfered octagonal button silhouette
    poly = [(3, 0), (44, 0), (47, 3), (47, 28), (44, 31), (3, 31), (0, 28), (0, 3)]
    d.polygon(poly, fill=base, outline=outline)

    # Upper half lit press surface + center horizontal relief ridge (uniform across x=8..39)
    d.rectangle([(3, 3), (44, 15)], fill=press)
    d.line([(3, 16), (44, 16)], fill=hex_rgba("#4B5563"))
    d.line([(3, 17), (44, 17)], fill=hex_rgba("#111827"))

    # Golden perimeter trim with top highlight & bottom shadow
    inner_poly = [(4, 1), (43, 1), (46, 4), (46, 27), (43, 30), (4, 30), (1, 27), (1, 4)]
    d.polygon(inner_poly, outline=g_mid)
    d.line([(4, 1), (43, 1)], fill=g_hi)
    d.line([(4, 2), (43, 2)], fill=g_mid)
    d.line([(4, 30), (43, 30)], fill=g_dark)

    # Corner gold rivets inside 8x8 corners
    for rx, ry in ((5, 5), (42, 5), (5, 26), (42, 26)):
        put_px(img, rx, ry, g_hi)
    return img


# ---------------------------------------------------------------------------
# Batch Generation Pipeline
# ---------------------------------------------------------------------------

PROJECTILES: list[tuple[str, Callable[[], Image.Image]]] = [
    ("proj_wind.png", draw_proj_wind),
    ("proj_fire.png", draw_proj_fire),
    ("proj_magic.png", draw_proj_magic),
    ("proj_holy.png", draw_proj_holy),
    ("proj_ice.png", draw_proj_ice),
    ("proj_poison.png", draw_proj_poison),
    ("proj_axe.png", draw_proj_axe),
    ("proj_arrow.png", draw_proj_arrow),
    ("proj_lightning.png", draw_proj_lightning),
    ("proj_dagger.png", draw_proj_dagger),
    ("proj_scythe.png", draw_proj_scythe),
]

HAZARDS: list[tuple[str, Callable[[], Image.Image]]] = [
    ("hazard_poison_pool.png", draw_hazard_poison_pool),
    ("hazard_miasma_pool.png", draw_hazard_miasma_pool),
    ("hazard_fire_pool.png", draw_hazard_fire_pool),
    ("hazard_danger_circle.png", draw_hazard_danger_circle),
]

UI_FRAMES: list[tuple[str, Callable[[], Image.Image]]] = [
    ("ui_frame_gothic_card.png", draw_ui_frame_gothic_card),
    ("ui_frame_slot.png", draw_ui_frame_slot),
    ("ui_frame_tooltip.png", draw_ui_frame_tooltip),
    ("ui_button_gothic.png", draw_ui_button_gothic),
]


def generate_all(out_root: Path) -> list[Path]:
    sprites_dir = out_root / "assets" / "sprites"
    tiles_dir = out_root / "assets" / "tiles"
    ui_dir = out_root / "assets" / "ui"

    for d in (sprites_dir, tiles_dir, ui_dir):
        d.mkdir(parents=True, exist_ok=True)

    saved: list[Path] = []
    groups = [
        (PROJECTILES, [sprites_dir]),
        (HAZARDS, [sprites_dir, tiles_dir]),
        (UI_FRAMES, [ui_dir, sprites_dir]),
    ]
    for items, dirs in groups:
        for filename, fn in items:
            img = fn()
            for target in dirs:
                path = target / filename
                img.save(path, format="PNG")
                saved.append(path)
    return saved


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Generate Group 10 Projectiles, Hazard Zones & 9-Slice Gothic UI (RGBA)."
    )
    parser.add_argument(
        "--out-root",
        type=Path,
        default=Path("."),
        help="Project root directory where assets/sprites/, assets/tiles/, and assets/ui/ will be written.",
    )
    args = parser.parse_args()

    files = generate_all(args.out_root)
    print(f"Generated {len(files)} PNG files under {(args.out_root / 'assets').resolve()}")


if __name__ == "__main__":
    main()