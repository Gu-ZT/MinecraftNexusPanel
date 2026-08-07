import { chmodSync, cpSync, existsSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const packageRequire = createRequire(import.meta.url);
const repositoryRoot = resolve(import.meta.dirname, '..');
const desktopBinaryDirectory = resolve(repositoryRoot, 'apps/desktop/src-tauri/binaries');
const desktopWebDirectory = resolve(repositoryRoot, 'apps/desktop/src-tauri/web');
const frontendBuildDirectory = resolve(repositoryRoot, 'frontend/app/dist');
const releaseProfile = process.env.TAURI_ENV_DEBUG === 'true' ? 'debug' : 'release';
const targetTriple = process.env.TAURI_ENV_TARGET_TRIPLE ?? hostTargetTriple();
const executableName = process.platform === 'win32' ? 'mcnp.exe' : 'mcnp';
const cargoTargetRoot = process.env.CARGO_TARGET_DIR
  ? resolve(repositoryRoot, process.env.CARGO_TARGET_DIR)
  : resolve(repositoryRoot, 'target');
const cargoTargetDirectory = resolve(cargoTargetRoot, targetTriple, releaseProfile);
const sourceBinary = resolve(cargoTargetDirectory, executableName);
const packagedBinary = resolve(desktopBinaryDirectory, executableName);

buildFrontend();
packageFrontend();

const cargoArguments = ['build', '-p', 'mcnp', '--target', targetTriple];
if (releaseProfile === 'release') {
  cargoArguments.splice(1, 0, '--release');
}
run('cargo', cargoArguments);

mkdirSync(desktopBinaryDirectory, { recursive: true });
rmSync(packagedBinary, { force: true });
cpSync(sourceBinary, packagedBinary);
if (process.platform !== 'win32') {
  chmodSync(packagedBinary, 0o755);
}

function hostTargetTriple() {
  const result = spawnSync('rustc', ['-vV'], { cwd: repositoryRoot, encoding: 'utf8' });
  if (result.status !== 0) {
    throw new Error(result.stderr || 'Unable to determine the Rust host target');
  }
  const match = result.stdout.match(/^host:\s*(\S+)$/m);
  if (!match) {
    throw new Error('Rust did not report a host target');
  }
  return match[1];
}

/**
 * 优先使用已经安装在工作区内的前端工具，避免 Tauri 构建再次触发 Corepack。
 * CI 在依赖安装完成后同样走这条路径；没有本地依赖的源码环境才回退到 pnpm。
 */
function buildFrontend() {
  const vueTypecheck = resolvePackageBinary('vue-tsc', 'bin/vue-tsc.js');
  const vite = resolvePackageBinary('vite', 'bin/vite.js');

  if (vueTypecheck && vite) {
    const frontendDirectory = resolve(repositoryRoot, 'frontend', 'app');
    run(process.execPath, [vueTypecheck, '--noEmit', '-p', 'tsconfig.json'], frontendDirectory);
    run(process.execPath, [vite, 'build'], frontendDirectory);
    return;
  }

  run(
    process.platform === 'win32' ? 'pnpm.cmd' : 'pnpm',
    ['--dir', 'frontend/app', 'build'],
    repositoryRoot,
  );
}

/** 将共享 WebUI 放入 Tauri 资源目录，供发布版 sidecar 在动态 Panel 端口同源托管。 */
function packageFrontend() {
  const entry = resolve(frontendBuildDirectory, 'index.html');
  if (!existsSync(entry)) {
    throw new Error(`Frontend build output is missing: ${entry}`);
  }
  rmSync(desktopWebDirectory, { force: true, recursive: true });
  cpSync(frontendBuildDirectory, desktopWebDirectory, { recursive: true });
  writeFileSync(resolve(desktopWebDirectory, '.gitkeep'), '');
}

function resolvePackageBinary(packageName, relativePath) {
  try {
    const packageDirectory = dirname(packageRequire.resolve(`${packageName}/package.json`));
    const binary = resolve(packageDirectory, relativePath);
    return existsSync(binary) ? binary : null;
  } catch {
    return null;
  }
}

function run(command, arguments_, cwd = repositoryRoot) {
  const result = spawnSync(command, arguments_, {
    cwd,
    stdio: 'inherit',
    shell: process.platform === 'win32' && command.endsWith('.cmd'),
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}
