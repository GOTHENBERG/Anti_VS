#!/usr/bin/env python3
"""Procedural pixel-art generator for Boss Weapons 22, 23, 28 (Group 7 Extension).

Generates 3 crisp 32x32 RGBA weapon icons in Dark Fantasy Gothic Pixel Art
style with transparent backgrounds, Tri-Layer Luminance contrast, and 1px
dark silhouette outlines:
  - 22: icon_faith_boomerang.png (Faith Boomerang / Клинок Веры)
  - 23: icon_cursed_chalice.png  (Cursed Chalice / Осквернённая Чаша)
  - 28: icon_astral_cloak.png    (Astral Cloak / Астральный Саван)

Outputs are saved to `assets/icons/` and mirrored to `assets/icons/weapons/`.

Usage:
    python scripts/generate_missing_weapons.py
    python scripts/generate_missing_weapons.py --out-root .
"""

from __future__ import annotations

import argparse
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
# 22. icon_faith_boomerang.png — Faith Boomerang (Клинок Веры)
# ---------------------------------------------------------------------------

def draw_faith_boomerang() -> Image.Image:
    body = new_canvas()
    glow = new_canvas()
    top = new_canvas()
    d = ImageDraw.Draw(body)
    dg = ImageDraw.Draw(glow)
    dt = ImageDraw.Draw(top)

    g_dark, g_mid, g_bright, g_spec = (
        hex_rgba("#78350F"), hex_rgba("#D97706"), hex_rgba("#FACC15"), hex_rgba("#FEF08A")
    )
    s_dark, s_mid, s_bright, s_glint = (
        hex_rgba("#334155"), hex_rgba("#94A3B8"), hex_rgba("#E2E8F0"), hex_rgba("#FFFFFF")
    )
    c_deep, c_mid, c_cyan, c_spark = (
        hex_rgba("#0C4A6E"), hex_rgba("#0284C7"), hex_rgba("#38BDF8"), hex_rgba("#CFFAFE")
    )

    # 1. Radiant Solar Glow
    dg.ellipse([(5, 5), (27, 27)], fill=hex_rgba("#F59E0B", 45))

    # 2. Four Aerodynamic Curved Holy Blades (Diagonal Cruciform Pinwheel)
    # Blade 1: Top-Right (curves toward top edge)
    d.polygon([(17, 13), (21, 9), (28, 4), (25, 10), (19, 15)], fill=g_mid)
    d.line([(17, 13), (21, 9), (28, 4)], fill=g_bright, width=1)
    d.line([(18, 14), (25, 10)], fill=s_bright, width=1)
    put_px(body, 28, 4, s_glint)

    # Blade 2: Bottom-Right (curves toward bottom edge)
    d.polygon([(19, 17), (23, 21), (28, 28), (22, 25), (17, 19)], fill=g_mid)
    d.line([(19, 17), (23, 21), (28, 28)], fill=g_bright, width=1)
    d.line([(18, 18), (22, 25)], fill=g_dark, width=1)
    put_px(body, 28, 28, g_spec)

    # Blade 3: Bottom-Left (curves toward left edge)
    d.polygon([(15, 19), (11, 23), (4, 28), (7, 22), (13, 17)], fill=g_mid)
    d.line([(15, 19), (11, 23), (4, 28)], fill=g_bright, width=1)
    d.line([(14, 18), (7, 22)], fill=s_mid, width=1)
    put_px(body, 4, 28, s_bright)

    # Blade 4: Top-Left (curves toward top edge)
    d.polygon([(13, 15), (9, 11), (4, 4), (10, 7), (15, 13)], fill=g_mid)
    d.line([(13, 15), (9, 11), (4, 4)], fill=g_bright, width=1)
    d.line([(14, 14), (10, 7)], fill=s_bright, width=1)
    put_px(body, 4, 4, s_glint)

    # 3. Central Sanctified Gold Boss (Faceted Octagonal Hub)
    d.polygon([(13, 15), (15, 13), (17, 13), (19, 15), (19, 17), (17, 19), (15, 19), (13, 17)], fill=g_dark)
    d.polygon([(14, 15), (15, 14), (17, 14), (18, 15), (18, 17), (17, 18), (15, 18), (14, 17)], fill=g_bright)

    # 4. Inlaid Holy Sapphire Core with Specular Glint
    d.rectangle([(15, 15), (17, 17)], fill=c_mid)
    put_px(body, 16, 16, c_cyan)
    put_px(body, 15, 15, s_glint)
    put_px(body, 17, 17, c_deep)

    # 5. Holy Cardinal Ray Glints
    dt.line([(16, 11), (16, 8)], fill=hex_rgba("#FEF08A", 180), width=1)
    dt.line([(16, 21), (16, 24)], fill=hex_rgba("#FEF08A", 180), width=1)
    dt.line([(11, 16), (8, 16)], fill=hex_rgba("#FEF08A", 180), width=1)
    dt.line([(21, 16), (24, 16)], fill=hex_rgba("#FEF08A", 180), width=1)

    return finalize(body, "#1A0C02", glow=glow, top=top)


