#!/usr/bin/env python3
"""Generate Hunk app icon variants (default, dark, mono) as PNG files.

No third-party dependencies are required.
"""

from __future__ import annotations

import math
import struct
import zlib
from pathlib import Path

SIZE = 1024


class Theme:
    def __init__(
        self,
        bg_top: tuple[float, float, float],
        bg_bottom: tuple[float, float, float],
        bg_glow: tuple[float, float, float],
        panel: tuple[float, float, float],
        panel_alpha: float,
        divider: tuple[float, float, float],
        divider_alpha: float,
        minus_main: tuple[float, float, float],
        plus_main: tuple[float, float, float],
        minus_soft: tuple[float, float, float],
        plus_soft: tuple[float, float, float],
        branch: tuple[float, float, float],
        branch_glow: tuple[float, float, float],
        border: tuple[float, float, float],
        border_alpha: float,
    ) -> None:
        self.bg_top = bg_top
        self.bg_bottom = bg_bottom
        self.bg_glow = bg_glow
        self.panel = panel
        self.panel_alpha = panel_alpha
        self.divider = divider
        self.divider_alpha = divider_alpha
        self.minus_main = minus_main
        self.plus_main = plus_main
        self.minus_soft = minus_soft
        self.plus_soft = plus_soft
        self.branch = branch
        self.branch_glow = branch_glow
        self.border = border
        self.border_alpha = border_alpha


DEFAULT_THEME = Theme(
    bg_top=(35, 92, 224),
    bg_bottom=(19, 42, 114),
    bg_glow=(120, 188, 255),
    panel=(8, 16, 34),
    panel_alpha=0.66,
    divider=(168, 192, 245),
    divider_alpha=0.34,
    minus_main=(255, 96, 116),
    plus_main=(86, 230, 148),
    minus_soft=(255, 145, 162),
    plus_soft=(132, 243, 180),
    branch=(242, 248, 255),
    branch_glow=(150, 216, 255),
    border=(198, 222, 255),
    border_alpha=0.35,
)

DARK_THEME = Theme(
    bg_top=(18, 22, 32),
    bg_bottom=(7, 9, 14),
    bg_glow=(52, 84, 150),
    panel=(22, 27, 42),
    panel_alpha=0.82,
    divider=(141, 161, 207),
    divider_alpha=0.28,
    minus_main=(255, 114, 138),
    plus_main=(101, 235, 162),
    minus_soft=(255, 153, 172),
    plus_soft=(144, 244, 188),
    branch=(230, 238, 255),
    branch_glow=(96, 132, 208),
    border=(120, 146, 195),
    border_alpha=0.42,
)

MONO_THEME = Theme(
    bg_top=(28, 28, 31),
    bg_bottom=(14, 14, 16),
    bg_glow=(86, 86, 90),
    panel=(0, 0, 0),
    panel_alpha=0.44,
    divider=(205, 205, 212),
    divider_alpha=0.3,
    minus_main=(235, 235, 235),
    plus_main=(235, 235, 235),
    minus_soft=(188, 188, 196),
    plus_soft=(188, 188, 196),
    branch=(255, 255, 255),
    branch_glow=(190, 190, 190),
    border=(224, 224, 228),
    border_alpha=0.34,
)


def clamp_u8(value: float) -> int:
    return max(0, min(255, int(round(value))))


def lerp(a: float, b: float, t: float) -> float:
    return a + (b - a) * t


def mix(a: tuple[float, float, float], b: tuple[float, float, float], t: float) -> tuple[float, float, float]:
    return (
        lerp(a[0], b[0], t),
        lerp(a[1], b[1], t),
        lerp(a[2], b[2], t),
    )


def smoothstep(edge0: float, edge1: float, x: float) -> float:
    if edge0 == edge1:
        return 1.0 if x >= edge1 else 0.0
    t = (x - edge0) / (edge1 - edge0)
    t = max(0.0, min(1.0, t))
    return t * t * (3.0 - 2.0 * t)


def sd_round_rect(px: float, py: float, cx: float, cy: float, half_w: float, half_h: float, radius: float) -> float:
    dx = abs(px - cx) - (half_w - radius)
    dy = abs(py - cy) - (half_h - radius)
    qx = max(dx, 0.0)
    qy = max(dy, 0.0)
    outside = math.hypot(qx, qy)
    inside = min(max(dx, dy), 0.0)
    return outside + inside - radius


def dist_segment(px: float, py: float, ax: float, ay: float, bx: float, by: float) -> float:
    vx = bx - ax
    vy = by - ay
    wx = px - ax
    wy = py - ay
    vv = vx * vx + vy * vy
    if vv <= 1e-8:
        return math.hypot(px - ax, py - ay)
    t = max(0.0, min(1.0, (wx * vx + wy * vy) / vv))
    cx = ax + t * vx
    cy = ay + t * vy
    return math.hypot(px - cx, py - cy)


def overlay(base: tuple[float, float, float], layer: tuple[float, float, float], alpha: float) -> tuple[float, float, float]:
    inv = 1.0 - alpha
    return (
        base[0] * inv + layer[0] * alpha,
        base[1] * inv + layer[1] * alpha,
        base[2] * inv + layer[2] * alpha,
    )


def px(n: float) -> float:
    return n / SIZE


