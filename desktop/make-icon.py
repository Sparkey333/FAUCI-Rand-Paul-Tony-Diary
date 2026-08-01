#!/usr/bin/env python3
"""Generate the app icon source, app-icon.png.

Kept in the repo so the icon is reproducible rather than an opaque binary blob.
Writes a PNG directly (zlib + CRC) so it has no third-party dependencies.

The mark: concentric rings with a wedge removed — a fingerprint with a break in
it, for a tool whose whole job is noticing when something does not match.

Usage: python3 desktop/make-icon.py [size]
"""

import math
import struct
import sys
import zlib
from pathlib import Path

BACKGROUND = (15, 23, 42)  # slate-950
RING = (56, 189, 248)  # sky-400
ACCENT = (251, 191, 36)  # amber-400


def _chunk(tag: bytes, data: bytes) -> bytes:
    return (
        struct.pack(">I", len(data))
        + tag
        + data
        + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
    )


def write_png(path: Path, pixels: list[list[tuple[int, int, int]]]) -> None:
    """Write 8-bit truecolour RGB with no filtering."""
    raw = b"".join(b"\x00" + bytes(v for px in row for v in px) for row in pixels)
    header = struct.pack(">IIBBBBB", len(pixels[0]), len(pixels), 8, 2, 0, 0, 0)
    path.write_bytes(
        b"\x89PNG\r\n\x1a\n"
        + _chunk(b"IHDR", header)
        + _chunk(b"IDAT", zlib.compress(raw, 9))
        + _chunk(b"IEND", b"")
    )


def blend(base, over, alpha):
    return tuple(round(b + (o - b) * alpha) for b, o in zip(base, over))


def coverage(distance: float, edge: float, softness: float = 1.0) -> float:
    """Antialiased step: 1 well inside the edge, 0 well outside."""
    return max(0.0, min(1.0, (edge - distance) / softness + 0.5))


def render(size: int) -> list[list[tuple[int, int, int]]]:
    centre = (size - 1) / 2
    corner = size * 0.235  # rounded-square radius
    # A thin, even stroke across all rings keeps the mark legible at 32px.
    stroke = 0.0165
    rings = [
        (0.088, stroke, RING),
        (0.161, stroke, RING),
        (0.234, stroke, ACCENT),
        (0.307, stroke, RING),
        (0.380, stroke, RING),
    ]
    # Wedge removed from each ring, pointing right.
    gap_centre, gap_half = 0.0, math.radians(23)
    softness = max(1.0, size / 256)

    rows = []
    for y in range(size):
        row = []
        for x in range(size):
            dx, dy = x - centre, y - centre

            # Rounded-square mask, so the icon sits correctly in a macOS dock.
            ox = max(abs(dx) - (centre - corner), 0.0)
            oy = max(abs(dy) - (centre - corner), 0.0)
            inside = coverage(math.hypot(ox, oy), corner, softness)
            if inside <= 0.0:
                row.append((0, 0, 0))
                continue

            px = blend((0, 0, 0), BACKGROUND, inside)
            radius = math.hypot(dx, dy)
            angle = math.atan2(dy, dx)
            delta = abs((angle - gap_centre + math.pi) % (2 * math.pi) - math.pi)

            if delta > gap_half:
                # Fade the ring out as it approaches the wedge, so the break
                # reads as deliberate rather than clipped.
                edge_fade = min(1.0, (delta - gap_half) / math.radians(7))
                for offset, half, colour in rings:
                    band = half * size
                    alpha = coverage(abs(radius - offset * size), band, softness)
                    if alpha > 0:
                        px = blend(px, colour, alpha * edge_fade * inside)
            row.append(px)
        rows.append(row)
    return rows


def main() -> int:
    size = int(sys.argv[1]) if len(sys.argv) > 1 else 1024
    out = Path(__file__).resolve().parent / "app-icon.png"
    write_png(out, render(size))
    print(f"wrote {out} ({size}x{size})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
