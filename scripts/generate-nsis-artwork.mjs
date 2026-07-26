// SPDX-License-Identifier: GPL-3.0-only

import {
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { inflateSync } from "node:zlib";

const projectRoot = resolve(fileURLToPath(new URL("..", import.meta.url)));
const tauriCli = resolve(projectRoot, "node_modules", "@tauri-apps", "cli", "tauri.js");
const outputRoot = resolve(projectRoot, "src-tauri", "windows", "nsis");
const checkOnly = process.argv.includes("--check");

const colors = {
  ink: [16, 9, 26],
  violetDark: [44, 13, 73],
  violet: [111, 0, 217],
  violetLight: [160, 0, 255],
  lavender: [226, 198, 255],
  focus: [255, 64, 129],
};

function renderSvg(source, staging, name) {
  const path = join(staging, `${name}.svg`);
  const output = join(staging, name);
  writeFileSync(path, source);
  const result = spawnSync(
    process.execPath,
    [tauriCli, "icon", path, "--output", output, "--png", "1024"],
    { cwd: projectRoot, stdio: "inherit" },
  );
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`Could not rasterize ${name}.svg`);
  return decodePng(readFileSync(join(output, "1024x1024.png")));
}

function canonicalIconSvg() {
  return readFileSync(resolve(projectRoot, "assets", "resona-icon.svg"), "utf8");
}

