import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { Resvg } from '@resvg/resvg-js';

export const DEFAULT_MANIFEST = [
  { name: 'architecture', width: 1200 },
  { name: 'terminal-proof', width: 1200 },
];

const PNG_SIGNATURE = [137, 80, 78, 71, 13, 10, 26, 10];
const MAX_BYTES = 8 * 1024 * 1024;
const SOURCE_DIR = 'assets/readme';
const PUBLIC_DIR = 'website/public/socialAssets';
const SCRIPT_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const SUPPORTED_MODES = new Set(['write', 'sync-public', 'check']);

function fontFiles() {
  return [
    join(
      SCRIPT_ROOT,
      'website/node_modules/@fontsource/inter/files/inter-latin-400-normal.woff',
    ),
    join(
      SCRIPT_ROOT,
      'website/node_modules/@fontsource/inter/files/inter-latin-700-normal.woff',
    ),
    join(
      SCRIPT_ROOT,
      'website/node_modules/@fontsource/inter/files/inter-latin-800-normal.woff',
    ),
    join(
      SCRIPT_ROOT,
      'website/node_modules/@fontsource/ibm-plex-mono/files/ibm-plex-mono-latin-400-normal.woff',
    ),
  ];
}

function altTextFor(document, name) {
  const heading = `## ${name}.png`;
  const start = document
    .split('\n')
    .findIndex((line) => line.trimEnd() === heading);

  if (start === -1) {
    return '';
  }

  const lines = document.split('\n');
  const next = lines.findIndex(
    (line, index) => index > start && line.startsWith('## '),
  );
  const end = next === -1 ? lines.length : next;

  return lines.slice(start + 1, end).join('\n').trim();
}

function validatePng(bytes, label, spec) {
  const png = Buffer.from(bytes);
  const hasSignature =
    png.length >= PNG_SIGNATURE.length &&
    PNG_SIGNATURE.every((byte, index) => png[index] === byte);

  if (!hasSignature || png.length < 20) {
    throw new Error(`${label} is not a PNG`);
  }

  const width = png.readUInt32BE(16);

  if (width !== spec.width) {
    throw new Error(`${label} is ${width}px wide; expected ${spec.width}px`);
  }

  if (png.length >= MAX_BYTES) {
    throw new Error(`${label} is ${png.length} bytes; it must be under ${MAX_BYTES}`);
  }
}

async function existing(path, label, options) {
  try {
    return await readFile(path, options);
  } catch (error) {
    if (error?.code === 'ENOENT') {
      throw new Error(`${label} is missing`);
    }
    throw error;
  }
}

async function render(svgPath, width, label = svgPath) {
  const svg = await existing(svgPath, label, 'utf8');
  const renderer = new Resvg(svg, {
    fitTo: { mode: 'width', value: width },
    font: {
      fontFiles: fontFiles(),
      loadSystemFonts: false,
      defaultFontFamily: 'Inter',
    },
  });

  return Buffer.from(renderer.render().asPng());
}

async function write(path, bytes) {
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, bytes);
}

export async function processSocialAssets({
  root,
  manifest = DEFAULT_MANIFEST,
  mode,
}) {
  if (!SUPPORTED_MODES.has(mode)) {
    throw new Error(`Unknown social asset mode: ${mode}`);
  }

  const repoRoot = resolve(root);
  const sourceDir = join(repoRoot, SOURCE_DIR);
  const publicDir = join(repoRoot, PUBLIC_DIR);
  const altTextPath = join(sourceDir, 'ALT_TEXT.md');
  const altTextDocument = String(await existing(altTextPath, 'ALT_TEXT.md'));
  const failures = [];

  for (const spec of manifest) {
    const label = `${spec.name}.png`;
    const svgLabel = `${spec.name}.svg`;
    const publicLabel = `website/public/socialAssets/${label}`;
    const svgPath = join(sourceDir, `${spec.name}.svg`);
    const sourcePath = join(sourceDir, label);
    const publicPath = join(publicDir, label);

    if (!altTextFor(altTextDocument, spec.name)) {
      failures.push(`ALT_TEXT.md has no alt text for ${label}`);
    }

    try {
      if (mode === 'write') {
        const rendered = await render(svgPath, spec.width, svgLabel);
        validatePng(rendered, label, spec);
        await write(sourcePath, rendered);
        await write(publicPath, rendered);
        continue;
      }

      const source = await existing(sourcePath, label);
      validatePng(source, label, spec);

      if (mode === 'sync-public') {
        await write(publicPath, source);
        continue;
      }

      const expected = await render(svgPath, spec.width, svgLabel);
      validatePng(expected, label, spec);

      if (!source.equals(expected)) {
        failures.push(
          `${label} is stale; run npm --prefix website run assets:write`,
        );
      }

      const publicCopy = await existing(publicPath, publicLabel);
      if (!publicCopy.equals(source)) {
        failures.push(
          `${publicLabel} is stale; run npm --prefix website run assets:write`,
        );
      }
    } catch (error) {
      failures.push(error.message);
    }
  }

  if (failures.length > 0) {
    throw new Error(failures.join('\n'));
  }
}
