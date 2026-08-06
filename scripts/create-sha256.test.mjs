import assert from 'node:assert/strict';
import { mkdtemp, readFile, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';

import { createChecksumManifest } from './create-sha256.mjs';

test('creates a stable manifest and excludes the previous manifest', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'mcnp-checksum-'));
  await writeFile(join(directory, 'b.exe'), 'second');
  await writeFile(join(directory, 'a.exe'), 'first');

  const manifestPath = await createChecksumManifest(directory);
  const firstManifest = await readFile(manifestPath, 'utf8');
  await createChecksumManifest(directory);
  const secondManifest = await readFile(manifestPath, 'utf8');

  assert.equal(firstManifest, secondManifest);
  assert.match(firstManifest, /^[0-9a-f]{64}  a\.exe\n[0-9a-f]{64}  b\.exe\n$/);
});
