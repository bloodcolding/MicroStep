"""一次性生成 src-tauri/icons 最小可行图标（stdlib zlib/struct，无第三方依赖）。"""
import os
import struct
import zlib


def make_png(size, palette_fn):
    def chunk(tag, data):
        body = tag + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))

    rows = []
    for y in range(size):
        row = b"\x00"
        for x in range(size):
            row += bytes(palette_fn(x, y, size))
        rows.append(row)
    raw = b"".join(rows)
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )


INDIGO = (63, 81, 181, 255)
LIGHT = (141, 155, 243, 255)


def pixel(x, y, size):
    lo, hi = size * 3 // 8, size * 5 // 8
    return LIGHT if lo <= x < hi and lo <= y < hi else INDIGO


def make_ico(png32):
    # ICO 容器直接内嵌 PNG（Vista+ 支持），header + directory entry + blob。
    header = struct.pack("<HHH", 0, 1, 1)
    entry = struct.pack("<BBBBHHII", 32, 32, 0, 0, 1, 32, len(png32), 6 + 16)
    return header + entry + png32


def main():
    out = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "src-tauri", "icons"))
    os.makedirs(out, exist_ok=True)
    png32 = make_png(32, pixel)
    png128 = make_png(128, pixel)
    open(os.path.join(out, "32x32.png"), "wb").write(png32)
    open(os.path.join(out, "128x128.png"), "wb").write(png128)
    open(os.path.join(out, "icon.ico"), "wb").write(make_ico(png32))
    print(f"icons -> {out}")


if __name__ == "__main__":
    main()
