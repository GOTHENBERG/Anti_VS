#!/usr/bin/env python3
"""Procedural pixel-art generator for 28 Passive Relics + 6 Limit Break Perks (Group 9).

Generates 34 crisp 32x32 RGBA pixel-art icons in Dark Fantasy Gothic style:
  - Part 1: 14 Basic Passives     -> assets/icons/ & assets/icons/passives/
  - Part 2: 14 Legendary Relics   -> assets/icons/ & assets/icons/legendary/
  - Part 3:  6 Limit Break Perks  -> assets/icons/ & assets/icons/limit_break/

Usage:
    python scripts/generate_passives.py
    python scripts/generate_passives.py --out-root .
"""

from __future__ import annotations

import argparse
import math
from pathlib import Path
from typing import Callable, Tuple

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


def add_lb_laurel(d: ImageDraw.ImageDraw, body: Image.Image) -> None:
    """Subtle golden laurel corner crests marking Limit Break perks."""
    g1, g2 = hex_rgba("#D97706"), hex_rgba("#FDE047")
    for pts in (
        [(2, 7), (2, 2), (7, 2)],
        [(29, 7), (29, 2), (24, 2)],
        [(2, 24), (2, 29), (7, 29)],
        [(29, 24), (29, 29), (24, 29)],
    ):
        d.line(pts, fill=g1, width=1)
        put_px(body, pts[1][0], pts[1][1], g2)


# ===========================================================================
# PART 1: 14 Basic Passives
# ===========================================================================

def draw_speed_gloves() -> Image.Image:
    body, top = new_canvas(), new_canvas()
    d, dt = ImageDraw.Draw(body), ImageDraw.Draw(top)
    s1, s2, s3 = hex_rgba("#334155"), hex_rgba("#94A3B8"), hex_rgba("#E2E8F0")
    g1, g2, wind = hex_rgba("#D97706"), hex_rgba("#FDE68A"), hex_rgba("#2DD4BF")

    # Clenched steel gauntlet (8,6)-(24,26)
    d.rectangle([(10, 7), (22, 16)], fill=s2)
    d.line([(10, 7), (22, 7)], fill=s3)
    for fx in (13, 16, 19):
        d.line([(fx, 7), (fx, 13)], fill=s1)
        put_px(body, fx - 1, 10, g2)  # gold knuckles
    # Thumb & cuff
    d.rectangle([(8, 12), (11, 17)], fill=s2)
    d.polygon([(11, 16), (21, 16), (23, 25), (9, 25)], fill=s1)
    d.line([(11, 17), (21, 17)], fill=g1, width=2)
    d.line([(12, 17), (20, 17)], fill=g2, width=1)
    # 2 golden aerodynamic wings from cuff + wind trail
    d.polygon([(9, 19), (3, 15), (5, 22), (10, 23)], fill=g1)
    d.line([(3, 15), (9, 19)], fill=g2)
    d.polygon([(23, 19), (29, 15), (27, 22), (22, 23)], fill=g1)
    d.line([(29, 15), (23, 19)], fill=g2)
    dt.line([(2, 12), (7, 13)], fill=wind)
    dt.line([(25, 13), (30, 12)], fill=wind)

    return finalize(body, "#0A0812", top=top)


def draw_giant_heart() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    r1, r2, o1, y1, w1 = (
        hex_rgba("#7F1D1D"), hex_rgba("#DC2626"), hex_rgba("#F97316"), hex_rgba("#FEF08A"), hex_rgba("#FFFBEB")
    )
    dg.ellipse([(5, 5), (27, 27)], fill=hex_rgba("#DC2626", 65))
    d.ellipse([(9, 8), (17, 18)], fill=r2)
    d.ellipse([(15, 8), (23, 18)], fill=r2)
    d.polygon([(9, 14), (23, 14), (16, 25)], fill=r1)
    d.polygon([(11, 13), (21, 13), (16, 22)], fill=r2)
    # Glowing magma & gold arteries
    d.line([(12, 10), (16, 16), (21, 12)], fill=o1, width=2)
    d.line([(16, 16), (16, 23)], fill=o1, width=2)
    d.rectangle([(15, 15), (17, 17)], fill=y1)
    put_px(body, 16, 16, w1)
    return finalize(body, "#100508", glow=glow)


def draw_cooldown_tome() -> Image.Image:
    body, top = new_canvas(), new_canvas()
    d = ImageDraw.Draw(body)
    b1, b2 = hex_rgba("#1E3A8A"), hex_rgba("#2563EB")
    g1, g2, page, rib = hex_rgba("#D97706"), hex_rgba("#FBBF24"), hex_rgba("#FEF3C7"), hex_rgba("#DC2626")

    # Angled sapphire grimoire 18x18
    d.polygon([(7, 8), (22, 6), (25, 22), (10, 25)], fill=b1)
    d.polygon([(9, 9), (21, 7), (23, 20), (11, 22)], fill=b2)
    # Parchment pages edge & crimson bookmark
    d.line([(10, 24), (24, 21)], fill=page, width=2)
    d.line([(15, 24), (14, 28)], fill=rib, width=2)
    # Embossed golden hourglass on cover
    d.polygon([(13, 11), (19, 11), (16, 15), (19, 19), (13, 19), (16, 15)], fill=g2)
    d.line([(13, 11), (19, 11)], fill=g1)
    d.line([(13, 19), (19, 19)], fill=g1)
    # 2 floating magic runes
    put_px(top, 25, 9, g2)
    put_px(top, 27, 13, page)
    return finalize(body, "#060814", top=top)