def render_icon(theme: Theme) -> bytes:
    # Layout in normalized coordinates.
    outer_half = px(430)
    outer_radius = px(180)

    panel_half_w = px(286)
    panel_half_h = px(250)
    panel_radius = px(72)

    divider_half_w = px(6)

    row_h = px(28)
    row_round = px(10)
    left_x0 = px(258)
    left_x1 = px(501)
    right_x0 = px(523)
    right_x1 = px(766)

    row_ys = [px(v) for v in (348, 396, 444, 492, 540, 588, 636)]
    left_lens = [0.86, 0.63, 0.78, 0.58, 0.82, 0.67, 0.74]
    right_lens = [0.64, 0.84, 0.57, 0.79, 0.61, 0.88, 0.68]

    branch_pts = [
        (px(282), px(682)),
        (px(396), px(592)),
        (px(512), px(500)),
        (px(642), px(404)),
        (px(748), px(330)),
    ]
    node_r = px(16)

    data = bytearray()
    for y in range(SIZE):
        data.append(0)  # PNG filter
        fy = (y + 0.5) / SIZE
        for x in range(SIZE):
            fx = (x + 0.5) / SIZE

            # Icon mask.
            d_outer = sd_round_rect(fx, fy, 0.5, 0.5, outer_half, outer_half, outer_radius)
            if d_outer > 0.0:
                data.extend((0, 0, 0, 0))
                continue

            edge_soft = smoothstep(0.0, px(2.2), -d_outer)

            # Base gradient + subtle glow.
            col = mix(theme.bg_top, theme.bg_bottom, fy)
            glow_dist = math.hypot(fx - px(384), fy - px(244))
            glow = 1.0 - smoothstep(px(56), px(460), glow_dist)
            col = overlay(col, theme.bg_glow, glow * 0.24)

            # Inner panel.
            d_panel = sd_round_rect(fx, fy, 0.5, 0.5, panel_half_w, panel_half_h, panel_radius)
            if d_panel <= 0.0:
                panel_alpha = theme.panel_alpha * smoothstep(px(2.4), 0.0, d_panel)
                col = overlay(col, theme.panel, panel_alpha)

            # Divider.
            d_div = sd_round_rect(fx, fy, 0.5, 0.5, divider_half_w, panel_half_h - px(8), px(6))
            if d_div <= 0.0:
                div_alpha = theme.divider_alpha * smoothstep(px(1.7), 0.0, d_div)
                col = overlay(col, theme.divider, div_alpha)

            # Diff rows.
            for i, ry in enumerate(row_ys):
                # Left (minus)
                lw = (left_x1 - left_x0) * left_lens[i]
                lc = left_x0 + lw * 0.5
                d_left = sd_round_rect(fx, fy, lc, ry, lw * 0.5, row_h * 0.5, row_round)
                if d_left <= 0.0:
                    t = i / (len(row_ys) - 1)
                    row_col = mix(theme.minus_main, theme.minus_soft, t)
                    a = 0.96 * smoothstep(px(1.5), 0.0, d_left)
                    col = overlay(col, row_col, a)

                # Right (plus)
                rw = (right_x1 - right_x0) * right_lens[i]
                rc = right_x0 + rw * 0.5
                d_right = sd_round_rect(fx, fy, rc, ry, rw * 0.5, row_h * 0.5, row_round)
                if d_right <= 0.0:
                    t = i / (len(row_ys) - 1)
                    row_col = mix(theme.plus_main, theme.plus_soft, t)
                    a = 0.96 * smoothstep(px(1.5), 0.0, d_right)
                    col = overlay(col, row_col, a)

            # Git branch polyline glow and stroke.
            min_seg = 1e9
            for a, b in zip(branch_pts, branch_pts[1:]):
                min_seg = min(min_seg, dist_segment(fx, fy, a[0], a[1], b[0], b[1]))

            glow_a = 0.18 * (1.0 - smoothstep(px(9), px(24), min_seg))
            if glow_a > 0.0:
                col = overlay(col, theme.branch_glow, glow_a)

            line_a = 0.96 * (1.0 - smoothstep(px(3.2), px(6.4), min_seg))
            if line_a > 0.0:
                col = overlay(col, theme.branch, line_a)

            # Branch nodes.
            for nx, ny in branch_pts:
                d = math.hypot(fx - nx, fy - ny)
                core = 1.0 - smoothstep(node_r * 0.55, node_r * 0.95, d)
                if core > 0.0:
                    col = overlay(col, theme.branch, core * 0.96)

            # Icon border.
            border_alpha = theme.border_alpha * (1.0 - smoothstep(px(0.8), px(4.2), -d_outer))
            if border_alpha > 0.0:
                col = overlay(col, theme.border, border_alpha)

            r = clamp_u8(col[0])
            g = clamp_u8(col[1])
            b = clamp_u8(col[2])
            a = clamp_u8(255 * edge_soft)
            data.extend((r, g, b, a))

    return bytes(data)


def png_chunk(tag: bytes, data: bytes) -> bytes:
    return (
        struct.pack(">I", len(data))
        + tag
        + data
        + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
    )


def write_png(path: Path, raw_scanlines: bytes) -> None:
    ihdr = struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0)
    compressed = zlib.compress(raw_scanlines, level=9)
    png = bytearray(b"\x89PNG\r\n\x1a\n")
    png.extend(png_chunk(b"IHDR", ihdr))
    png.extend(png_chunk(b"IDAT", compressed))
    png.extend(png_chunk(b"IEND", b""))
    path.write_bytes(png)


def generate_all(out_dir: Path) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)

    variants = {
        "hunk-icon-default.png": DEFAULT_THEME,
        "hunk-icon-dark.png": DARK_THEME,
        "hunk-icon-mono.png": MONO_THEME,
    }

    for name, theme in variants.items():
        print(f"Generating {name}...")
        raw = render_icon(theme)
        write_png(out_dir / name, raw)


def main() -> None:
    out_dir = Path("assets/icons")
    generate_all(out_dir)
    print("Done.")


if __name__ == "__main__":
    main()
