import { existsSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';

const repositoryRoot = resolve(import.meta.dirname, '..');
const [projectDirectoryArgument, ...tauriArguments] = process.argv.slice(2);

if (!projectDirectoryArgument || tauriArguments.length === 0) {
  throw new Error('Usage: node scripts/run-tauri.mjs <project-directory> <tauri-command> [...args]');
}

const projectDirectory = resolve(repositoryRoot, projectDirectoryArgument);
const tauriConfig = resolve(projectDirectory, 'src-tauri', 'tauri.conf.json');
const tauriCli = resolve(repositoryRoot, 'node_modules', '@tauri-apps', 'cli', 'tauri.js');

if (!existsSync(tauriConfig)) {
  throw new Error(`Tauri configuration not found: ${tauriConfig}`);
}
if (!existsSync(tauriCli)) {
  throw new Error('Tauri CLI is not installed; run pnpm install from the repository root');
}

const result = spawnSync(process.execPath, [tauriCli, ...tauriArguments], {
  cwd: projectDirectory,
  stdio: 'inherit',
});

if (result.error) {
  throw result.error;
}
process.exit(result.status ?? 1);