def draw_light_armor() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    s1, s2, s3 = hex_rgba("#475569"), hex_rgba("#94A3B8"), hex_rgba("#F8FAFC")
    g1, g2 = hex_rgba("#F59E0B"), hex_rgba("#FEF08A")

    dg.ellipse([(5, 6), (27, 27)], fill=hex_rgba("#FEF08A", 55))
    # Pauldrons & cuirass (6,8)-(26,26)
    d.polygon([(6, 10), (11, 8), (21, 8), (26, 10), (25, 16), (22, 16), (21, 25), (16, 27), (11, 25), (10, 16), (7, 16)], fill=s2)
    d.line([(7, 10), (25, 10)], fill=s3, width=2)
    d.line([(21, 15), (21, 25)], fill=s1, width=2)
    d.line([(11, 25), (16, 27), (21, 25)], fill=s1, width=1)
    # Golden solar cross on chest
    d.rectangle([(15, 11), (17, 22)], fill=g1)
    d.rectangle([(12, 14), (20, 16)], fill=g1)
    d.line([(16, 12), (16, 21)], fill=g2)
    d.line([(13, 15), (19, 15)], fill=g2)
    return finalize(body, "#0B0C14", glow=glow)


def draw_frost_ring() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    s1, s2 = hex_rgba("#64748B"), hex_rgba("#CBD5E1")
    c1, c2, c3, white = hex_rgba("#0284C7"), hex_rgba("#38BDF8"), hex_rgba("#BAE6FD"), hex_rgba("#FFFFFF")

    dg.ellipse([(9, 4), (23, 18)], fill=hex_rgba("#38BDF8", 70))
    # Perspective silver ring band 14x10
    d.ellipse([(9, 14), (23, 25)], outline=s1, width=3)
    d.ellipse([(9, 14), (23, 24)], outline=s2, width=1)
    # Octagonal ice crystal crown
    d.polygon([(16, 5), (21, 10), (19, 15), (13, 15), (11, 10)], fill=c1)
    d.polygon([(16, 6), (20, 10), (16, 14), (12, 10)], fill=c2)
    d.polygon([(16, 6), (12, 10), (16, 10)], fill=c3)
    put_px(body, 15, 8, white)
    # Frost rays
    for rx, ry in ((16, 3), (9, 8), (23, 8)):
        put_px(glow, rx, ry, white)
    return finalize(body, "#040D18", glow=glow)


def draw_corrosive_flask() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    gl1, gl2 = hex_rgba("#334155"), hex_rgba("#94A3B8")
    a1, a2, a3, a4, cork = (
        hex_rgba("#14532D"), hex_rgba("#84CC16"), hex_rgba("#BEF264"), hex_rgba("#ECFCCB"), hex_rgba("#78350F")
    )

    dg.ellipse([(7, 10), (25, 28)], fill=hex_rgba("#84CC16", 60))
    # Cork & bottle neck
    d.rectangle([(14, 4), (18, 7)], fill=cork)
    d.rectangle([(13, 7), (19, 12)], fill=gl1, outline=gl2)
    # Round glass flask d=16
    d.ellipse([(8, 11), (24, 27)], fill=gl1, outline=gl2)
    # Boiling green acid inside lower 2/3
    d.pieslice([(9, 12), (23, 26)], start=0, end=180, fill=a2)
    d.rectangle([(10, 17), (22, 21)], fill=a2)
    d.line([(10, 17), (22, 17)], fill=a3)
    d.ellipse([(11, 21), (21, 25)], fill=a1)
    # Bubbles & glass specular
    for bx, by in ((13, 19), (18, 18), (16, 22)):
        put_px(body, bx, by, a4)
    d.line([(10, 15), (11, 13)], fill=a4)
    return finalize(body, "#071205", glow=glow)


def draw_blood_stone() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    c1, c2, c3, c4 = hex_rgba("#450A0A"), hex_rgba("#991B1B"), hex_rgba("#EF4444"), hex_rgba("#FCA5A5")

    dg.ellipse([(6, 5), (26, 27)], fill=hex_rgba("#EF4444", 65))
    # Spiky raw ruby geode 14x18
    d.polygon([(16, 4), (23, 11), (21, 23), (16, 28), (11, 23), (9, 11)], fill=c1)
    d.polygon([(16, 6), (21, 12), (16, 26), (11, 12)], fill=c2)
    d.polygon([(16, 6), (11, 12), (16, 16)], fill=c3)
    d.line([(16, 7), (13, 12)], fill=c4)
    # Surrounding sharp spikes
    d.polygon([(9, 14), (5, 16), (10, 18)], fill=c2)
    d.polygon([(23, 14), (27, 16), (22, 18)], fill=c2)
    d.line([(14, 15), (18, 19), (16, 24)], fill=c3)
    return finalize(body, "#140407", glow=glow)


def draw_eagle_scope() -> Image.Image:
    body, top = new_canvas(), new_canvas()
    d = ImageDraw.Draw(body)
    g1, g2, g3 = hex_rgba("#92400E"), hex_rgba("#D97706"), hex_rgba("#FDE68A")
    l1, l2, white = hex_rgba("#06B6D4"), hex_rgba("#67E8F9"), hex_rgba("#FFFFFF")

    # Diagonal brass spyglass (7,24)->(24,7)
    d.line([(6, 25), (22, 9)], fill=g1, width=6)
    d.line([(7, 24), (21, 10)], fill=g2, width=4)
    d.line([(8, 22), (20, 10)], fill=g3, width=1)
    # Wide front optical lens
    d.ellipse([(17, 5), (27, 15)], fill=g2, outline=g1)
    d.ellipse([(19, 7), (25, 13)], fill=l1)
    d.ellipse([(20, 8), (24, 12)], fill=l2)
    # Crosshair reticle inside lens
    put_px(top, 22, 10, white)
    put_px(top, 22, 8, white)
    put_px(top, 20, 10, white)
    return finalize(body, "#0E0C06", top=top)


