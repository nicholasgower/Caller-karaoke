// Build assets/icons/caller-karaoke.ico from the PNG sizes beside it.
//
// The ICO entries are stored as PNG images, which Windows has read since
// Vista. Run it after regenerating the PNGs: `node scripts/make-icon-ico.mjs`.

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const icons = join(root, "assets", "icons");
const sizes = [16, 32, 48, 64, 128, 256];

const images = sizes.map((size) => ({
  size,
  bytes: readFileSync(join(icons, `caller-karaoke-${size}.png`)),
}));

// ICONDIR (6 bytes) then one ICONDIRENTRY (16 bytes) per image, then the
// image bytes. A size of 256 is stored as 0 in the directory.
const header = Buffer.alloc(6 + images.length * 16);
header.writeUInt16LE(0, 0); // reserved
header.writeUInt16LE(1, 2); // type: icon
header.writeUInt16LE(images.length, 4);

let offset = header.length;
images.forEach((image, index) => {
  const entry = 6 + index * 16;
  header.writeUInt8(image.size === 256 ? 0 : image.size, entry);
  header.writeUInt8(image.size === 256 ? 0 : image.size, entry + 1);
  header.writeUInt8(0, entry + 2); // palette colours
  header.writeUInt8(0, entry + 3); // reserved
  header.writeUInt16LE(1, entry + 4); // colour planes
  header.writeUInt16LE(32, entry + 6); // bits per pixel
  header.writeUInt32LE(image.bytes.length, entry + 8);
  header.writeUInt32LE(offset, entry + 12);
  offset += image.bytes.length;
});

const ico = Buffer.concat([header, ...images.map((image) => image.bytes)]);
writeFileSync(join(icons, "caller-karaoke.ico"), ico);
console.log(`wrote caller-karaoke.ico (${sizes.join(", ")} px)`);