# ---------------------------------------------------------------------------
# 23. icon_cursed_chalice.png — Cursed Chalice (Осквернённая Чаша)
# ---------------------------------------------------------------------------

def draw_cursed_chalice() -> Image.Image:
    body = new_canvas()
    glow = new_canvas()
    top = new_canvas()
    d = ImageDraw.Draw(body)
    dg = ImageDraw.Draw(glow)
    dt = ImageDraw.Draw(top)

    m_dark, m_mid, m_bright, m_hi = (
        hex_rgba("#0F172A"), hex_rgba("#334155"), hex_rgba("#64748B"), hex_rgba("#CBD5E1")
    )
    g_dark, g_mid, g_bright = (
        hex_rgba("#78350F"), hex_rgba("#B45309"), hex_rgba("#F59E0B")
    )
    b_deep, b_crimson, b_flame, b_foam, b_molten = (
        hex_rgba("#450A0A"), hex_rgba("#991B1B"), hex_rgba("#EF4444"), hex_rgba("#F87171"), hex_rgba("#FEF08A")
    )

    # 1. Demonic Crimson Miasma Glow
    dg.ellipse([(6, 8), (26, 28)], fill=hex_rgba("#EF4444", 55))

    # 2. Ornate Base and Pedestal
    d.rectangle([(10, 27), (22, 28)], fill=m_dark)
    d.line([(11, 27), (21, 27)], fill=g_bright, width=1)
    d.polygon([(12, 26), (20, 26), (18, 23), (14, 23)], fill=m_mid)
    # Stem with corrupted gold demonic knot
    d.rectangle([(15, 20), (17, 23)], fill=m_bright)
    put_px(body, 16, 21, g_bright)
    put_px(body, 16, 22, g_mid)

    # 3. Flared Gothic Goblet Bowl
    cup_outer = [(13, 20), (19, 20), (24, 14), (25, 11), (7, 11), (8, 14)]
    d.polygon(cup_outer, fill=m_dark)
    d.polygon([(14, 19), (18, 19), (23, 14), (23, 12), (9, 12), (9, 14)], fill=m_mid)

    # 4. Ornate Dragon-Claw Side Handles
    # Left handle
    d.line([(8, 12), (5, 15), (6, 18), (10, 19)], fill=g_bright, width=1)
    put_px(body, 5, 15, m_hi)
    # Right handle
    d.line([(24, 12), (27, 15), (26, 18), (22, 19)], fill=g_mid, width=1)
    put_px(body, 27, 15, g_bright)

    # 5. Churning Boiling Cursed Blood-Fire Liquid
    d.polygon([(9, 12), (23, 12), (22, 16), (10, 16)], fill=b_deep)
    d.polygon([(10, 12), (22, 12), (21, 15), (11, 15)], fill=b_crimson)
    d.line([(10, 12), (22, 12)], fill=b_flame, width=1)
    # Boiling bubbles & molten foam
    put_px(body, 13, 12, b_molten)
    put_px(body, 18, 13, b_foam)
    put_px(body, 15, 14, b_flame)

    # 6. Overflowing Blood Drips running down the front
    d.line([(12, 12), (12, 16), (13, 18)], fill=b_crimson, width=1)
    put_px(body, 13, 19, b_flame)
    put_px(body, 13, 18, b_foam)

    # 7. Gothic Spiked Gold Rim
    d.line([(7, 11), (25, 11)], fill=g_bright, width=1)
    put_px(body, 7, 10, g_bright)
    put_px(body, 16, 11, m_hi)
    put_px(body, 25, 10, g_bright)

    # 8. Rising Unholy Soul Embers / Fumes
    dt.line([(14, 9), (13, 7)], fill=b_flame, width=1)
    put_px(top, 13, 6, b_molten)
    dt.line([(18, 9), (19, 7)], fill=b_crimson, width=1)
    put_px(top, 19, 6, b_foam)
    put_px(top, 16, 5, b_molten)

    return finalize(body, "#0B0404", glow=glow, top=top)


# ---------------------------------------------------------------------------
# 28. icon_astral_cloak.png — Astral Cloak (Астральный Саван)
# ---------------------------------------------------------------------------

