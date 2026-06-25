import assert from 'node:assert/strict';
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';

import { processSocialAssets } from '../social-assets.mjs';

const MANIFEST = [{ name: 'example', width: 1200 }];

const svg = `
<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630" viewBox="0 0 1200 630">
  <rect width="1200" height="630" fill="#07111f"/>
</svg>`;

async function createFixture(svgContent = svg) {
  const root = await mkdtemp(join(tmpdir(), 'eternalmac-social-assets-'));
  await mkdir(join(root, 'assets/readme'), { recursive: true });
  await mkdir(join(root, 'website/public'), { recursive: true });
  await writeFile(join(root, 'assets/readme/example.svg'), svgContent.trim());
  await writeFile(
    join(root, 'assets/readme/ALT_TEXT.md'),
    '# Social asset alt text\n\n## example.png\n\nA dark example social card.\n',
  );
  return root;
}

test('write mode renders a source PNG and matching public copy', async () => {
  const root = await createFixture();

  await processSocialAssets({ root, manifest: MANIFEST, mode: 'write' });

  const source = await readFile(join(root, 'assets/readme/example.png'));
  const publicCopy = await readFile(
    join(root, 'website/public/socialAssets/example.png'),
  );
  assert.equal(source.subarray(1, 4).toString('ascii'), 'PNG');
  assert.deepEqual(publicCopy, source);
});

test('write mode renders text with bundled fonts', async () => {
  const textSvg = `
<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630" viewBox="0 0 1200 630">
  <rect width="1200" height="630" fill="#07111f"/>
  <text x="120" y="330" fill="#ffffff" font-family="Inter" font-size="112" font-weight="700">Eternal Mac</text>
</svg>`;
  const blankRoot = await createFixture();
  const textRoot = await createFixture(textSvg);

  await processSocialAssets({ root: blankRoot, manifest: MANIFEST, mode: 'write' });
  await processSocialAssets({ root: textRoot, manifest: MANIFEST, mode: 'write' });

  const blank = await readFile(join(blankRoot, 'assets/readme/example.png'));
  const text = await readFile(join(textRoot, 'assets/readme/example.png'));

  assert.notDeepEqual(text, blank);
  assert.ok(text.length > blank.length);
});

test('write mode reports missing source SVGs with a stable label', async () => {
  const root = await createFixture();
  await rm(join(root, 'assets/readme/example.svg'));

  await assert.rejects(
    processSocialAssets({ root, manifest: MANIFEST, mode: 'write' }),
    (error) => {
      assert.equal(error.message, 'example.svg is missing');
      return true;
    },
  );
});

test('check mode rejects a stale committed PNG without rewriting it', async () => {
  const root = await createFixture();
  const sourcePath = join(root, 'assets/readme/example.png');

  await processSocialAssets({ root, manifest: MANIFEST, mode: 'write' });
  const original = await readFile(sourcePath);
  const stale = Buffer.from(original);
  stale[stale.length - 1] ^= 1;
  await writeFile(sourcePath, stale);

  await assert.rejects(
    processSocialAssets({ root, manifest: MANIFEST, mode: 'check' }),
    /example\.png is stale; run npm --prefix website run assets:write/,
  );
  assert.deepEqual(await readFile(sourcePath), stale);
});

test('check mode requires non-empty alt text for every declared asset', async () => {
  const root = await createFixture();
  await processSocialAssets({ root, manifest: MANIFEST, mode: 'write' });
  await writeFile(join(root, 'assets/readme/ALT_TEXT.md'), '## example.png\n\n');

  await assert.rejects(
    processSocialAssets({ root, manifest: MANIFEST, mode: 'check' }),
    /ALT_TEXT.md has no alt text for example.png/,
  );
});
