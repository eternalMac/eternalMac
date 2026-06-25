import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

import { processSocialAssets } from './social-assets.mjs';

const flags = new Set(process.argv.slice(2));
const supported = ['--write', '--sync-public', '--check'];
const selected = supported.filter((flag) => flags.has(flag));

if (selected.length !== 1 || flags.size !== 1) {
  console.error('Usage: node scripts/render-social-assets.mjs --write|--sync-public|--check');
  process.exitCode = 1;
} else {
  const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
  const mode = selected[0].slice(2);
  try {
    await processSocialAssets({ root, mode });
    console.log(`Social assets ${mode} completed.`);
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