def draw_chrono_hourglass() -> Image.Image:
    body = new_canvas()
    d = ImageDraw.Draw(body)
    c1, c2 = hex_rgba("#78350F"), hex_rgba("#B45309")
    s1, s2, glass = hex_rgba("#F59E0B"), hex_rgba("#FEF08A"), hex_rgba("#64748B")

    # Gothic copper frame & columns
    d.rectangle([(8, 4), (24, 7)], fill=c2)
    d.rectangle([(8, 25), (24, 28)], fill=c2)
    d.line([(9, 7), (9, 25)], fill=c1, width=2)
    d.line([(23, 7), (23, 25)], fill=c1, width=2)
    # Glass bulbs
    d.polygon([(11, 7), (21, 7), (17, 16), (21, 25), (11, 25), (15, 16)], fill=glass)
    # Golden sand top, falling stream, and bottom mound
    d.polygon([(13, 10), (19, 10), (16, 15)], fill=s1)
    d.line([(16, 15), (16, 23)], fill=s2)
    d.polygon([(12, 24), (20, 24), (16, 19)], fill=s1)
    d.line([(14, 22), (18, 22)], fill=s2)
    return finalize(body, "#0C0804")


def draw_magnet_sphere() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    st = hex_rgba("#1E293B")
    b1, b2, b3, b4 = hex_rgba("#1D4ED8"), hex_rgba("#3B82F6"), hex_rgba("#60A5FA"), hex_rgba("#DBEAFE")

    dg.ellipse([(4, 4), (28, 28)], fill=hex_rgba("#3B82F6", 65))
    # Two perpendicular gimbal rings
    d.ellipse([(5, 10), (27, 22)], outline=b2, width=2)
    d.ellipse([(10, 5), (22, 27)], outline=b3, width=2)
    # Central iron-blue core d=10
    d.ellipse([(11, 11), (21, 21)], fill=st, outline=b1)
    d.ellipse([(13, 13), (19, 19)], fill=b2)
    d.rectangle([(14, 14), (16, 16)], fill=b4)
    return finalize(body, "#060A14", glow=glow)


def draw_tesla_coil() -> Image.Image:
    body, glow, top = new_canvas(), new_canvas(), new_canvas()
    d, dg, dt = ImageDraw.Draw(body), ImageDraw.Draw(glow), ImageDraw.Draw(top)
    c1, c2 = hex_rgba("#B45309"), hex_rgba("#D97706")
    p1, p2, cy, white = hex_rgba("#7C3AED"), hex_rgba("#A855F7"), hex_rgba("#67E8F9"), hex_rgba("#FFFFFF")

    dg.ellipse([(6, 3), (26, 19)], fill=hex_rgba("#7C3AED", 75))
    # Base & 5-turn conical copper coil
    d.rectangle([(10, 25), (22, 28)], fill=c1)
    for i, y in enumerate((23, 20, 17, 14, 11)):
        hw = 5 - i // 2
        d.rectangle([(16 - hw, y), (16 + hw, y + 2)], fill=c2 if i % 2 == 0 else c1)
    # Top toroid discharge ring
    d.ellipse([(10, 6), (22, 11)], fill=c2, outline=c1)
    d.line([(12, 8), (20, 8)], fill=white)
    # 3 zigzag sparks
    for bolt in ([(10, 8), (5, 6), (3, 9)], [(22, 8), (27, 6), (29, 9)], [(16, 6), (14, 3), (18, 2)]):
        dt.line(bolt, fill=p2, width=2)
        dt.line(bolt, fill=cy, width=1)
        put_px(top, bolt[-1][0], bolt[-1][1], white)
    put_px(glow, 16, 8, p1)
    return finalize(body, "#080712", glow=glow, top=top)


def draw_clover() -> Image.Image:
    body = new_canvas()
    d = ImageDraw.Draw(body)
    e1, e2, e3, e4 = hex_rgba("#064E3B"), hex_rgba("#059669"), hex_rgba("#10B981"), hex_rgba("#6EE7B7")
    g1, g2 = hex_rgba("#F59E0B"), hex_rgba("#FEF08A")

    # Curved stem
    d.line([(16, 17), (15, 24), (12, 28)], fill=e1, width=2)
    # 4 heart-shaped jade leaves with golden rims
    for lx, ly in ((16, 9), (16, 21), (10, 15), (22, 15)):
        d.ellipse([(lx - 4, ly - 4), (lx + 4, ly + 4)], fill=e2, outline=g1)
        d.ellipse([(lx - 2, ly - 2), (lx + 2, ly + 2)], fill=e3)
        put_px(body, lx - 1, ly - 1, e4)
    # Center golden rivet
    d.rectangle([(15, 14), (17, 16)], fill=g1)
    put_px(body, 16, 15, g2)
    return finalize(body, "#041008")


