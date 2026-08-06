import { createReadStream } from 'node:fs';
import { readdir, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { basename, join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const DEFAULT_MANIFEST_NAME = 'SHA256SUMS.txt';

/**
 * 为目录中的发布文件生成按文件名排序的 SHA-256 清单。
 * 清单自身和子目录不会参与计算，避免重复执行时把旧清单哈希进去。
 */
export async function createChecksumManifest(directory, manifestName = DEFAULT_MANIFEST_NAME) {
  const absoluteDirectory = resolve(directory);
  const entries = await readdir(absoluteDirectory, { withFileTypes: true });
  const fileNames = entries
    .filter((entry) => entry.isFile() && entry.name !== manifestName)
    .map((entry) => entry.name)
    .sort((left, right) => left.localeCompare(right, 'en'));
  if (fileNames.length === 0) {
    throw new Error(`No release files found in ${absoluteDirectory}`);
  }

  const lines = [];
  for (const fileName of fileNames) {
    const digest = await sha256File(join(absoluteDirectory, fileName));
    lines.push(`${digest}  ${fileName}`);
  }

  const manifestPath = join(absoluteDirectory, manifestName);
  await writeFile(manifestPath, `${lines.join('\n')}\n`, 'utf8');
  return manifestPath;
}

async function sha256File(path) {
  const hash = createHash('sha256');
  for await (const chunk of createReadStream(path)) {
    hash.update(chunk);
  }
  return hash.digest('hex');
}

async function main() {
  const directory = process.argv[2];
  if (!directory) {
    throw new Error('Usage: node scripts/create-sha256.mjs <artifact-directory>');
  }
  const manifestPath = await createChecksumManifest(directory);
  process.stdout.write(`${basename(manifestPath)} created in ${resolve(directory)}\n`);
}

const entryPoint = process.argv[1] ? pathToFileURL(resolve(process.argv[1])).href : '';
if (import.meta.url === entryPoint) {
  await main();
}