def draw_astral_cloak() -> Image.Image:
    body = new_canvas()
    glow = new_canvas()
    top = new_canvas()
    d = ImageDraw.Draw(body)
    dg = ImageDraw.Draw(glow)
    dt = ImageDraw.Draw(top)

    v_black, v_deep, v_indigo, v_purple = (
        hex_rgba("#020617"), hex_rgba("#0F172A"), hex_rgba("#1E1B4B"), hex_rgba("#312E81")
    )
    c_deep, c_mid, c_cyan, c_neon, c_spark = (
        hex_rgba("#083344"), hex_rgba("#0891B2"), hex_rgba("#06B6D4"), hex_rgba("#22D3EE"), hex_rgba("#CFFAFE")
    )
    pure_white = hex_rgba("#FFFFFF")

    # 1. Soft Astral Cyan Nebula Aura
    dg.ellipse([(5, 5), (27, 27)], fill=hex_rgba("#06B6D4", 45))

    # 2. Flowing Gothic Mantle Silhouette (Draped shoulders to billowy hem)
    cloak_poly = [
        (13, 6), (19, 6), (24, 9), (25, 15), (26, 26), (22, 28),
        (18, 25), (16, 27), (14, 25), (10, 28), (6, 26), (7, 15), (8, 9)
    ]
    d.polygon(cloak_poly, fill=v_black)

    # 3. Outer Illuminated Mantle Folds & Fabric Waves
    d.polygon([(8, 9), (13, 7), (11, 20), (7, 22)], fill=v_indigo)
    d.polygon([(24, 9), (19, 7), (21, 20), (25, 22)], fill=v_deep)
    d.polygon([(11, 10), (14, 8), (14, 24), (10, 24)], fill=v_purple)
    d.polygon([(21, 10), (18, 8), (18, 24), (22, 24)], fill=v_indigo)

    # Central inner deep-space shadow pleat
    d.polygon([(14, 11), (18, 11), (17, 24), (15, 24)], fill=v_black)

    # 4. High Gothic Flared Collar
    d.polygon([(10, 6), (8, 4), (12, 6)], fill=c_mid)
    d.polygon([(22, 6), (24, 4), (20, 6)], fill=c_mid)
    put_px(body, 8, 4, c_neon)
    put_px(body, 24, 4, c_neon)

    # 5. Constellation Starlight embroidered across the cosmic fabric
    # Left Constellation
    d.line([(10, 14), (12, 17), (9, 21)], fill=c_deep, width=1)
    put_px(body, 10, 14, c_spark)
    put_px(body, 12, 17, c_neon)
    put_px(body, 9, 21, pure_white)
    # Right Constellation
    d.line([(22, 14), (20, 18), (23, 22)], fill=c_deep, width=1)
    put_px(body, 22, 14, c_neon)
    put_px(body, 20, 18, c_spark)
    put_px(body, 23, 22, pure_white)

    # 6. Central Celestial Brooch / Star Clasp at Neck
    d.rectangle([(15, 8), (17, 10)], fill=c_mid)
    d.line([(16, 7), (16, 11)], fill=c_neon, width=1)
    d.line([(14, 9), (18, 9)], fill=c_neon, width=1)
    put_px(body, 16, 9, pure_white)

    # 7. Hem Stardust Edge
    d.line([(7, 26), (10, 27)], fill=c_mid, width=1)
    d.line([(22, 27), (25, 26)], fill=c_mid, width=1)
    put_px(body, 16, 27, c_neon)

    # 8. Three Floating Astral Shield Orbs (3 Ward Charges)
    # Orb 1: Top (16, 2)
    dt.polygon([(16, 1), (18, 3), (16, 5), (14, 3)], fill=c_neon)
    put_px(top, 16, 3, pure_white)
    # Orb 2: Left (4, 17)
    dt.polygon([(4, 15), (6, 17), (4, 19), (2, 17)], fill=c_neon)
    put_px(top, 4, 17, pure_white)
    # Orb 3: Right (28, 17)
    dt.polygon([(28, 15), (30, 17), (28, 19), (26, 17)], fill=c_neon)
    put_px(top, 28, 17, pure_white)

    return finalize(body, "#050714", glow=glow, top=top)


# ---------------------------------------------------------------------------
# Batch Generator
# ---------------------------------------------------------------------------

GENERATORS: list[tuple[str, Callable[[], Image.Image]]] = [
    ("icon_faith_boomerang.png", draw_faith_boomerang),
    ("icon_cursed_chalice.png", draw_cursed_chalice),
    ("icon_astral_cloak.png", draw_astral_cloak),
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
        description="Generate Boss Weapons 22, 23, 28 32x32 RGBA pixel-art icons."
    )
    parser.add_argument(
        "--out-root",
        type=Path,
        default=Path("."),
        help="Project root directory where assets/icons/ will be written.",
    )
    args = parser.parse_args()

    files = generate_all(args.out_root)
    print(f"Generated {len(files)} PNG files under {(args.out_root / 'assets' / 'icons').resolve()}")


if __name__ == "__main__":
    main()
