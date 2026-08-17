"""Renders the Nuru mark to a 1024px master PNG for `tauri icon`.

The mark is drawn rather than traced from the SVG so it can be regenerated at
any size without a rasteriser dependency, and so the app icon and the in-app
mark stay the same shape by construction. Replace this whole script when real
artwork exists.

    python scripts/make-icon.py
    npx tauri icon src-tauri/icons/source.png
"""

from __future__ import annotations

import math
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

SIZE = 1024
SS = 4  # supersample factor; the arcs need it to stay smooth

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "src-tauri" / "icons"

# Matches tokens.css: the app base grey and the brand amber.
BG_TOP = (26, 28, 32)
BG_BOTTOM = (13, 14, 16)
AMBER = (255, 180, 84)
AMBER_BRIGHT = (255, 201, 120)


def lerp(a: tuple[int, ...], b: tuple[int, ...], t: float) -> tuple[int, ...]:
    return tuple(round(x + (y - x) * t) for x, y in zip(a, b))


def build() -> Image.Image:
    n = SIZE * SS
    img = Image.new("RGBA", (n, n), (0, 0, 0, 0))

    # Rounded-square plate with a vertical gradient, matching the tile radius
    # ratio used in the UI (20px on a 200px tile → 10%).
    plate = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    pd = ImageDraw.Draw(plate)
    for y in range(n):
        pd.line([(0, y), (n, y)], fill=(*lerp(BG_TOP, BG_BOTTOM, y / n), 255))

    mask = Image.new("L", (n, n), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, n - 1, n - 1], radius=int(n * 0.22), fill=255)
    img.paste(plate, (0, 0), mask)

    # Warm bloom behind the core, so the mark reads as a light source rather
    # than a flat glyph.
    glow = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    gd = ImageDraw.Draw(glow)
    cx, cy = n / 2, n * 0.645
    gd.ellipse(
        [cx - n * 0.30, cy - n * 0.30, cx + n * 0.30, cy + n * 0.30],
        fill=(*AMBER, 120),
    )
    glow = glow.filter(ImageFilter.GaussianBlur(n * 0.075))
    img.alpha_composite(Image.composite(glow, Image.new("RGBA", (n, n), (0, 0, 0, 0)), mask))

    d = ImageDraw.Draw(img)

    # Core.
    r = n * 0.098
    d.ellipse([cx - r, cy - r, cx + r, cy + r], fill=(*AMBER_BRIGHT, 255))

    # Two arcs rising off it. Drawn as upper semicircles, matching NuruMark.svelte.
    for radius_f, width_f, alpha in ((0.185, 0.055, 200), (0.300, 0.049, 105)):
        rr = n * radius_f
        d.arc(
            [cx - rr, cy - rr, cx + rr, cy + rr],
            start=180,
            end=360,
            fill=(*AMBER, alpha),
            width=max(1, int(n * width_f)),
        )

    # Round the arc terminals so they match the SVG's stroke-linecap.
    for radius_f, width_f, alpha in ((0.185, 0.055, 200), (0.300, 0.049, 105)):
        rr = n * radius_f
        cap = n * width_f / 2
        for sx in (cx - rr, cx + rr):
            d.ellipse([sx - cap, cy - cap, sx + cap, cy + cap], fill=(*AMBER, alpha))

    # Trim anything the arcs pushed past the plate.
    img.putalpha(Image.composite(img.getchannel("A"), Image.new("L", (n, n), 0), mask))

    return img.resize((SIZE, SIZE), Image.LANCZOS)


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    icon = build()
    icon.save(OUT / "source.png")
    print(f"wrote {OUT / 'source.png'} ({SIZE}x{SIZE})")


if __name__ == "__main__":
    main()