function canonicalWordmarkSquareSvg() {
  const source = readFileSync(
    resolve(projectRoot, "assets", "resona-resonance-wordmark.svg"),
    "utf8",
  );
  const inner = source
    .replace(/^<svg[^>]*>\s*/u, "")
    .replace(/\s*<\/svg>\s*$/u, "");
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 900 900">
  <svg x="0" y="360" width="900" height="180" viewBox="130 40 640 180">
${inner}
  </svg>
</svg>
`;
}

function decodePng(buffer) {
  const signature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  if (!buffer.subarray(0, 8).equals(signature)) throw new Error("Invalid PNG signature");

  let width = 0;
  let height = 0;
  let bitDepth = 0;
  let colorType = 0;
  const dataChunks = [];
  for (let offset = 8; offset < buffer.length;) {
    const length = buffer.readUInt32BE(offset);
    const type = buffer.toString("ascii", offset + 4, offset + 8);
    const data = buffer.subarray(offset + 8, offset + 8 + length);
    if (type === "IHDR") {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      bitDepth = data[8];
      colorType = data[9];
      if (data[12] !== 0) throw new Error("Interlaced PNG is not supported");
    } else if (type === "IDAT") {
      dataChunks.push(data);
    } else if (type === "IEND") {
      break;
    }
    offset += 12 + length;
  }
  if (bitDepth !== 8 || colorType !== 6) {
    throw new Error(`Expected an 8-bit RGBA PNG, received depth ${bitDepth}, type ${colorType}`);
  }

  const bytesPerPixel = 4;
  const stride = width * bytesPerPixel;
  const inflated = inflateSync(Buffer.concat(dataChunks));
  const pixels = Buffer.alloc(width * height * bytesPerPixel);
  let sourceOffset = 0;
  for (let y = 0; y < height; y += 1) {
    const filter = inflated[sourceOffset];
    sourceOffset += 1;
    const rowOffset = y * stride;
    for (let x = 0; x < stride; x += 1) {
      const raw = inflated[sourceOffset + x];
      const left = x >= bytesPerPixel ? pixels[rowOffset + x - bytesPerPixel] : 0;
      const up = y > 0 ? pixels[rowOffset + x - stride] : 0;
      const upLeft = y > 0 && x >= bytesPerPixel
        ? pixels[rowOffset + x - stride - bytesPerPixel]
        : 0;
      let value;
      if (filter === 0) value = raw;
      else if (filter === 1) value = raw + left;
      else if (filter === 2) value = raw + up;
      else if (filter === 3) value = raw + Math.floor((left + up) / 2);
      else if (filter === 4) value = raw + paeth(left, up, upLeft);
      else throw new Error(`Unsupported PNG filter ${filter}`);
      pixels[rowOffset + x] = value & 0xff;
    }
    sourceOffset += stride;
  }
  return { width, height, pixels };
}

function paeth(left, up, upLeft) {
  const estimate = left + up - upLeft;
  const leftDistance = Math.abs(estimate - left);
  const upDistance = Math.abs(estimate - up);
  const cornerDistance = Math.abs(estimate - upLeft);
  if (leftDistance <= upDistance && leftDistance <= cornerDistance) return left;
  return upDistance <= cornerDistance ? up : upLeft;
}

function cropTransparent(image) {
  let left = image.width;
  let top = image.height;
  let right = -1;
  let bottom = -1;
  for (let y = 0; y < image.height; y += 1) {
    for (let x = 0; x < image.width; x += 1) {
      if (image.pixels[(y * image.width + x) * 4 + 3] === 0) continue;
      left = Math.min(left, x);
      top = Math.min(top, y);
      right = Math.max(right, x);
      bottom = Math.max(bottom, y);
    }
  }
  if (right < left || bottom < top) throw new Error("Rasterized SVG is empty");
  return { ...image, crop: { left, top, right, bottom } };
}

function createCanvas(width, height) {
  return { width, height, pixels: Buffer.alloc(width * height * 3) };
}

function setPixel(canvas, x, y, color) {
  if (x < 0 || y < 0 || x >= canvas.width || y >= canvas.height) return;
  const offset = (y * canvas.width + x) * 3;
  canvas.pixels[offset] = Math.round(color[0]);
  canvas.pixels[offset + 1] = Math.round(color[1]);
  canvas.pixels[offset + 2] = Math.round(color[2]);
}

function getPixel(canvas, x, y) {
  const offset = (y * canvas.width + x) * 3;
  return [
    canvas.pixels[offset],
    canvas.pixels[offset + 1],
    canvas.pixels[offset + 2],
  ];
}

function blendPixel(canvas, x, y, color, alpha) {
  if (alpha <= 0 || x < 0 || y < 0 || x >= canvas.width || y >= canvas.height) return;
  const background = getPixel(canvas, x, y);
  setPixel(canvas, x, y, [
    color[0] * alpha + background[0] * (1 - alpha),
    color[1] * alpha + background[1] * (1 - alpha),
    color[2] * alpha + background[2] * (1 - alpha),
  ]);
}

function fillGradient(canvas, top, bottom, horizontalAccent = 0) {
  for (let y = 0; y < canvas.height; y += 1) {
    const vertical = canvas.height === 1 ? 0 : y / (canvas.height - 1);
    for (let x = 0; x < canvas.width; x += 1) {
      const horizontal = canvas.width === 1 ? 0 : x / (canvas.width - 1);
      const accent = horizontal * horizontalAccent;
      setPixel(canvas, x, y, [
        top[0] * (1 - vertical) + bottom[0] * vertical + accent,
        top[1] * (1 - vertical) + bottom[1] * vertical,
        top[2] * (1 - vertical) + bottom[2] * vertical + accent * 1.4,
      ]);
    }
  }
}

function drawImage(canvas, image, left, top, width, height) {
  const crop = image.crop ?? {
    left: 0,
    top: 0,
    right: image.width - 1,
    bottom: image.height - 1,
  };
  const cropWidth = crop.right - crop.left + 1;
  const cropHeight = crop.bottom - crop.top + 1;
  for (let y = 0; y < height; y += 1) {
    const sourceY = crop.top + Math.min(
      cropHeight - 1,
      Math.floor(((y + 0.5) * cropHeight) / height),
    );
    for (let x = 0; x < width; x += 1) {
      const sourceX = crop.left + Math.min(
        cropWidth - 1,
        Math.floor(((x + 0.5) * cropWidth) / width),
      );
      const sourceOffset = (sourceY * image.width + sourceX) * 4;
      const alpha = image.pixels[sourceOffset + 3] / 255;
      blendPixel(
        canvas,
        left + x,
        top + y,
        [
          image.pixels[sourceOffset],
          image.pixels[sourceOffset + 1],
          image.pixels[sourceOffset + 2],
        ],
        alpha,
      );
    }
  }
}

function drawRing(canvas, centerX, centerY, radius, color, alpha) {
  const inner = Math.max(0, radius - 0.7);
  const outer = radius + 0.7;
  const minX = Math.floor(centerX - outer);
  const maxX = Math.ceil(centerX + outer);
  const minY = Math.floor(centerY - outer);
  const maxY = Math.ceil(centerY + outer);
  for (let y = minY; y <= maxY; y += 1) {
    for (let x = minX; x <= maxX; x += 1) {
      const distance = Math.hypot(x + 0.5 - centerX, y + 0.5 - centerY);
      if (distance >= inner && distance <= outer) blendPixel(canvas, x, y, color, alpha);
    }
  }
}

function encodeBmp(canvas) {
  const rowStride = Math.ceil((canvas.width * 3) / 4) * 4;
  const pixelBytes = rowStride * canvas.height;
  const output = Buffer.alloc(54 + pixelBytes);
  output.write("BM", 0, "ascii");
  output.writeUInt32LE(output.length, 2);
  output.writeUInt32LE(54, 10);
  output.writeUInt32LE(40, 14);
  output.writeInt32LE(canvas.width, 18);
  output.writeInt32LE(canvas.height, 22);
  output.writeUInt16LE(1, 26);
  output.writeUInt16LE(24, 28);
  output.writeUInt32LE(pixelBytes, 34);
  output.writeInt32LE(3780, 38);
  output.writeInt32LE(3780, 42);

  for (let y = 0; y < canvas.height; y += 1) {
    const sourceY = canvas.height - 1 - y;
    const rowOffset = 54 + y * rowStride;
    for (let x = 0; x < canvas.width; x += 1) {
      const sourceOffset = (sourceY * canvas.width + x) * 3;
      const targetOffset = rowOffset + x * 3;
      output[targetOffset] = canvas.pixels[sourceOffset + 2];
      output[targetOffset + 1] = canvas.pixels[sourceOffset + 1];
      output[targetOffset + 2] = canvas.pixels[sourceOffset];
    }
  }
  return output;
}

function headerArtwork(icon, wordmark) {
  const canvas = createCanvas(150, 57);
  fillGradient(canvas, colors.ink, colors.violetDark, 4);
  drawImage(canvas, icon, 10, 8, 41, 41);
  drawImage(canvas, wordmark, 59, 18, 81, 23);
  return encodeBmp(canvas);
}

function sidebarArtwork(icon, wordmark) {
  const canvas = createCanvas(164, 314);
  fillGradient(canvas, colors.ink, colors.violetDark, 8);
  for (const radius of [24, 40, 56, 72]) {
    drawRing(canvas, 82, 276, radius, colors.lavender, 0.18);
  }
  drawRing(canvas, 82, 276, 8, colors.focus, 0.72);
  drawImage(canvas, icon, 34, 34, 96, 96);
  drawImage(canvas, wordmark, 18, 157, 128, 36);
  return encodeBmp(canvas);
}

function writeOrCheck(name, contents) {
  const output = resolve(outputRoot, name);
  if (checkOnly) {
    if (!readFileSync(output).equals(contents)) {
      throw new Error(`${name} is not reproducible from the canonical Resona SVG assets`);
    }
  } else {
    writeFileSync(output, contents);
  }
  process.stdout.write(`${name} ${contents.length} bytes\n`);
}

const staging = mkdtempSync(join(tmpdir(), "resona-nsis-artwork-"));
try {
  const icon = cropTransparent(renderSvg(canonicalIconSvg(), staging, "icon"));
  const wordmark = cropTransparent(
    renderSvg(canonicalWordmarkSquareSvg(), staging, "wordmark"),
  );
  writeOrCheck("header.bmp", headerArtwork(icon, wordmark));
  writeOrCheck("sidebar.bmp", sidebarArtwork(icon, wordmark));
  if (checkOnly) process.stdout.write("NSIS artwork is reproducible.\n");
} finally {
  rmSync(staging, { recursive: true, force: true });
}