def draw_prismatic() -> Image.Image:
    body, top = new_canvas(), new_canvas()
    d, dt = ImageDraw.Draw(body), ImageDraw.Draw(top)
    cr1, cr2 = hex_rgba("#E2E8F0"), hex_rgba("#F8FAFC")
    spec = [hex_rgba("#EF4444"), hex_rgba("#F59E0B"), hex_rgba("#10B981"), hex_rgba("#06B6D4"), hex_rgba("#8B5CF6")]

    # Incoming white ray from left
    dt.line([(2, 18), (12, 15)], fill=cr2, width=1)
    # Spectral rainbow fan exiting right
    for i, col in enumerate(spec):
        dt.line([(19, 14 + i // 2), (29, 9 + i * 3)], fill=col, width=2)
    # Central crystal prism triangle
    d.polygon([(16, 6), (24, 24), (8, 24)], fill=cr1)
    d.polygon([(16, 8), (16, 23), (10, 23)], fill=cr2)
    return finalize(body, "#080514", top=top)


def draw_aegis() -> Image.Image:
    body = new_canvas()
    d = ImageDraw.Draw(body)
    o1, o2, ti = hex_rgba("#0F172A"), hex_rgba("#1E293B"), hex_rgba("#475569")
    r1, r2 = hex_rgba("#DC2626"), hex_rgba("#F87171")

    # Heavy teardrop obsidian shield 16x22
    shield = [(8, 5), (24, 5), (24, 19), (16, 27), (8, 19)]
    d.polygon(shield, fill=o1, outline=ti)
    d.polygon([(10, 7), (16, 7), (16, 25), (10, 18)], fill=o2)
    d.line([(16, 6), (16, 26)], fill=ti)
    # 4 corner ruby rivets & center rune
    for rx, ry in ((10, 7), (22, 7), (10, 18), (22, 18)):
        d.rectangle([(rx, ry), (rx + 1, ry + 1)], fill=r1)
        put_px(body, rx, ry, r2)
    d.polygon([(16, 11), (19, 15), (16, 19), (13, 15)], outline=r2)
    return finalize(body, "#08060E")


# ===========================================================================
# PART 2: 14 Legendary Relics
# ===========================================================================

def draw_immortal_aegis() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    g1, g2, blue, fire, white = (
        hex_rgba("#F59E0B"), hex_rgba("#FDE047"), hex_rgba("#3B82F6"), hex_rgba("#EF4444"), hex_rgba("#FFFFFF")
    )
    dg.ellipse([(4, 4), (28, 28)], fill=hex_rgba("#F59E0B", 70))
    # Phoenix wings flanking shield
    d.polygon([(10, 14), (2, 7), (3, 17), (10, 20)], fill=g1)
    d.polygon([(22, 14), (30, 7), (29, 17), (22, 20)], fill=g1)
    d.line([(3, 10), (9, 15)], fill=g2)
    d.line([(29, 10), (23, 15)], fill=g2)
    # Golden shield + azure resurrection gem & rebirth flame
    d.polygon([(10, 7), (22, 7), (22, 19), (16, 26), (10, 19)], fill=g2, outline=g1)
    d.ellipse([(13, 12), (19, 18)], fill=blue)
    d.polygon([(14, 14), (16, 9), (18, 14)], fill=fire)
    put_px(body, 16, 15, white)
    return finalize(body, "#0A0812", glow=glow)


def draw_archmage_shard() -> Image.Image:
    body, glow, top = new_canvas(), new_canvas(), new_canvas()
    d, dg, dt = ImageDraw.Draw(body), ImageDraw.Draw(glow), ImageDraw.Draw(top)
    a1, a2, a3, gold, white = (
        hex_rgba("#7C3AED"), hex_rgba("#C084FC"), hex_rgba("#E879F9"), hex_rgba("#F59E0B"), hex_rgba("#FFFFFF")
    )
    dg.ellipse([(5, 4), (27, 28)], fill=hex_rgba("#7C3AED", 75))
    # Orbital golden arcane ring
    dt.ellipse([(4, 11), (28, 21)], outline=gold, width=1)
    # Floating crystal shard
    d.polygon([(16, 4), (22, 15), (16, 28), (10, 15)], fill=a1)
    d.polygon([(16, 5), (11, 15), (16, 26)], fill=a2)
    d.line([(16, 6), (16, 25)], fill=a3)
    put_px(body, 15, 11, white)
    put_px(top, 25, 8, white)
    return finalize(body, "#0A0812", glow=glow, top=top)


def draw_divine_rapier() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    g1, g2, l1, white, ruby = (
        hex_rgba("#B45309"), hex_rgba("#FDE68A"), hex_rgba("#FEF08A"), hex_rgba("#FFFFFF"), hex_rgba("#EF4444")
    )
    dg.line([(12, 20), (27, 5)], fill=hex_rgba("#FEF08A", 110), width=6)
    # Radiant light blade
    d.line([(11, 21), (27, 5)], fill=l1, width=3)
    d.line([(12, 20), (27, 5)], fill=white, width=1)
    # Filigree cup hilt & ruby pommel
    d.ellipse([(7, 17), (15, 25)], fill=g1, outline=g2)
    d.line([(5, 27), (10, 22)], fill=g1, width=2)
    d.rectangle([(4, 26), (6, 28)], fill=ruby)
    return finalize(body, "#0A0812", glow=glow)


def draw_octarine_core() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    o1, o2, lime, deep, white = (
        hex_rgba("#9333EA"), hex_rgba("#C084FC"), hex_rgba("#A3E635"), hex_rgba("#2E1065"), hex_rgba("#FFFFFF")
    )
    dg.ellipse([(3, 3), (29, 29)], fill=hex_rgba("#9333EA", 80))
    # Icosahedral octarine core
    d.polygon([(16, 4), (26, 10), (26, 22), (16, 28), (6, 22), (6, 10)], fill=deep, outline=o2)
    d.polygon([(16, 6), (24, 16), (16, 26), (8, 16)], fill=o1)
    d.polygon([(16, 9), (21, 16), (16, 23), (11, 16)], fill=lime)
    d.ellipse([(14, 14), (18, 18)], fill=white)
    return finalize(body, "#0A0812", glow=glow)


def draw_solar_relic() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    g1, g2, o1, y1 = hex_rgba("#D97706"), hex_rgba("#F59E0B"), hex_rgba("#F97316"), hex_rgba("#FEF9C3")

    dg.ellipse([(3, 3), (29, 29)], fill=hex_rgba("#F59E0B", 70))
    # 12 sharp sun rays
    for i in range(12):
        ang = i * (math.pi / 6.0)
        r = 13 if i % 2 == 0 else 10
        d.line([(16, 16), (int(16 + math.cos(ang) * r), int(16 + math.sin(ang) * r))], fill=g2 if i % 2 == 0 else g1, width=2)
    # Golden solar diadem ring & amber center
    d.ellipse([(9, 9), (23, 23)], fill=g1, outline=y1)
    d.ellipse([(12, 12), (20, 20)], fill=o1)
    d.ellipse([(14, 14), (18, 18)], fill=y1)
    return finalize(body, "#0A0812", glow=glow)


def draw_stormhammer_core() -> Image.Image:
    body, glow, top = new_canvas(), new_canvas(), new_canvas()
    d, dg, dt = ImageDraw.Draw(body), ImageDraw.Draw(glow), ImageDraw.Draw(top)
    s1, s2, c1, c2, white = (
        hex_rgba("#334155"), hex_rgba("#94A3B8"), hex_rgba("#06B6D4"), hex_rgba("#67E8F9"), hex_rgba("#FFFFFF")
    )
    dg.ellipse([(4, 4), (28, 28)], fill=hex_rgba("#06B6D4", 75))
    d.ellipse([(5, 5), (27, 27)], outline=c1, width=2)
    # Miniature hammer suspended inside storm sphere
    d.line([(16, 16), (11, 24)], fill=s1, width=3)
    d.rectangle([(12, 10), (22, 17)], fill=s2, outline=s1)
    # Electromagnetic storm arcs
    dt.line([(6, 12), (13, 13), (20, 10), (26, 14)], fill=c2, width=1)
    dt.line([(8, 22), (16, 18), (24, 21)], fill=white, width=1)
    return finalize(body, "#0A0812", glow=glow, top=top)


def draw_colossal_eye() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    st, gold, black, r1, r2 = (
        hex_rgba("#292524"), hex_rgba("#B45309"), hex_rgba("#000000"), hex_rgba("#DC2626"), hex_rgba("#F87171")
    )
    dg.ellipse([(4, 7), (28, 25)], fill=hex_rgba("#DC2626", 70))
    # Stone & gold almond eye bezel
    d.polygon([(3, 16), (10, 8), (22, 8), (29, 16), (22, 24), (10, 24)], fill=st, outline=gold)
    d.ellipse([(10, 10), (22, 22)], fill=r1)
    d.ellipse([(12, 12), (20, 20)], fill=r2)
    d.rectangle([(15, 11), (17, 21)], fill=black)
    put_px(body, 14, 13, hex_rgba("#FFFFFF"))
    return finalize(body, "#0A0812", glow=glow)


def draw_blessed_core() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    c1, c2, c3, gold, white = (
        hex_rgba("#0284C7"), hex_rgba("#38BDF8"), hex_rgba("#E0F2FE"), hex_rgba("#F59E0B"), hex_rgba("#FFFFFF")
    )
    dg.ellipse([(4, 4), (28, 28)], fill=hex_rgba("#38BDF8", 70))
    # Golden feather halo
    d.polygon([(10, 13), (3, 9), (5, 18), (11, 20)], fill=gold)
    d.polygon([(22, 13), (29, 9), (27, 18), (21, 20)], fill=gold)
    # Sapphire crystal heart
    d.ellipse([(10, 9), (17, 17)], fill=c2)
    d.ellipse([(15, 9), (22, 17)], fill=c1)
    d.polygon([(10, 14), (22, 14), (16, 24)], fill=c2)
    d.polygon([(12, 12), (16, 12), (16, 20)], fill=c3)
    put_px(body, 14, 13, white)
    return finalize(body, "#0A0812", glow=glow)


def draw_cinder_heart() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    coal, lava1, lava2, hi, smoke = (
        hex_rgba("#1C1917"), hex_rgba("#991B1B"), hex_rgba("#EA580C"), hex_rgba("#FEF08A"), hex_rgba("#44403C")
    )
    dg.ellipse([(5, 5), (27, 27)], fill=hex_rgba("#EA580C", 65))
    d.ellipse([(9, 9), (17, 18)], fill=coal)
    d.ellipse([(15, 9), (23, 18)], fill=coal)
    d.polygon([(9, 15), (23, 15), (16, 26)], fill=coal)
    # Molten regeneration cracks
    d.line([(12, 12), (16, 17), (20, 12)], fill=lava1, width=3)
    d.line([(12, 12), (16, 17), (16, 24)], fill=lava2, width=2)
    d.rectangle([(15, 16), (17, 18)], fill=hi)
    put_px(glow, 13, 6, smoke)
    put_px(glow, 19, 5, smoke)
    return finalize(body, "#0A0812", glow=glow)


def draw_spectral_jar() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    onyx, chain = hex_rgba("#0F172A"), hex_rgba("#334155")
    e1, e2, e3, e4 = hex_rgba("#059669"), hex_rgba("#10B981"), hex_rgba("#6EE7B7"), hex_rgba("#ECFDF5")

    dg.ellipse([(6, 3), (26, 23)], fill=hex_rgba("#10B981", 65))
    # Onyx jar with chains
    d.rectangle([(12, 12), (20, 15)], fill=chain)
    d.ellipse([(9, 14), (23, 28)], fill=onyx, outline=chain)
    d.line([(10, 20), (22, 20)], fill=chain, width=2)
    # Escaping emerald souls rising from neck
    d.polygon([(16, 14), (11, 7), (16, 3), (21, 7)], fill=e2)
    d.ellipse([(13, 6), (19, 12)], fill=e3)
    put_px(body, 15, 8, e4)
    put_px(body, 14, 18, e1)
    put_px(body, 18, 22, e2)
    return finalize(body, "#0A0812", glow=glow)


def draw_halo_radiance() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    g1, g2, g3, white = hex_rgba("#D97706"), hex_rgba("#F59E0B"), hex_rgba("#FDE047"), hex_rgba("#FFFBEB")

    dg.ellipse([(2, 2), (30, 30)], fill=hex_rgba("#FDE047", 65))
    # Cross blessing rays
    d.line([(16, 2), (16, 30)], fill=g3, width=2)
    d.line([(2, 16), (30, 16)], fill=g3, width=2)
    # Triple concentric golden halo rings
    d.ellipse([(4, 4), (28, 28)], outline=g1, width=2)
    d.ellipse([(8, 8), (24, 24)], outline=g2, width=2)
    d.ellipse([(12, 12), (20, 20)], outline=g3, width=2)
    d.rectangle([(15, 15), (17, 17)], fill=white)
    return finalize(body, "#0A0812", glow=glow)


def draw_resonant_strings() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    wood, gold = hex_rgba("#451A03"), hex_rgba("#D97706")
    c1, c2, white = hex_rgba("#06B6D4"), hex_rgba("#67E8F9"), hex_rgba("#FFFFFF")

    dg.ellipse([(7, 6), (25, 24)], fill=hex_rgba("#06B6D4", 65))
    # Rosewood & gold lyre frame
    d.rectangle([(10, 22), (22, 27)], fill=wood, outline=gold)
    d.line([(10, 23), (7, 14), (10, 7)], fill=wood, width=3)
    d.line([(22, 23), (25, 14), (22, 7)], fill=wood, width=3)
    d.line([(9, 7), (23, 7)], fill=gold, width=2)
    # 4 vibrating lightning strings
    for i, sx in enumerate((12, 15, 17, 20)):
        d.line([(sx, 8), (sx + (1 if i % 2 == 0 else -1), 15), (sx, 22)], fill=c1 if i % 2 == 0 else c2, width=1)
    put_px(body, 16, 15, white)
    return finalize(body, "#0A0812", glow=glow)


def draw_fate_weaver() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    e1, e2, e3, gold, star = (
        hex_rgba("#064E3B"), hex_rgba("#059669"), hex_rgba("#34D399"), hex_rgba("#FBBF24"), hex_rgba("#ECFDF5")
    )
    dg.ellipse([(3, 3), (29, 29)], fill=hex_rgba("#10B981", 65))
    # Outer fractal leaf ring (8 sub-leaves)
    for i in range(8):
        ang = i * (math.pi / 4.0)
        lx, ly = int(16 + math.cos(ang) * 9), int(16 + math.sin(ang) * 9)
        d.ellipse([(lx - 3, ly - 3), (lx + 3, ly + 3)], fill=e1 if i % 2 else e2)
    # Inner 4 bright emerald leaves with gold veins
    for lx, ly in ((16, 11), (16, 21), (11, 16), (21, 16)):
        d.ellipse([(lx - 3, ly - 3), (lx + 3, ly + 3)], fill=e3)
        d.line([(16, 16), (lx, ly)], fill=gold)
    put_px(body, 16, 16, star)
    return finalize(body, "#0A0812", glow=glow)


def draw_spore_mushroom() -> Image.Image:
    body, glow, top = new_canvas(), new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    stem, cap, spot, spore, hi = (
        hex_rgba("#1E293B"), hex_rgba("#0D9488"), hex_rgba("#2DD4BF"), hex_rgba("#99F6E4"), hex_rgba("#F0FDFA")
    )
    dg.ellipse([(5, 4), (27, 24)], fill=hex_rgba("#2DD4BF", 70))
    # Stem & glowing turquoise mushroom cap
    d.rectangle([(13, 18), (19, 27)], fill=stem)
    d.pieslice([(6, 8), (26, 24)], start=180, end=360, fill=cap)
    d.rectangle([(7, 16), (25, 19)], fill=cap)
    # Bioluminescent spots & floating healing spores
    for sx, sy in ((11, 13), (16, 10), (21, 13), (15, 15)):
        d.rectangle([(sx, sy), (sx + 1, sy + 1)], fill=spot)
    for px0, py0 in ((6, 7), (12, 4), (22, 5), (27, 9), (8, 22), (24, 21)):
        put_px(top, px0, py0, spore if (px0 + py0) % 2 else hi)
    return finalize(body, "#0A0812", glow=glow, top=top)


# ===========================================================================
# PART 3: 6 Limit Break Perks
# ===========================================================================

def draw_lb_damage() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    white, r1, r2, gold = hex_rgba("#FFFFFF"), hex_rgba("#EF4444"), hex_rgba("#DC2626"), hex_rgba("#F59E0B")

    dg.ellipse([(4, 4), (28, 28)], fill=hex_rgba("#EF4444", 85))
    add_lb_laurel(d, body)
    # Cracked anvil & reality-splitting fissures at bottom
    d.polygon([(8, 24), (24, 24), (21, 28), (11, 28)], fill=hex_rgba("#334155"))
    d.line([(16, 24), (8, 19)], fill=r1, width=2)
    d.line([(16, 24), (24, 19)], fill=r1, width=2)
    # White-hot vertical greatsword plunging into anvil
    d.polygon([(14, 10), (18, 10), (17, 25), (15, 25)], fill=r2)
    d.line([(16, 10), (16, 25)], fill=white, width=2)
    d.rectangle([(10, 9), (22, 11)], fill=gold)
    d.rectangle([(15, 4), (17, 9)], fill=gold)
    return finalize(body, "#140204", glow=glow)


def draw_lb_cooldown() -> Image.Image:
    body, glow, top = new_canvas(), new_canvas(), new_canvas()
    d, dg, dt = ImageDraw.Draw(body), ImageDraw.Draw(glow), ImageDraw.Draw(top)
    face, num, hand, white = hex_rgba("#1E1B4B"), hex_rgba("#7C3AED"), hex_rgba("#22D3EE"), hex_rgba("#FFFFFF")

    dg.ellipse([(4, 4), (28, 28)], fill=hex_rgba("#22D3EE", 75))
    add_lb_laurel(d, body)
    # Shattering 22x22 chronometer dial
    d.ellipse([(6, 6), (26, 26)], fill=face, outline=num, width=2)
    for ang_deg in range(0, 360, 45):
        ang = math.radians(ang_deg)
        put_px(body, int(16 + math.cos(ang) * 8), int(16 + math.sin(ang) * 8), num)
    # High-speed swirling light hands
    dt.arc([(9, 9), (23, 23)], start=20, end=290, fill=hand, width=2)
    dt.line([(16, 16), (23, 11)], fill=white, width=2)
    dt.line([(16, 16), (11, 10)], fill=hand, width=1)
    put_px(top, 16, 16, white)
    return finalize(body, "#070614", glow=glow, top=top)


def draw_lb_projectiles() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    c1, c2, c3, white = hex_rgba("#0284C7"), hex_rgba("#38BDF8"), hex_rgba("#E0F2FE"), hex_rgba("#FFFFFF")

    dg.ellipse([(5, 5), (27, 27)], fill=hex_rgba("#38BDF8", 75))
    add_lb_laurel(d, body)
    # 5 comet arrows fanning from (6,26) into upper-right quadrant
    targets = [(11, 5), (18, 4), (26, 6), (27, 14), (26, 21)]
    for tx, ty in targets:
        d.line([(6, 26), (tx, ty)], fill=c1, width=2)
        d.line([(8, 24), (tx, ty)], fill=c2, width=1)
        d.rectangle([(tx - 1, ty), (tx, ty + 1)], fill=white)
    d.ellipse([(4, 24), (8, 28)], fill=c3)
    put_px(body, 6, 26, white)
    return finalize(body, "#060A14", glow=glow)


def draw_lb_titan_vigor() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    ad, g1, g2, e1, e2 = (
        hex_rgba("#1E293B"), hex_rgba("#F59E0B"), hex_rgba("#FDE047"), hex_rgba("#10B981"), hex_rgba("#6EE7B7")
    )
    dg.ellipse([(4, 4), (28, 28)], fill=hex_rgba("#10B981", 80))
    add_lb_laurel(d, body)
    # Armored golden titan heart
    d.ellipse([(8, 8), (17, 18)], fill=g1, outline=ad)
    d.ellipse([(15, 8), (24, 18)], fill=g1, outline=ad)
    d.polygon([(8, 14), (24, 14), (16, 26)], fill=ad)
    d.polygon([(10, 14), (22, 14), (16, 23)], fill=g2)
    # Emerald regeneration cross bursting from center
    d.rectangle([(15, 10), (17, 22)], fill=e1)
    d.rectangle([(10, 15), (22, 17)], fill=e1)
    d.line([(16, 11), (16, 21)], fill=e2)
    d.line([(11, 16), (21, 16)], fill=e2)
    return finalize(body, "#0F0C04", glow=glow)


def draw_lb_adaptive_armor() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)
    s1, s2, b1, b2, white = (
        hex_rgba("#475569"), hex_rgba("#94A3B8"), hex_rgba("#3B82F6"), hex_rgba("#60A5FA"), hex_rgba("#FFFFFF")
    )
    dg.ellipse([(4, 4), (28, 28)], fill=hex_rgba("#3B82F6", 75))
    add_lb_laurel(d, body)
    # 3D hexagonal diamond bastion shield
    hex_pts = [(16, 5), (25, 10), (25, 22), (16, 27), (7, 22), (7, 10)]
    d.polygon(hex_pts, fill=s1, outline=b2)
    d.polygon([(16, 7), (16, 25), (9, 21), (9, 11)], fill=s2)
    d.polygon([(16, 10), (21, 16), (16, 22), (11, 16)], fill=b1)
    d.line([(16, 10), (11, 16)], fill=white)
    # 6 surrounding micro barrier prisms
    for x, y in ((16, 2), (28, 10), (28, 22), (16, 29), (4, 22), (4, 10)):
        put_px(glow, x, y, b2)
    return finalize(body, "#040914", glow=glow)


def draw_lb_predator_speed() -> Image.Image:
    body, glow, top = new_canvas(), new_canvas(), new_canvas()
    d, dg, dt = ImageDraw.Draw(body), ImageDraw.Draw(glow), ImageDraw.Draw(top)
    b1, b2, gold, l1, l2 = (
        hex_rgba("#3F6212"), hex_rgba("#65A30D"), hex_rgba("#F59E0B"), hex_rgba("#BEF264"), hex_rgba("#F0FDF4")
    )
    dg.ellipse([(4, 6), (28, 26)], fill=hex_rgba("#65A30D", 70))
    add_lb_laurel(d, body)
    # Winged clawed boot surging right (6,20)->(26,12)
    d.polygon([(10, 12), (17, 11), (18, 19), (26, 20), (25, 25), (10, 25)], fill=b1)
    d.polygon([(12, 13), (16, 13), (17, 21), (24, 21), (23, 23), (12, 23)], fill=b2)
    # Golden Hermes wing on heel
    d.polygon([(11, 15), (4, 9), (6, 17), (11, 19)], fill=gold)
    d.line([(5, 11), (10, 16)], fill=l2)
    # Hyper-speed horizontal motion streaks
    dt.line([(3, 21), (10, 21)], fill=l1, width=1)
    dt.line([(5, 24), (12, 24)], fill=l2, width=1)
    dt.line([(20, 13), (28, 13)], fill=l1, width=1)
    return finalize(body, "#0A0E04", glow=glow, top=top)


# ---------------------------------------------------------------------------
# Batch Generation Pipeline
# ---------------------------------------------------------------------------

BASIC_PASSIVES: list[tuple[str, Callable[[], Image.Image]]] = [
    ("icon_speed_gloves.png", draw_speed_gloves),
    ("icon_giant_heart.png", draw_giant_heart),
    ("icon_cooldown_tome.png", draw_cooldown_tome),
    ("icon_light_armor.png", draw_light_armor),
    ("icon_frost_ring.png", draw_frost_ring),
    ("icon_corrosive_flask.png", draw_corrosive_flask),
    ("icon_blood_stone.png", draw_blood_stone),
    ("icon_eagle_scope.png", draw_eagle_scope),
    ("icon_chrono_hourglass.png", draw_chrono_hourglass),
    ("icon_magnet_sphere.png", draw_magnet_sphere),
    ("icon_tesla_coil.png", draw_tesla_coil),
    ("icon_clover.png", draw_clover),
    ("icon_prismatic.png", draw_prismatic),
    ("icon_aegis.png", draw_aegis),
]

LEGENDARY_RELICS: list[tuple[str, Callable[[], Image.Image]]] = [
    ("icon_immortal_aegis.png", draw_immortal_aegis),
    ("icon_archmage_shard.png", draw_archmage_shard),
    ("icon_divine_rapier.png", draw_divine_rapier),
    ("icon_octarine_core.png", draw_octarine_core),
    ("icon_solar_relic.png", draw_solar_relic),
    ("icon_stormhammer_core.png", draw_stormhammer_core),
    ("icon_colossal_eye.png", draw_colossal_eye),
    ("icon_blessed_core.png", draw_blessed_core),
    ("icon_cinder_heart.png", draw_cinder_heart),
    ("icon_spectral_jar.png", draw_spectral_jar),
    ("icon_halo_radiance.png", draw_halo_radiance),
    ("icon_resonant_strings.png", draw_resonant_strings),
    ("icon_fate_weaver.png", draw_fate_weaver),
    ("icon_spore_mushroom.png", draw_spore_mushroom),
]

LIMIT_BREAK_PERKS: list[tuple[str, Callable[[], Image.Image]]] = [
    ("icon_lb_damage.png", draw_lb_damage),
    ("icon_lb_cooldown.png", draw_lb_cooldown),
    ("icon_lb_projectiles.png", draw_lb_projectiles),
    ("icon_lb_titan_vigor.png", draw_lb_titan_vigor),
    ("icon_lb_adaptive_armor.png", draw_lb_adaptive_armor),
    ("icon_lb_predator_speed.png", draw_lb_predator_speed),
]


def generate_all(out_root: Path) -> list[Path]:
    icons_dir = out_root / "assets" / "icons"
    groups = [
        (BASIC_PASSIVES, icons_dir / "passives"),
        (LEGENDARY_RELICS, icons_dir / "legendary"),
        (LIMIT_BREAK_PERKS, icons_dir / "limit_break"),
    ]

    icons_dir.mkdir(parents=True, exist_ok=True)
    saved: list[Path] = []

    for items, subdir in groups:
        subdir.mkdir(parents=True, exist_ok=True)
        for filename, fn in items:
            img = fn()
            for target_dir in (icons_dir, subdir):
                path = target_dir / filename
                img.save(path, format="PNG")
                saved.append(path)
    return saved


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Generate 28 Passive Relics + 6 Limit Break 32x32 RGBA icons (Group 9)."
    )
    parser.add_argument(
        "--out-root",
        type=Path,
        default=Path("."),
        help="Project root directory where assets/icons/ and subfolders will be written.",
    )
    args = parser.parse_args()

    files = generate_all(args.out_root)
    print(f"Generated {len(files)} PNG files under {(args.out_root / 'assets' / 'icons').resolve()}")


if __name__ == "__main__":
    main()
