// SPDX-License-Identifier: GPL-3.0-only

import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = resolve(fileURLToPath(new URL("..", import.meta.url)));
const sourcePath = resolve(projectRoot, "assets", "resona-icon.svg");
const checkOnly = process.argv.includes("--check");

const sourcePalette = {
  body: "#6f00d9",
  highlight: "#a000ff",
  structure: "#440080",
  metal: "#e2c6ff",
};

const formats = [
  {
    id: "mp3",
    label: "MP3",
    colorName: "blue",
    palette: {
      body: "#2979ff",
      highlight: "#82b1ff",
      structure: "#0d47a1",
      metal: "#e3f2fd",
    },
    labelStart: 8.796,
    glyphs: [
      [
        0,
        "M641 400H639L541 150H361L263 400H261V0H73V730H281L450 300H452L621 730H833V0H641Z",
      ],
      [
        881,
        "M268 228V0H73V725Q176 740 293 740Q633 740 633 495Q633 362 556.5 293.5Q480 225 333 225Q294 225 268 228ZM268 373Q287 370 313 370Q378 370 413 400Q448 430 448 485Q448 590 313 590Q291 590 268 585Z",
      ],
      [
        1512,
        "M390 430H400Q482 430 533.5 375.5Q585 321 585 230Q585 113 513 51.5Q441 -10 305 -10Q171 -10 55 50L104 200Q209 148 285 148Q336 148 363 168.5Q390 189 390 225Q390 255 376 270Q362 285 323.5 292.5Q285 300 210 300H170V430L340 568V570H70V730H575V570L390 432Z",
      ],
    ],
  },
  {
    id: "wav",
    label: "WAV",
    colorName: "yellow",
    palette: {
      body: "#ffab00",
      highlight: "#ffd740",
      structure: "#9a5a00",
      metal: "#fff3c4",
    },
    labelStart: 7.144,
    glyphs: [
      [
        0,
        "M494 530H492L418 0H188L33 730H235L309 200H311L375 730H615L679 200H681L755 730H953L798 0H568Z",
      ],
      [
        961,
        "M300 290H432L367 560H365ZM264 140 231 0H23L263 730H473L713 0H501L467 140Z",
      ],
      [1672, "M368 190 505 730H713L473 0H263L23 730H235L366 190Z"],
    ],
  },
  {
    id: "flac",
    label: "FLAC",
    colorName: "violet",
    palette: sourcePalette,
    labelStart: 6.367,
    glyphs: [
      [0, "M523 430V280H273V0H73V730H543V570H273V430Z"],
      [551, "M278 730V165H573V0H73V730Z"],
      [
        1122,
        "M300 290H432L367 560H365ZM264 140 231 0H23L263 730H473L713 0H501L467 140Z",
      ],
      [
        1808,
        "M418 -10Q241 -10 134.5 90.5Q28 191 28 365Q28 541 130.5 640.5Q233 740 418 740Q559 740 658 670L603 525Q520 580 438 580Q341 580 287 524.5Q233 469 233 365Q233 265 289.5 207.5Q346 150 438 150Q520 150 603 205L658 60Q559 -10 418 -10Z",
      ],
    ],
  },
];

function replacePalette(source, palette) {
  let result = source;
  for (const role of Object.keys(sourcePalette)) {
    const from = sourcePalette[role];
    const to = palette[role];
    if (!result.includes(from)) {
      throw new Error(`Main icon no longer contains the ${role} color ${from}`);
    }
    result = result.replaceAll(from, to);
  }
  return result;
}

function labelPaths(format) {
  const paths = format.glyphs
    .map(
      ([offset, path]) =>
        `    <path${offset === 0 ? "" : ` transform="translate(${offset})"`} d="${path}"/>`,
    )
    .join("\n");
  return `  <g transform="translate(${format.labelStart} 39.5) scale(0.014 -0.014)" fill="#fafafa" aria-hidden="true">\n${paths}\n  </g>`;
}

function renderFileIcon(mainIcon, format) {
  const geometry = mainIcon.match(
    /  <g transform="translate\(-6\.3158 -6\.3158\) scale\(1\.2632\)">\r?\n([\s\S]*?)\r?\n  <\/g>/,
  )?.[1];
  if (!geometry) {
    throw new Error("Could not locate the main icon geometry group");
  }

  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48" role="img" aria-labelledby="title desc">
  <title id="title">Resona ${format.label} file icon</title>
  <desc id="desc">The ${format.colorName} Resona turntable with a dark format band labelled ${format.label}.</desc>
  <g transform="translate(-6.3158 -6.3158) scale(1.2632)">
${replacePalette(geometry, format.palette)}
  <path d="M5 26h38v8a9 9 0 0 1-9 9H14a9 9 0 0 1-9-9z" fill="#212121"/>
${labelPaths(format)}
  </g>
</svg>
`;
}

const mainIcon = readFileSync(sourcePath, "utf8");
for (const format of formats) {
  const outputPath = resolve(
    projectRoot,
    "assets",
    `resona-file-${format.id}.svg`,
  );
  const generated = renderFileIcon(mainIcon, format);
  if (checkOnly) {
    if (readFileSync(outputPath, "utf8").replaceAll("\r\n", "\n") !== generated) {
      throw new Error(
        `resona-file-${format.id}.svg is not reproducible from resona-icon.svg`,
      );
    }
  } else {
    writeFileSync(outputPath, generated);
  }
  process.stdout.write(`resona-file-${format.id}.svg\n`);
}

if (checkOnly) {
  process.stdout.write("File icon SVG sources are reproducible.\n");
}
