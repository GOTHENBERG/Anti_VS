#!/usr/bin/env python3
"""Procedural pixel-art generator for 20 Weapon Evolutions Icons (Group 8).

Generates 20 monumental 32x32 RGBA pixel-art evolution icons in Dark Fantasy
Gothic style with prestige filigree, plasma cores, 1px dark outlines, and soft
translucent magical halos.

Outputs are saved to `assets/icons/` and `assets/icons/evolutions/`, including
all canonical engine aliases (e.g. `icon_calamity_mortar.png`,
`icon_stormhammer_core.png`, `icon_halo_radiance.png`, `icon_spectral_jar.png`,
`icon_immortal_aegis.png`, `icon_cinder_heart.png`, etc.).

Usage:
    python scripts/generate_evolutions.py
    python scripts/generate_evolutions.py --out-root .
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
# 1. icon_evo_blade_storm.png — Blade Storm
# ---------------------------------------------------------------------------

def draw_evo_blade_storm() -> Image.Image:
    body, glow, top = new_canvas(), new_canvas(), new_canvas()
    d, dg, dt = ImageDraw.Draw(body), ImageDraw.Draw(glow), ImageDraw.Draw(top)

    ti, st, pl = hex_rgba("#1E293B"), hex_rgba("#94A3B8"), hex_rgba("#F8FAFC")
    c1, c2, c3, c4 = hex_rgba("#0D9488"), hex_rgba("#14B8A6"), hex_rgba("#5EEAD4"), hex_rgba("#F0FDFA")
    g1, g2 = hex_rgba("#D97706"), hex_rgba("#FDE68A")

    dg.ellipse([(2, 2), (30, 30)], fill=hex_rgba("#14B8A6", 65))

    # Closed hurricane wind ring r=12 around (16,16)
    d.ellipse([(4, 4), (28, 28)], outline=c2, width=2)
    d.ellipse([(5, 5), (27, 27)], outline=c3, width=1)
    # 4 orbiting micro wind-blades
    for mx, my in ((16, 2), (30, 16), (16, 30), (2, 16)):
        d.rectangle([(mx - 1, my - 1), (mx + 1, my + 1)], fill=c4)

    # Crossed curved boomerang orichalcum blades
    b1 = [(6, 26), (9, 20), (16, 15), (23, 8), (26, 6), (22, 13), (16, 17), (10, 23)]
    b2 = [(26, 26), (23, 20), (16, 15), (9, 8), (6, 6), (10, 13), (16, 17), (22, 23)]
    d.polygon(b1, fill=st)
    d.polygon(b2, fill=ti)
    d.line([(6, 26), (16, 16), (26, 6)], fill=pl, width=1)
    d.line([(26, 26), (16, 16), (6, 6)], fill=st, width=1)
    # Golden hilts near bottom corners
    d.line([(5, 27), (9, 23)], fill=g1, width=2)
    d.line([(27, 27), (23, 23)], fill=g1, width=2)
    put_px(body, 6, 26, g2)
    put_px(body, 26, 26, g2)

    # Blinding white flash at center (16,16)
    dt.line([(13, 16), (19, 16)], fill=c4, width=1)
    dt.line([(16, 13), (16, 19)], fill=c4, width=1)
    dt.rectangle([(15, 15), (17, 17)], fill=pl)
    put_px(top, 16, 16, c4)
    put_px(glow, 16, 16, c1)

    return finalize(body, "#051410", glow=glow, top=top)


# ---------------------------------------------------------------------------
# 2. icon_evo_hellfire.png — Hellfire
# ---------------------------------------------------------------------------

def draw_evo_hellfire() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)

    h1, h2, h3 = hex_rgba("#450A0A"), hex_rgba("#991B1B"), hex_rgba("#EF4444")
    iron, hot_o, hot_y = hex_rgba("#18181B"), hex_rgba("#EA580C"), hex_rgba("#FDE047")
    white, plasma = hex_rgba("#FFFBEB"), hex_rgba("#F59E0B")

    dg.ellipse([(4, 4), (28, 29)], fill=hex_rgba("#EF4444", 70))

    # 3 infernal flame torches erupting from top aortas (y=3..5)
    for fx, fy in ((11, 4), (16, 2), (21, 5)):
        d.polygon([(fx - 2, 11), (fx, fy), (fx + 2, 11)], fill=h3)
        d.line([(fx, fy + 1), (fx, 10)], fill=plasma)
        put_px(body, fx, fy + 1, hot_y)

    # Anatomical demonic heart 14x16 centered at (16,16)
    d.ellipse([(9, 9), (17, 19)], fill=h2)
    d.ellipse([(15, 9), (23, 19)], fill=h2)
    d.polygon([(9, 15), (23, 15), (16, 25)], fill=h2)
    d.polygon([(11, 13), (21, 13), (16, 22)], fill=h3)
    d.ellipse([(13, 13), (19, 19)], fill=plasma)
    d.rectangle([(15, 15), (17, 17)], fill=white)
    d.line([(10, 17), (16, 25)], fill=h1, width=2)

    # Glowing iron chains crossed over heart
    d.line([(9, 11), (22, 22)], fill=iron, width=3)
    d.line([(23, 11), (10, 22)], fill=iron, width=3)
    for i in range(0, 12, 3):
        put_px(body, 10 + i, 12 + i, hot_o)
        put_px(body, 22 - i, 12 + i, hot_o)
    for rx, ry in ((12, 14), (16, 17), (20, 14)):
        put_px(body, rx, ry, hot_y)

    # 3 dripping lava globs below heart
    for lx, ly in ((12, 28), (16, 29), (20, 27)):
        d.line([(lx, 24), (lx, ly)], fill=hot_o)
        put_px(body, lx, ly, hot_y)

    return finalize(body, "#120305", glow=glow)


# ---------------------------------------------------------------------------
# 3. icon_evo_death_ray.png — Death Ray
# ---------------------------------------------------------------------------

def draw_evo_death_ray() -> Image.Image:
    body, glow, top = new_canvas(), new_canvas(), new_canvas()
    d, dg, dt = ImageDraw.Draw(body), ImageDraw.Draw(glow), ImageDraw.Draw(top)

    ebony, g1, g2 = hex_rgba("#1C1917"), hex_rgba("#B45309"), hex_rgba("#FBBF24")
    amethyst, sapphire = hex_rgba("#7E22CE"), hex_rgba("#2563EB")
    uv, mag, white = hex_rgba("#A855F7"), hex_rgba("#F43F5E"), hex_rgba("#FFFFFF")

    # Prismatic disintegration laser beam from fork to (28,4)
    dg.line([(16, 16), (28, 4)], fill=hex_rgba("#F43F5E", 100), width=7)
    d.line([(16, 16), (28, 4)], fill=uv, width=4)
    d.line([(16, 16), (28, 4)], fill=mag, width=3)
    d.line([(16, 16), (28, 4)], fill=white, width=1)

    # Concentric plasma focusing rings along beam
    for rx, ry in ((20, 12), (24, 8)):
        dt.line([(rx - 2, ry - 2), (rx + 2, ry + 2)], fill=mag, width=2)
        dt.line([(rx - 1, ry - 1), (rx + 1, ry + 1)], fill=white, width=1)

    # Ebony & gold archmage scepter shaft (4,28)->(16,16)
    d.line([(4, 28), (15, 17)], fill=ebony, width=3)
    d.line([(5, 27), (14, 18)], fill=g1, width=1)
    put_px(body, 4, 28, g2)

    # Forked golden tuning-fork head holding 2 lenses
    d.line([(13, 15), (16, 16), (17, 19)], fill=g2, width=2)
    d.line([(13, 15), (18, 10)], fill=g2, width=2)
    d.line([(17, 19), (22, 14)], fill=g1, width=2)
    d.rectangle([(15, 11), (16, 12)], fill=amethyst)
    d.rectangle([(19, 15), (20, 16)], fill=sapphire)

    return finalize(body, "#0A0414", glow=glow, top=top)


# ---------------------------------------------------------------------------
# 4. icon_evo_divine_shield.png — Divine Shield
# ---------------------------------------------------------------------------

def draw_evo_divine_shield() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)

    s1, s2, s3 = hex_rgba("#475569"), hex_rgba("#CBD5E1"), hex_rgba("#F8FAFC")
    g1, g2, g3, white = hex_rgba("#92400E"), hex_rgba("#F59E0B"), hex_rgba("#FEF08A"), hex_rgba("#FFFFFF")

    # 8 golden halo rays radiating behind shield
    dg.ellipse([(3, 3), (29, 29)], fill=hex_rgba("#FEF08A", 65))
    for i in range(8):
        ang = i * (math.pi / 4.0)
        x1, y1 = int(16 + math.cos(ang) * 9), int(16 + math.sin(ang) * 9)
        x2, y2 = int(16 + math.cos(ang) * 15), int(16 + math.sin(ang) * 15)
        dg.line([(x1, y1), (x2, y2)], fill=g3, width=2)

    # Crusader pavise tower shield 16x22: (8,5) to (24,27) with top lance notch
    shield = [(8, 6), (12, 5), (14, 7), (18, 7), (20, 5), (24, 6), (24, 20), (16, 27), (8, 20)]
    d.polygon(shield, fill=s2, outline=g2)
    d.line([(9, 7), (9, 19)], fill=s3, width=2)
    d.line([(23, 7), (23, 19)], fill=s1, width=2)

    # 3D raised golden cross (3px thick)
    d.rectangle([(15, 8), (17, 24)], fill=g2)
    d.rectangle([(10, 14), (22, 16)], fill=g2)
    d.line([(15, 8), (15, 24)], fill=g3)
    d.line([(10, 14), (22, 14)], fill=g3)
    d.line([(17, 9), (17, 24)], fill=g1)
    d.line([(11, 16), (22, 16)], fill=g1)

    # Central 3x3 faceted white diamond at (16,16)
    d.rectangle([(15, 14), (17, 16)], fill=white)
    put_px(body, 16, 15, g3)

    return finalize(body, "#0D0B05", glow=glow)


# ---------------------------------------------------------------------------
# 5. icon_evo_blizzard_storm.png — Blizzard Storm
# ---------------------------------------------------------------------------

def draw_evo_blizzard_storm() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)

    c1, c2, c3, c4, c5 = (
        hex_rgba("#0C4A6E"), hex_rgba("#0284C7"), hex_rgba("#38BDF8"), hex_rgba("#BAE6FD"), hex_rgba("#F0F9FF")
    )

    dg.ellipse([(3, 3), (29, 29)], fill=hex_rgba("#38BDF8", 65))

    # 6 symmetric barbed crystalline snowflake blades (diameter 24px)
    for i in range(6):
        ang = i * (math.pi / 3.0) - math.pi / 2.0
        tx, ty = int(round(16 + math.cos(ang) * 12)), int(round(16 + math.sin(ang) * 12))
        mx, my = int(round(16 + math.cos(ang) * 7)), int(round(16 + math.sin(ang) * 7))
        d.line([(16, 16), (tx, ty)], fill=c2, width=3)
        d.line([(16, 16), (tx, ty)], fill=c4, width=1)
        put_px(body, tx, ty, c5)
        # Side barbs
        for s in (-1, 1):
            bx = int(round(mx + math.cos(ang + s * 0.9) * 4))
            by = int(round(my + math.sin(ang + s * 0.9) * 4))
            d.line([(mx, my), (bx, by)], fill=c3, width=1)

    # Central "Eye of the Storm" d=6 (#0C4A6E) + absolute zero core (#F0F9FF)
    d.ellipse([(13, 13), (19, 19)], fill=c1, outline=c4)
    d.rectangle([(15, 15), (17, 17)], fill=c3)
    put_px(body, 16, 16, c5)

    # 8 orbiting frost micro-shards
    for i in range(8):
        ang = i * (math.pi / 4.0) + 0.2
        put_px(glow, int(round(16 + math.cos(ang) * 14)), int(round(16 + math.sin(ang) * 14)), c5)

    return finalize(body, "#030D18", glow=glow)


# ---------------------------------------------------------------------------
# 6. icon_evo_toxic_pestilence.png — Toxic Pestilence
# ---------------------------------------------------------------------------

def draw_evo_toxic_pestilence() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)

    b1, b2, b3 = hex_rgba("#14532D"), hex_rgba("#3F6212"), hex_rgba("#84CC16")
    a1, a2, a3, a4 = hex_rgba("#65A30D"), hex_rgba("#A3E635"), hex_rgba("#BEF264"), hex_rgba("#ECFCCB")

    dg.ellipse([(5, 6), (27, 29)], fill=hex_rgba("#84CC16", 65))

    # Boiling acid puddle at bottom
    d.ellipse([(6, 23), (26, 29)], fill=a1)
    d.ellipse([(9, 24), (23, 28)], fill=a2)
    for bx, by in ((10, 26), (17, 25), (22, 26)):
        put_px(body, bx, by, a4)

    # Three-eyed plague demon skull (width 16, height 18: y=6..24)
    d.ellipse([(8, 6), (24, 20)], fill=b2)
    d.rectangle([(11, 17), (21, 24)], fill=b2)
    d.arc([(9, 7), (23, 18)], start=200, end=340, fill=b3, width=2)
    # 3 sunken eye sockets: (12,12), (20,12), forehead (16,9)
    for ex, ey in ((12, 12), (20, 12), (16, 9)):
        d.rectangle([(ex - 1, ey - 1), (ex + 1, ey + 1)], fill=b1)
        put_px(body, ex, ey, a3)
        # Acid streams pouring from sockets
        d.line([(ex, ey + 1), (ex, 25)], fill=a2, width=1)
    # Gaping jaw pouring acid geyser (14,20)-(18,24)
    d.rectangle([(13, 19), (19, 24)], fill=b1)
    d.rectangle([(14, 20), (18, 26)], fill=a3)
    d.line([(16, 20), (16, 26)], fill=a4)

    return finalize(body, "#081005", glow=glow)


# ---------------------------------------------------------------------------
# 7. icon_evo_whirlwind_cleaver.png — Whirlwind Cleaver
# ---------------------------------------------------------------------------

def draw_evo_whirlwind_cleaver() -> Image.Image:
    body, glow, top = new_canvas(), new_canvas(), new_canvas()
    d, dg, dt = ImageDraw.Draw(body), ImageDraw.Draw(glow), ImageDraw.Draw(top)

    ir1, ir2, ir3 = hex_rgba("#18181B"), hex_rgba("#334155"), hex_rgba("#94A3B8")
    r1, r2, r3, r4 = hex_rgba("#7F1D1D"), hex_rgba("#B91C1C"), hex_rgba("#EF4444"), hex_rgba("#FCA5A5")
    oak, spike = hex_rgba("#451A03"), hex_rgba("#64748B")

    dg.ellipse([(5, 3), (29, 27)], fill=hex_rgba("#B91C1C", 65))

    # Diagonal spiked oak shaft (5,27)->(27,5)
    d.line([(5, 27), (27, 5)], fill=oak, width=3)
    d.polygon([(27, 5), (24, 5), (27, 8)], fill=spike)

    # Double-sided massive crescent blades centered on (17,15), 22px span
    left_b = [(16, 13), (9, 6), (6, 11), (9, 17), (15, 16)]
    right_b = [(18, 16), (25, 23), (26, 17), (22, 12), (18, 13)]
    for poly, edge, runes in (
        (left_b, [(9, 6), (6, 11), (9, 17)], ((10, 10), (11, 13), (13, 14))),
        (right_b, [(25, 23), (26, 17), (22, 12)], ((22, 15), (23, 18), (20, 17))),
    ):
        d.polygon(poly, fill=ir2)
        d.line(edge, fill=ir3, width=2)
        for rx, ry in runes:
            put_px(body, rx, ry, r3)
    d.rectangle([(15, 13), (19, 17)], fill=ir1)
    put_px(body, 17, 15, r4)

    # Swirling crimson cleaving arcs around both blades
    dt.arc([(4, 4), (28, 26)], start=140, end=250, fill=r3, width=2)
    dt.arc([(5, 5), (27, 25)], start=320, end=70, fill=r3, width=2)
    put_px(top, 7, 8, r4)
    put_px(top, 25, 21, r4)
    put_px(top, 5, 15, r2)
    put_px(top, 27, 14, r1)

    return finalize(body, "#120406", glow=glow, top=top)


# ---------------------------------------------------------------------------
# 8. icon_evo_starfall_barrage.png — Starfall Barrage
# ---------------------------------------------------------------------------

def draw_evo_starfall_barrage() -> Image.Image:
    body, glow, top = new_canvas(), new_canvas(), new_canvas()
    d, dg, dt = ImageDraw.Draw(body), ImageDraw.Draw(glow), ImageDraw.Draw(top)

    g1, g2, g3 = hex_rgba("#B45309"), hex_rgba("#F59E0B"), hex_rgba("#FEF08A")
    s1, white = hex_rgba("#CBD5E1"), hex_rgba("#FFFFFF")
    c1, c2 = hex_rgba("#38BDF8"), hex_rgba("#E0F2FE")

    dg.ellipse([(8, 2), (30, 24)], fill=hex_rgba("#38BDF8", 65))

    # Winged seraph golden bow (5,8) -> grip (10,21) -> (23,26)
    bow = [(5, 8), (5, 14), (7, 18), (10, 21), (15, 25), (23, 26)]
    d.line(bow, fill=g2, width=3)
    d.line(bow, fill=g3, width=1)
    # Feathered silver wings on bow limbs
    d.polygon([(5, 8), (2, 13), (6, 15)], fill=s1)
    d.polygon([(23, 26), (19, 29), (16, 25)], fill=s1)
    put_px(body, 3, 12, white)
    put_px(body, 19, 28, white)
    # Diamond optical sight on grip with cyan crosshair
    d.polygon([(10, 18), (13, 21), (10, 24), (7, 21)], fill=g1)
    put_px(body, 10, 21, c1)

    # Golden bowstring
    dt.line([(5, 8), (23, 26)], fill=g3, width=1)

    # 3 Fanning comet arrows toward (27,3), (28,9), (22,14)
    for tx, ty in ((27, 3), (28, 9), (24, 15)):
        d.line([(12, 19), (tx, ty)], fill=c1, width=2)
        d.line([(13, 18), (tx, ty)], fill=c2, width=1)
        d.rectangle([(tx - 1, ty), (tx, ty + 1)], fill=white)

    return finalize(body, "#090A14", glow=glow, top=top)


# ---------------------------------------------------------------------------
# 9. icon_evo_thunderstorm_wrath.png — Thunderstorm Wrath
# ---------------------------------------------------------------------------

def draw_evo_thunderstorm_wrath() -> Image.Image:
    body, glow, top = new_canvas(), new_canvas(), new_canvas()
    d, dg, dt = ImageDraw.Draw(body), ImageDraw.Draw(glow), ImageDraw.Draw(top)

    br1, br2, br3 = hex_rgba("#78350F"), hex_rgba("#D97706"), hex_rgba("#FDE68A")
    p1, p2, cy, white = hex_rgba("#4C1D95"), hex_rgba("#7C3AED"), hex_rgba("#22D3EE"), hex_rgba("#FFFFFF")

    dg.ellipse([(5, 5), (27, 27)], fill=hex_rgba("#7C3AED", 70))

    # Hourglass glass bulbs (14x22: y=5..27)
    d.polygon([(10, 7), (22, 7), (17, 16), (22, 25), (10, 25), (15, 16)], fill=p1)
    d.polygon([(12, 8), (20, 8), (16, 15)], fill=p2)
    d.polygon([(12, 24), (20, 24), (16, 17)], fill=p2)
    # Brass top & bottom plates + side pillars
    d.rectangle([(8, 4), (24, 7)], fill=br2)
    d.line([(9, 4), (23, 4)], fill=br3)
    d.rectangle([(8, 25), (24, 28)], fill=br2)
    d.line([(9, 28), (23, 28)], fill=br1)
    d.line([(9, 7), (9, 25)], fill=br1, width=1)
    d.line([(23, 7), (23, 25)], fill=br1, width=1)

    # Ball lightning core in neck (16,16) & 3 escaping arcs
    d.ellipse([(14, 14), (18, 18)], fill=cy)
    put_px(body, 16, 16, white)
    for arc in (
        [(16, 15), (11, 11), (4, 9)],
        [(16, 16), (22, 13), (28, 10)],
        [(16, 17), (22, 21), (28, 24)],
    ):
        dt.line(arc, fill=cy, width=2)
        dt.line(arc, fill=white, width=1)

    return finalize(body, "#080414", glow=glow, top=top)


# ---------------------------------------------------------------------------
# 10. icon_evo_blood_harvest.png — Blood Harvest
# ---------------------------------------------------------------------------

def draw_evo_blood_harvest() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)

    obs, pl, r_gl = hex_rgba("#18181B"), hex_rgba("#475569"), hex_rgba("#DC2626")
    b1, b2, b3, b4 = hex_rgba("#7F1D1D"), hex_rgba("#B91C1C"), hex_rgba("#EF4444"), hex_rgba("#FECDD3")

    dg.ellipse([(5, 4), (27, 26)], fill=hex_rgba("#B91C1C", 65))

    # Crossed scythe/sickle handles at bottom (16,26)
    d.line([(12, 28), (19, 21)], fill=obs, width=2)
    d.line([(20, 28), (13, 21)], fill=obs, width=2)

    # Two massive crescent sickles curving outward and inward forming blood funnel
    left_s = [(14, 22), (7, 17), (5, 9), (12, 5), (10, 10), (11, 16), (15, 20)]
    right_s = [(18, 22), (25, 17), (27, 9), (20, 5), (22, 10), (21, 16), (17, 20)]
    for poly, inner_edge in (
        (left_s, [(12, 5), (10, 10), (11, 16)]),
        (right_s, [(20, 5), (22, 10), (21, 16)]),
    ):
        d.polygon(poly, fill=pl)
        d.line(inner_edge, fill=r_gl, width=1)

    # 4 floating blood orbs inside funnel (sizes 3..6 px)
    for cx, cy, r in ((16, 11, 3), (13, 15, 2), (20, 15, 2), (16, 18, 1)):
        d.ellipse([(cx - r, cy - r), (cx + r, cy + r)], fill=b2)
        if r >= 2:
            d.ellipse([(cx - r + 1, cy - r + 1), (cx + r - 1, cy + r - 1)], fill=b3)
        put_px(body, cx - (1 if r >= 2 else 0), cy - (1 if r >= 2 else 0), b4)

    # Spiral blood pull streams
    put_px(glow, 8, 22, b1)
    put_px(glow, 24, 22, b1)

    return finalize(body, "#140306", glow=glow)


# ---------------------------------------------------------------------------
# 11. icon_evo_event_horizon.png (alias: icon_event_horizon.png)
# ---------------------------------------------------------------------------

def draw_evo_event_horizon() -> Image.Image:
    body, glow, top = new_canvas(), new_canvas(), new_canvas()
    d, dg, dt = ImageDraw.Draw(body), ImageDraw.Draw(glow), ImageDraw.Draw(top)

    black, purp = hex_rgba("#000000"), hex_rgba("#2E1065")
    t1, t2, t3, white = hex_rgba("#0284C7"), hex_rgba("#38BDF8"), hex_rgba("#A5F3FC"), hex_rgba("#FFFFFF")
    c1, c2 = hex_rgba("#B45309"), hex_rgba("#F59E0B")

    dg.ellipse([(3, 3), (29, 29)], fill=hex_rgba("#2E1065", 95))

    # Spiral violet-blue accretion disk
    for ang_deg in range(0, 360, 15):
        ang = math.radians(ang_deg)
        r = 8.5 + 2.0 * math.sin(ang * 2)
        put_px(glow, int(16 + math.cos(ang) * r), int(16 + math.sin(ang) * r), t2)

    # 4 copper Tesla coil electrodes at r=12 (N, S, E, W)
    electrodes = [(16, 4), (16, 28), (4, 16), (28, 16)]
    for ex, ey in electrodes:
        d.rectangle([(ex - 2, ey - 2), (ex + 2, ey + 2)], fill=c1)
        d.rectangle([(ex - 1, ey - 1), (ex + 1, ey + 1)], fill=c2)
        # High-voltage Tesla arc from electrode to black hole rim
        mx = (ex + 16) // 2 + (2 if ey == 16 else -2)
        my = (ey + 16) // 2 + (-2 if ex == 16 else 2)
        dt.line([(ex, ey), (mx, my), (16, 16)], fill=t1, width=2)
        dt.line([(ex, ey), (mx, my), (16, 16)], fill=t3, width=1)

    # Pure black singularity core r=6 on top so event horizon stays pitch-black
    dt.ellipse([(9, 9), (23, 23)], fill=purp)
    dt.ellipse([(10, 10), (22, 22)], fill=black)
    dt.arc([(10, 10), (22, 22)], start=210, end=330, fill=white, width=1)

    return finalize(body, "#05020E", glow=glow, top=top)


# ---------------------------------------------------------------------------
# 12. icon_evo_absolute_zero.png (alias: icon_absolute_zero.png)
# ---------------------------------------------------------------------------

def draw_evo_absolute_zero() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)

    e1, e2, e3, e4 = hex_rgba("#047857"), hex_rgba("#10B981"), hex_rgba("#6EE7B7"), hex_rgba("#ECFDF5")
    i1, i2, white = hex_rgba("#0369A1"), hex_rgba("#38BDF8"), hex_rgba("#FFFFFF")

    dg.ellipse([(3, 3), (29, 29)], fill=hex_rgba("#38BDF8", 65))

    # 6 needle frost spikes radiating from prism vertices
    for ang_deg in (0, 60, 120, 180, 240, 300):
        ang = math.radians(ang_deg)
        d.line(
            [(int(16 + math.cos(ang) * 9), int(16 + math.sin(ang) * 9)),
             (int(16 + math.cos(ang) * 14), int(16 + math.sin(ang) * 14))],
            fill=i2, width=1,
        )

    # Octahedral crystal prism 20x20 centered at (16,16)
    d.polygon([(16, 6), (26, 16), (16, 26), (6, 16)], fill=i1, outline=i2)
    d.polygon([(16, 6), (6, 16), (16, 16)], fill=i2)
    d.line([(16, 6), (26, 16)], fill=white)

    # Four-leaf frost clover suspended inside crystal
    for lx, ly in ((16, 12), (16, 20), (12, 16), (20, 16)):
        d.ellipse([(lx - 2, ly - 2), (lx + 2, ly + 2)], fill=e2)
        put_px(body, lx, ly, e3)
    d.rectangle([(15, 15), (17, 17)], fill=e1)
    put_px(body, 16, 16, e4)

    return finalize(body, "#020C16", glow=glow)


# ---------------------------------------------------------------------------
# 13. icon_evo_infernal_cataclysm.png (alias: icon_infernal_cataclysm.png)
# ---------------------------------------------------------------------------

def draw_evo_infernal_cataclysm() -> Image.Image:
    body, glow, top = new_canvas(), new_canvas(), new_canvas()
    d, dg, dt = ImageDraw.Draw(body), ImageDraw.Draw(glow), ImageDraw.Draw(top)

    obs, r1, r2, r3 = hex_rgba("#1C1917"), hex_rgba("#991B1B"), hex_rgba("#EF4444"), hex_rgba("#FCA5A5")
    l1, l2, l3, white = hex_rgba("#DC2626"), hex_rgba("#F97316"), hex_rgba("#FEF08A"), hex_rgba("#FFFFFF")

    # Incoming sulfur ray into bottom-left prism
    d.line([(2, 26), (10, 21)], fill=l1, width=3)
    d.line([(2, 26), (10, 21)], fill=l3, width=1)

    # 3 Fanning laser beams exiting prism to (27,5), (28,12), (24,19)
    for tx, ty in ((27, 5), (28, 12), (24, 19)):
        dg.line([(14, 19), (tx, ty)], fill=hex_rgba("#F97316", 95), width=5)
        d.line([(14, 19), (tx, ty)], fill=l1, width=3)
        dt.line([(14, 19), (tx, ty)], fill=l2, width=2)
        dt.line([(14, 19), (tx, ty)], fill=white, width=1)

    # Large triangular ruby prism at (11,21)
    d.polygon([(11, 13), (18, 24), (5, 24)], fill=r1, outline=obs)
    d.polygon([(11, 15), (15, 23), (8, 23)], fill=r2)
    d.line([(11, 15), (8, 22)], fill=r3)

    return finalize(body, "#120205", glow=glow, top=top)


# ---------------------------------------------------------------------------
# 14. icon_evo_doomsday_barrage.png (alias: icon_calamity_mortar.png)
# ---------------------------------------------------------------------------

def draw_evo_doomsday_barrage() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)

    t1, t2, t3 = hex_rgba("#1E293B"), hex_rgba("#475569"), hex_rgba("#94A3B8")
    f1, f2, f3 = hex_rgba("#DC2626"), hex_rgba("#F97316"), hex_rgba("#FEF08A")

    dg.ellipse([(10, 4), (29, 23)], fill=hex_rgba("#F97316", 65))

    # 3 heavy nuclear missiles launching out of mortar tubes
    for tx, ty in ((25, 5), (28, 12), (18, 6)):
        bx, by = tx - 7, ty + 7
        d.line([(bx, by), (bx - 4, by + 4)], fill=f1, width=3)
        d.line([(bx, by), (bx - 3, by + 3)], fill=f2, width=2)
        d.line([(bx, by), (bx - 1, by + 1)], fill=f3, width=1)
        d.line([(bx, by), (tx, ty)], fill=t2, width=3)
        d.line([(bx + 1, by - 1), (tx, ty)], fill=t3, width=1)
        put_px(body, tx, ty, f3)

    # Heavy 4-barrel angled titanium mortar block in bottom-left (7,15)-(16,26)
    d.polygon([(5, 22), (13, 14), (20, 21), (12, 29)], fill=t1, outline=t3)
    d.line([(7, 22), (14, 15)], fill=t2, width=2)
    d.line([(10, 25), (17, 18)], fill=t2, width=2)
    # Rivets & smoke puffs
    for rx, ry in ((8, 24), (12, 27), (17, 21)):
        put_px(body, rx, ry, t3)

    return finalize(body, "#0D0C14", glow=glow)


# ---------------------------------------------------------------------------
# 15. icon_evo_storm_god_wrath.png (alias: icon_stormhammer_core.png)
# ---------------------------------------------------------------------------

def draw_evo_storm_god_wrath() -> Image.Image:
    body, glow, top = new_canvas(), new_canvas(), new_canvas()
    d, dg, dt = ImageDraw.Draw(body), ImageDraw.Draw(glow), ImageDraw.Draw(top)

    s1, s2, s3 = hex_rgba("#334155"), hex_rgba("#94A3B8"), hex_rgba("#E2E8F0")
    e1, e2, e3, white = hex_rgba("#1E1B4B"), hex_rgba("#06B6D4"), hex_rgba("#67E8F9"), hex_rgba("#FFFFFF")
    gold = hex_rgba("#F59E0B")

    dg.ellipse([(4, 4), (28, 28)], fill=hex_rgba("#06B6D4", 75))

    # Tilted divine warhammer centered at (16,16)
    d.line([(16, 18), (9, 28)], fill=gold, width=3)
    d.polygon([(11, 11), (21, 11), (23, 19), (13, 19)], fill=s2)
    d.line([(11, 11), (21, 11)], fill=s3, width=2)
    d.line([(13, 19), (23, 19)], fill=s1, width=2)
    d.rectangle([(15, 14), (18, 16)], fill=gold)

    # Vertical titanic lightning column from (16,1) striking hammer
    dt.line([(16, 1), (15, 6), (17, 10), (16, 14)], fill=e3, width=3)
    dt.line([(16, 1), (15, 6), (17, 10), (16, 14)], fill=white, width=1)

    # Branching chain lightning web to all 4 corners
    for bolt in (
        [(13, 14), (7, 10), (3, 5)],
        [(21, 14), (26, 9), (29, 5)],
        [(12, 18), (6, 22), (3, 27)],
        [(21, 18), (26, 23), (29, 27)],
    ):
        dt.line(bolt, fill=e2, width=2)
        dt.line(bolt, fill=e3, width=1)
        put_px(top, bolt[-1][0], bolt[-1][1], white)
    put_px(glow, 16, 16, e1)

    return finalize(body, "#050816", glow=glow, top=top)


# ---------------------------------------------------------------------------
# 16. icon_evo_cathedral_aura.png (alias: icon_halo_radiance.png)
# ---------------------------------------------------------------------------

def draw_evo_cathedral_aura() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)

    g1, g2, g3, g4 = hex_rgba("#B45309"), hex_rgba("#F59E0B"), hex_rgba("#FDE047"), hex_rgba("#FFFBEB")
    w1, w2 = hex_rgba("#F8FAFC"), hex_rgba("#E0F2FE")

    # Concentric golden radiance rings & 8 solar rays
    dg.ellipse([(2, 2), (30, 30)], outline=hex_rgba("#FDE047", 110), width=2)
    dg.ellipse([(5, 5), (27, 27)], outline=hex_rgba("#F59E0B", 130), width=1)
    for i in range(8):
        ang = i * (math.pi / 4.0)
        dg.line(
            [(int(16 + math.cos(ang) * 7), int(16 + math.sin(ang) * 7)),
             (int(16 + math.cos(ang) * 14), int(16 + math.sin(ang) * 14))],
            fill=g3, width=1,
        )

    # Cherub white-silver wings flanking bell
    d.polygon([(11, 15), (3, 10), (4, 17), (10, 19)], fill=w1)
    d.polygon([(21, 15), (29, 10), (28, 17), (22, 19)], fill=w1)
    d.line([(4, 14), (10, 17)], fill=w2)
    d.line([(28, 14), (22, 17)], fill=w2)

    # Central golden temple bell 10x12 at (16,16)
    bell = [(14, 10), (18, 10), (20, 16), (21, 21), (11, 21), (12, 16)]
    d.polygon(bell, fill=g2)
    d.line([(14, 11), (13, 20)], fill=g3, width=2)
    d.line([(11, 21), (21, 21)], fill=g1)
    d.rectangle([(15, 8), (17, 10)], fill=g3)
    d.rectangle([(15, 21), (17, 23)], fill=g4)

    return finalize(body, "#0E0902", glow=glow)


# ---------------------------------------------------------------------------
# 17. icon_evo_banshee_wail.png (alias: icon_spectral_jar.png)
# ---------------------------------------------------------------------------

def draw_evo_banshee_wail() -> Image.Image:
    body, glow, top = new_canvas(), new_canvas(), new_canvas()
    d, dg, dt = ImageDraw.Draw(body), ImageDraw.Draw(glow), ImageDraw.Draw(top)

    jar_d, jar_l, gold = hex_rgba("#0F172A"), hex_rgba("#334155"), hex_rgba("#F59E0B")
    e1, e2, e3, e4, e5 = (
        hex_rgba("#064E3B"), hex_rgba("#059669"), hex_rgba("#10B981"), hex_rgba("#6EE7B7"), hex_rgba("#ECFDF5")
    )

    dg.ellipse([(5, 2), (27, 24)], fill=hex_rgba("#10B981", 70))

    # Shattered onyx jar at bottom (16,24) with golden seal
    d.polygon([(10, 21), (15, 22), (14, 28), (10, 27)], fill=jar_d, outline=jar_l)
    d.polygon([(17, 22), (22, 21), (22, 27), (18, 28)], fill=jar_d, outline=jar_l)
    d.rectangle([(15, 25), (17, 27)], fill=gold)

    # Colossal screaming spectral Banshee skull erupting upward
    # Flowing ectoplasm hair streams
    d.polygon([(16, 22), (9, 14), (5, 7), (11, 5), (21, 5), (27, 7), (23, 14)], fill=e2)
    d.ellipse([(10, 5), (22, 18)], fill=e3)
    d.ellipse([(12, 6), (20, 15)], fill=e4)
    # Hollow eyes blazing emerald + wide screaming jaw
    d.rectangle([(12, 9), (14, 11)], fill=e1)
    d.rectangle([(18, 9), (20, 11)], fill=e1)
    put_px(body, 13, 10, e5)
    put_px(body, 19, 10, e5)
    d.ellipse([(14, 13), (18, 18)], fill=e1)

    # Sonic wail wave arcs radiating from mouth
    dt.arc([(7, 9), (25, 22)], start=200, end=340, fill=e5, width=1)
    dt.arc([(4, 6), (28, 24)], start=210, end=330, fill=hex_rgba("#ECFDF5", 160), width=1)

    return finalize(body, "#02140D", glow=glow, top=top)


# ---------------------------------------------------------------------------
# 18. icon_evo_invincible_bastion.png (alias: icon_immortal_aegis.png)
# ---------------------------------------------------------------------------

def draw_evo_invincible_bastion() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)

    a1, a2, a3 = hex_rgba("#0F172A"), hex_rgba("#334155"), hex_rgba("#94A3B8")
    f1, f2, f3, f4, white = (
        hex_rgba("#1D4ED8"), hex_rgba("#2563EB"), hex_rgba("#60A5FA"), hex_rgba("#DBEAFE"), hex_rgba("#FFFFFF")
    )

    # Triangular force-field barrier linking the 3 shields
    tri = [(16, 7), (8, 22), (24, 22), (16, 7)]
    dg.line(tri, fill=hex_rgba("#2563EB", 110), width=5)
    d.line(tri, fill=f3, width=2)
    d.line(tri, fill=f4, width=1)

    # Central floating rune of absolute protection at (16,17)
    d.polygon([(16, 14), (19, 17), (16, 20), (13, 17)], fill=f2)
    put_px(body, 16, 17, white)

    # Three 7x10 adamantium shields at (16,6), (8,22), (24,22)
    for cx, cy in ((16, 7), (8, 22), (24, 22)):
        poly = [(cx - 3, cy - 4), (cx + 3, cy - 4), (cx + 3, cy + 1), (cx, cy + 5), (cx - 3, cy + 1)]
        d.polygon(poly, fill=a2, outline=a3)
        d.line([(cx, cy - 3), (cx, cy + 3)], fill=f3)
        d.line([(cx - 2, cy - 1), (cx + 2, cy - 1)], fill=f3)
        put_px(body, cx, cy - 1, white)
        put_px(body, cx - 2, cy - 3, a1)
    put_px(glow, 16, 17, f1)

    return finalize(body, "#060A14", glow=glow)


# ---------------------------------------------------------------------------
# 19. icon_evo_armageddon_storm.png (alias: icon_cinder_heart.png)
# ---------------------------------------------------------------------------

def draw_evo_armageddon_storm() -> Image.Image:
    body, glow = new_canvas(), new_canvas()
    d, dg = ImageDraw.Draw(body), ImageDraw.Draw(glow)

    obs, m1, m2 = hex_rgba("#1C1917"), hex_rgba("#991B1B"), hex_rgba("#EF4444")
    f1, f2, f3, f4 = hex_rgba("#DC2626"), hex_rgba("#F97316"), hex_rgba("#FEF08A"), hex_rgba("#FFFBEB")

    dg.ellipse([(4, 4), (28, 28)], fill=hex_rgba("#DC2626", 70))

    # 3 orbiting fiery volcanic meteors spiraling around heart
    for mx, my, tx, ty in ((8, 9, 3, 4), (25, 11, 28, 5), (14, 26, 22, 29)):
        d.line([(mx, my), (tx, ty)], fill=f1, width=3)
        d.line([(mx, my), (tx, ty)], fill=f2, width=1)
        d.ellipse([(mx - 2, my - 2), (mx + 2, my + 2)], fill=obs)
        put_px(body, mx, my, f3)

    # Central crystalline lava Cinder Heart 12x12 at (16,16)
    heart = [(16, 10), (22, 14), (20, 21), (16, 23), (12, 21), (10, 14)]
    d.polygon(heart, fill=obs)
    # Glowing magma fracture network across heart
    d.line([(16, 11), (14, 16), (17, 21)], fill=m2, width=2)
    d.line([(12, 15), (20, 16)], fill=f2, width=2)
    d.rectangle([(15, 15), (17, 17)], fill=f3)
    put_px(body, 16, 16, f4)
    put_px(body, 13, 19, m1)

    return finalize(body, "#120303", glow=glow)


# ---------------------------------------------------------------------------
# 20. icon_evo_singularity_collapse.png (alias: icon_singularity_collapse.png)
# ---------------------------------------------------------------------------

def draw_evo_singularity_collapse() -> Image.Image:
    body, glow, top = new_canvas(), new_canvas(), new_canvas()
    d, dg, dt = ImageDraw.Draw(body), ImageDraw.Draw(glow), ImageDraw.Draw(top)

    v1, v2, v3, v4 = hex_rgba("#581C87"), hex_rgba("#9333EA"), hex_rgba("#C084FC"), hex_rgba("#E879F9")
    white, plasma = hex_rgba("#FFFFFF"), hex_rgba("#FDF4FF")

    dg.ellipse([(2, 2), (30, 30)], fill=hex_rgba("#9333EA", 80))

    # 6 spiral octarine energy streams warping space outward
    for i in range(6):
        base_ang = i * (math.pi / 3.0)
        pts = []
        for step in range(8):
            r = 5 + step * 1.4
            ang = base_ang + step * 0.18
            pts.append((int(round(16 + math.cos(ang) * r)), int(round(16 + math.sin(ang) * r))))
        d.line(pts, fill=v2 if i % 2 == 0 else v3, width=2)

    # 2 concentric elliptical gravitational shockwave rings
    d.ellipse([(4, 7), (28, 25)], outline=v3, width=1)
    d.ellipse([(7, 9), (25, 23)], outline=v4, width=2)

    # Supernova white core r=5 + diffraction cross rays
    dt.line([(16, 3), (16, 29)], fill=plasma, width=1)
    dt.line([(3, 16), (29, 16)], fill=plasma, width=1)
    dt.ellipse([(11, 11), (21, 21)], fill=v4)
    dt.ellipse([(12, 12), (20, 20)], fill=white)
    put_px(glow, 16, 16, v1)

    return finalize(body, "#080214", glow=glow, top=top)


# ---------------------------------------------------------------------------
# Batch Generation Pipeline (Primary Names + Canonical Engine Aliases)
# ---------------------------------------------------------------------------

GENERATORS: list[tuple[str, list[str], Callable[[], Image.Image]]] = [
    ("icon_evo_blade_storm.png", [], draw_evo_blade_storm),
    ("icon_evo_hellfire.png", [], draw_evo_hellfire),
    ("icon_evo_death_ray.png", [], draw_evo_death_ray),
    ("icon_evo_divine_shield.png", [], draw_evo_divine_shield),
    ("icon_evo_blizzard_storm.png", [], draw_evo_blizzard_storm),
    ("icon_evo_toxic_pestilence.png", [], draw_evo_toxic_pestilence),
    ("icon_evo_whirlwind_cleaver.png", [], draw_evo_whirlwind_cleaver),
    ("icon_evo_starfall_barrage.png", [], draw_evo_starfall_barrage),
    ("icon_evo_thunderstorm_wrath.png", [], draw_evo_thunderstorm_wrath),
    ("icon_evo_blood_harvest.png", [], draw_evo_blood_harvest),
    ("icon_evo_event_horizon.png", ["icon_event_horizon.png"], draw_evo_event_horizon),
    ("icon_evo_absolute_zero.png", ["icon_absolute_zero.png"], draw_evo_absolute_zero),
    ("icon_evo_infernal_cataclysm.png", ["icon_infernal_cataclysm.png"], draw_evo_infernal_cataclysm),
    ("icon_evo_doomsday_barrage.png", ["icon_calamity_mortar.png"], draw_evo_doomsday_barrage),
    ("icon_evo_storm_god_wrath.png", ["icon_stormhammer_core.png"], draw_evo_storm_god_wrath),
    ("icon_evo_cathedral_aura.png", ["icon_halo_radiance.png"], draw_evo_cathedral_aura),
    ("icon_evo_banshee_wail.png", ["icon_spectral_jar.png"], draw_evo_banshee_wail),
    ("icon_evo_invincible_bastion.png", ["icon_immortal_aegis.png"], draw_evo_invincible_bastion),
    ("icon_evo_armageddon_storm.png", ["icon_cinder_heart.png"], draw_evo_armageddon_storm),
    ("icon_evo_singularity_collapse.png", ["icon_singularity_collapse.png"], draw_evo_singularity_collapse),
]


def generate_all(out_root: Path) -> list[Path]:
    out_dirs = [out_root / "assets" / "icons", out_root / "assets" / "icons" / "evolutions"]
    for d in out_dirs:
        d.mkdir(parents=True, exist_ok=True)

    saved: list[Path] = []
    for primary, aliases, fn in GENERATORS:
        img = fn()
        for filename in [primary, *aliases]:
            for d in out_dirs:
                path = d / filename
                img.save(path, format="PNG")
                saved.append(path)
    return saved


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Generate 20 Weapon Evolution 32x32 RGBA pixel-art icons + aliases (Group 8)."
    )
    parser.add_argument(
        "--out-root",
        type=Path,
        default=Path("."),
        help="Project root directory where assets/icons/ and assets/icons/evolutions/ will be written.",
    )
    args = parser.parse_args()

    files = generate_all(args.out_root)
    print(f"Generated {len(files)} PNG files under {(args.out_root / 'assets' / 'icons').resolve()}")


if __name__ == "__main__":
    main()
