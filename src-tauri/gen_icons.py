import struct
import zlib
import os

ICONS_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "icons")
os.makedirs(ICONS_DIR, exist_ok=True)


def create_png(width, height, filepath):
    def make_chunk(chunk_type, data):
        chunk = chunk_type + data
        return struct.pack(">I", len(data)) + chunk + struct.pack(">I", zlib.crc32(chunk) & 0xFFFFFFFF)

    signature = b"\x89PNG\r\n\x1a\n"

    ihdr_data = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)
    ihdr = make_chunk(b"IHDR", ihdr_data)

    raw_data = b""
    for y in range(height):
        raw_data += b"\x00"
        raw_data += b"\x1a\x3c\x5e" * width

    compressed = zlib.compress(raw_data)
    idat = make_chunk(b"IDAT", compressed)

    iend = make_chunk(b"IEND", b"")

    with open(filepath, "wb") as f:
        f.write(signature + ihdr + idat + iend)


def create_ico(filepath):
    png_32 = os.path.join(ICONS_DIR, "32x32.png")
    png_16_path = os.path.join(ICONS_DIR, "_tmp_16.png")
    create_png(16, 16, png_16_path)

    with open(png_32, "rb") as f:
        png32_data = f.read()
    with open(png_16_path, "rb") as f:
        png16_data = f.read()

    ico_header = struct.pack("<HHH", 0, 1, 2)

    offset = 6 + 2 * 16

    entry_16 = struct.pack("<BBBBHHII", 16, 16, 0, 0, 1, 32, len(png16_data), offset)
    entry_32 = struct.pack("<BBBBHHII", 32, 32, 0, 0, 1, 32, len(png32_data), offset + len(png16_data))

    with open(filepath, "wb") as f:
        f.write(ico_header + entry_16 + entry_32 + png16_data + png32_data)

    os.remove(png_16_path)


def create_icns(filepath):
    png_128 = os.path.join(ICONS_DIR, "128x128.png")
    with open(png_128, "rb") as f:
        png_data = f.read()

    icon_data = b"ic08" + struct.pack(">I", len(png_data) + 8) + png_data

    file_size = 4 + 4 + len(icon_data)
    icns_header = b"icns" + struct.pack(">I", file_size)

    with open(filepath, "wb") as f:
        f.write(icns_header + icon_data)


create_png(32, 32, os.path.join(ICONS_DIR, "32x32.png"))
create_png(128, 128, os.path.join(ICONS_DIR, "128x128.png"))
create_png(256, 256, os.path.join(ICONS_DIR, "128x128@2x.png"))
create_ico(os.path.join(ICONS_DIR, "icon.ico"))
create_icns(os.path.join(ICONS_DIR, "icon.icns"))

print("Icons generated successfully:")
for f in sorted(os.listdir(ICONS_DIR)):
    fp = os.path.join(ICONS_DIR, f)
    print(f"  {f} ({os.path.getsize(fp)} bytes)")
