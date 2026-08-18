from __future__ import annotations

import math
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFilter

SIZE = 1024
SS = 4

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "src-tauri" / "icons"

BG_TOP = (26, 28, 32)
BG_BOTTOM = (13, 14, 16)
AMBER = (255, 180, 84)
AMBER_BRIGHT = (255, 201, 120)

def lerp(a: tuple[int, ...], b: tuple[int, ...], t: float) -> tuple[int, ...]:
    return tuple(round(x + (y - x) * t) for x, y in zip(a, b))

def build() -> Image.Image:
    n = SIZE * SS
    base = Image.new("RGB", (n, n), (0, 0, 0))
    masks = []

    r = n * 0.255
    cx, cy = n / 2, n / 2
    spread = n * 0.155
    centres = [
        (cx, cy - spread * 1.05),
        (cx - spread, cy + spread * 0.72),
        (cx + spread, cy + spread * 0.72),
    ]
    colours = [(255, 196, 104), (255, 142, 108), (255, 226, 150)]

    for (px, py), colour in zip(centres, colours):
        layer = Image.new("RGB", (n, n), (0, 0, 0))
        mask = Image.new("L", (n, n), 0)
        ImageDraw.Draw(layer).ellipse([px - r, py - r, px + r, py + r], fill=colour)
        ImageDraw.Draw(mask).ellipse([px - r, py - r, px + r, py + r], fill=255)
        base = ImageChops.add(base, layer)
        masks.append(mask)

    alpha = masks[0]
    for m in masks[1:]:
        alpha = ImageChops.lighter(alpha, m)

    img = base.convert("RGBA")
    img.putalpha(alpha)

    glow = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    ImageDraw.Draw(glow).ellipse(
        [cx - n * 0.36, cy - n * 0.36, cx + n * 0.36, cy + n * 0.36],
        fill=(*AMBER, 90),
    )
    glow = glow.filter(ImageFilter.GaussianBlur(n * 0.06))
    out = Image.alpha_composite(glow, img)

    return out.resize((SIZE, SIZE), Image.LANCZOS)


MSIX_ASSETS = {
    "Square44x44Logo": [(44, 100), (55, 125), (66, 150), (88, 200), (176, 400)],
    "Square150x150Logo": [(150, 100), (188, 125), (225, 150), (300, 200), (600, 400)],
    "Square71x71Logo": [(71, 100), (89, 125), (107, 150), (142, 200), (284, 400)],
    "Square310x310Logo": [(310, 100), (388, 125), (465, 150), (620, 200), (1240, 400)],
    "StoreLogo": [(50, 100), (63, 125), (75, 150), (100, 200), (200, 400)],
}

TARGET_SIZES = [16, 24, 32, 48, 256]

def write_msix_assets(icon: Image.Image) -> None:
    out = ROOT / "msix" / "Assets"
    out.mkdir(parents=True, exist_ok=True)

    for name, variants in MSIX_ASSETS.items():
        for px, scale in variants:
            icon.resize((px, px), Image.LANCZOS).save(out / f"{name}.scale-{scale}.png")
        base = variants[0][0]
        icon.resize((base, base), Image.LANCZOS).save(out / f"{name}.png")

    for px in TARGET_SIZES:
        resized = icon.resize((px, px), Image.LANCZOS)
        resized.save(out / f"Square44x44Logo.targetsize-{px}.png")
        resized.save(out / f"Square44x44Logo.altform-unplated_targetsize-{px}.png")

    for px, scale in [(310, 100), (388, 125), (465, 150), (620, 200), (1240, 400)]:
        h = round(px * 150 / 310)
        wide = Image.new("RGBA", (px, h), (0, 0, 0, 0))
        mark = icon.resize((h, h), Image.LANCZOS)
        wide.paste(mark, ((px - h) // 2, 0), mark)
        wide.save(out / f"Wide310x150Logo.scale-{scale}.png")
        if scale == 100:
            wide.save(out / "Wide310x150Logo.png")

    for px, scale in [(620, 100), (775, 125), (930, 150), (1240, 200), (2480, 400)]:
        h = round(px * 300 / 620)
        splash = Image.new("RGBA", (px, h), (19, 20, 23, 255))
        m = round(h * 0.55)
        mark = icon.resize((m, m), Image.LANCZOS)
        splash.paste(mark, ((px - m) // 2, (h - m) // 2), mark)
        splash.save(out / f"SplashScreen.scale-{scale}.png")
        if scale == 100:
            splash.save(out / "SplashScreen.png")

    count = len(list(out.glob("*.png")))
    print(f"wrote {count} MSIX assets to {out}")

def write_svg() -> None:
    svg = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" fill="none">
  <g style="mix-blend-mode:screen">
    <circle cx="32" cy="22" r="16" fill="#ffc468"/>
    <circle cx="22" cy="41" r="16" fill="#ff8e6c"/>
    <circle cx="42" cy="41" r="16" fill="#ffe296"/>
  </g>
</svg>
"""
    target = ROOT / "docs" / "assets" / "icon.svg"
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(svg, encoding="utf-8")
    print(f"wrote {target}")


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    icon = build()
    icon.save(OUT / "source.png")
    print(f"wrote {OUT / 'source.png'} ({SIZE}x{SIZE})")
    write_msix_assets(icon)
    write_svg()

    web = ROOT / "docs" / "assets" / "icon.png"
    web.parent.mkdir(parents=True, exist_ok=True)
    icon.resize((256, 256), Image.LANCZOS).save(web)
    print(f"wrote {web}")

if __name__ == "__main__":
    main()
