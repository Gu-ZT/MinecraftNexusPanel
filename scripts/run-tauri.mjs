import { existsSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';

const repositoryRoot = resolve(import.meta.dirname, '..');
const desktopProjectDirectory = resolve(repositoryRoot, 'apps/desktop');
const frontendBuildDirectory = resolve(repositoryRoot, 'frontend/app/dist');
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

const childEnvironment = { ...process.env };
if (projectDirectory === desktopProjectDirectory && tauriArguments[0] === 'dev') {
  childEnvironment.MCNP_PANEL_WEB_ROOT = prepareDesktopDevelopmentWebUi();
  childEnvironment.MCNP_DESKTOP_DEV_SIDECAR_PATH = prepareDesktopDevelopmentSidecar();
}

const result = spawnSync(process.execPath, [tauriCli, ...tauriArguments], {
  cwd: projectDirectory,
  env: childEnvironment,
  stdio: 'inherit',
});

if (result.error) {
  throw result.error;
}
process.exit(result.status ?? 1);

/**
 * 浏览器从托盘地址访问 Panel 时必须与 API 保持同源；开发态因此先生成一份静态 WebUI。
 * Tauri 主窗口仍由 beforeDevCommand 启动的 Vite 提供热更新，两条入口共享同一套源码。
 */
function prepareDesktopDevelopmentWebUi() {
  const command = process.platform === 'win32' ? 'pnpm.cmd' : 'pnpm';
  const result = spawnSync(command, ['--dir', 'frontend/app', 'build'], {
    cwd: repositoryRoot,
    stdio: 'inherit',
    shell: process.platform === 'win32',
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }

  const entry = resolve(frontendBuildDirectory, 'index.html');
  if (!existsSync(entry)) {
    throw new Error(`Desktop development WebUI was not produced: ${entry}`);
  }
  return frontendBuildDirectory;
}

/**
 * Desktop 开发态不执行 beforeBuildCommand，因此必须在启动 Tauri 前刷新本地 sidecar。
 * 显式路径可避免运行时误用 target/debug 中由旧提交遗留的同名可执行文件。
 */
function prepareDesktopDevelopmentSidecar() {
  const result = spawnSync('cargo', ['build', '-p', 'mcnp', '--locked'], {
    cwd: repositoryRoot,
    stdio: 'inherit',
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }

  const cargoTargetRoot = process.env.CARGO_TARGET_DIR
    ? resolve(repositoryRoot, process.env.CARGO_TARGET_DIR)
    : resolve(repositoryRoot, 'target');
  const executableName = process.platform === 'win32' ? 'mcnp.exe' : 'mcnp';
  const sidecarPath = resolve(cargoTargetRoot, 'debug', executableName);
  if (!existsSync(sidecarPath)) {
    throw new Error(`Desktop development sidecar was not produced: ${sidecarPath}`);
  }
  return sidecarPath;
}
