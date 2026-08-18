from __future__ import annotations

import math
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

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
    img = Image.new("RGBA", (n, n), (0, 0, 0, 0))

    plate = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    pd = ImageDraw.Draw(plate)
    for y in range(n):
        pd.line([(0, y), (n, y)], fill=(*lerp(BG_TOP, BG_BOTTOM, y / n), 255))

    mask = Image.new("L", (n, n), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, n - 1, n - 1], radius=int(n * 0.22), fill=255)
    img.paste(plate, (0, 0), mask)

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

    r = n * 0.098
    d.ellipse([cx - r, cy - r, cx + r, cy + r], fill=(*AMBER_BRIGHT, 255))

    for radius_f, width_f, alpha in ((0.185, 0.055, 200), (0.300, 0.049, 105)):
        rr = n * radius_f
        d.arc(
            [cx - rr, cy - rr, cx + rr, cy + rr],
            start=180,
            end=360,
            fill=(*AMBER, alpha),
            width=max(1, int(n * width_f)),
        )

    for radius_f, width_f, alpha in ((0.185, 0.055, 200), (0.300, 0.049, 105)):
        rr = n * radius_f
        cap = n * width_f / 2
        for sx in (cx - rr, cx + rr):
            d.ellipse([sx - cap, cy - cap, sx + cap, cy + cap], fill=(*AMBER, alpha))

    img.putalpha(Image.composite(img.getchannel("A"), Image.new("L", (n, n), 0), mask))

    return img.resize((SIZE, SIZE), Image.LANCZOS)

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
        wide = Image.new("RGBA", (px, h), (19, 20, 23, 255))
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

def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    icon = build()
    icon.save(OUT / "source.png")
    print(f"wrote {OUT / 'source.png'} ({SIZE}x{SIZE})")
    write_msix_assets(icon)

if __name__ == "__main__":
    main()
