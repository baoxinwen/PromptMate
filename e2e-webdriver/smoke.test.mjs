/**
 * tauri-driver 全栈真机冒烟测试（node --test 运行）
 *
 * 链路：真实 Rust 应用二进制 + WebView2 + 真实 IPC，无任何 mock。
 * 连接方式（Windows/WebView2 的正确姿势）：
 *   1. 我们自己启动应用（PROMPTMATE_E2E=1 使应用以 additional_browser_args
 *      显式开启 --remote-debugging-port=9222，环境变量会被 wry 的默认参数覆盖，
 *      必须由应用侧给出）；
 *   2. 启动 tauri-driver（--native-driver 指向版本匹配的 msedgedriver）；
 *   3. WebDriver 会话通过 ms:edgeOptions.debuggerAddress 附加到运行中的 WebView2。
 *
 * 前置条件（`pnpm test:e2e:fullstack` 前需完成一次）：
 *   1. cargo install tauri-driver --locked
 *   2. tools/msedgedriver/msedgedriver.exe（版本需匹配本机 WebView2 Runtime）
 *   3. pnpm run build:debug  （生成 src-tauri/target/debug/promptmate.exe）
 */
import test, { before, after } from 'node:test';
import assert from 'node:assert/strict';
import { spawn, execSync } from 'node:child_process';
import { createConnection } from 'node:net';
import { mkdtempSync, existsSync, mkdirSync, writeFileSync, rmSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Builder, By, until } from 'selenium-webdriver';

const root = resolve(fileURLToPath(new URL('..', import.meta.url)));
const APP = join(root, 'src-tauri', 'target', 'debug', 'promptmate.exe');
const TAURI_DRIVER = join(process.env.USERPROFILE ?? '', '.cargo', 'bin', 'tauri-driver.exe');
const NATIVE_DRIVER = join(root, 'tools', 'msedgedriver', 'msedgedriver.exe');
const DRIVER_PORT = 4444;
const DEBUG_PORT = 9222;
const ARTIFACTS = join(root, 'e2e-artifacts');

function waitForPort(host, port, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  return new Promise((resolvePromise, rejectPromise) => {
    const tryOnce = () => {
      const sock = createConnection({ host, port }, () => {
        sock.destroy();
        resolvePromise();
      });
      sock.on('error', () => {
        sock.destroy();
        if (Date.now() > deadline) rejectPromise(new Error(`端口 ${host}:${port} 等待超时`));
        else setTimeout(tryOnce, 300);
      });
    };
    tryOnce();
  });
}

/** 只清理 target/debug 下本次构建的应用实例，不动用户正常安装的应用 */
function killDebugApp() {
  try {
    execSync(
      `powershell -NoProfile -Command "Get-Process promptmate -ErrorAction SilentlyContinue | ` +
        `Where-Object { $_.Path -like '*target*debug*' } | Stop-Process -Force"`,
      { stdio: 'ignore' },
    );
  } catch {
    /* 没有残留进程时静默 */
  }
}

let dataDir;
let appProc;
let driverProc;
let driver;

before(async () => {
  for (const [label, f] of [
    ['应用二进制', APP],
    ['tauri-driver', TAURI_DRIVER],
    ['msedgedriver', NATIVE_DRIVER],
  ]) {
    if (!existsSync(f)) {
      throw new Error(
        `缺少${label}: ${f}\n请先执行 cargo install tauri-driver --locked / pnpm run build:debug / 下载匹配版 msedgedriver`,
      );
    }
  }
  mkdirSync(ARTIFACTS, { recursive: true });
  killDebugApp();
  // 防陈旧二进制假绿：应用必须内嵌 E2E 调试端口参数，否则构建不完整
  const embedded = readFileSync(APP).includes('remote-debugging-port=9222');
  assert(embedded, '应用二进制缺少 E2E 调试端口参数（remote-debugging-port=9222），构建不完整，请重新执行 pnpm run build:debug');
  dataDir = mkdtempSync(join(tmpdir(), 'promptmate-e2e-'));

  // 1. 启动真实应用（E2E 模式：可见主窗口 + 固定调试端口 + 隔离数据目录）
  appProc = spawn(APP, [], {
    env: { ...process.env, PROMPTMATE_E2E: '1', PROMPTMATE_DATA_DIR: dataDir },
    stdio: 'ignore',
    detached: false,
  });
  await waitForPort('127.0.0.1', DEBUG_PORT, 30_000);

  // 2. 启动 tauri-driver（内部代理到 msedgedriver）
  driverProc = spawn(TAURI_DRIVER, ['--native-driver', NATIVE_DRIVER], {
    env: { ...process.env },
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  driverProc.stderr.on('data', (d) => process.stderr.write(`[tauri-driver] ${d}`));
  await waitForPort('127.0.0.1', DRIVER_PORT, 20_000);
});

after(async () => {
  await driver?.quit().catch(() => {});
  driver = undefined;
  driverProc?.kill();
  appProc?.kill();
  killDebugApp();
  try {
    if (dataDir) rmSync(dataDir, { recursive: true, force: true });
  } catch {
    /* WebView2 释放文件可能有延迟 */
  }
});

test('真机全栈：附加到运行中的应用，真实 IPC 已走通', async () => {
  driver = await new Builder()
    .usingServer(`http://127.0.0.1:${DRIVER_PORT}/`)
    .withCapabilities({
      browserName: 'msedge',
      'ms:edgeOptions': { args: [], debuggerAddress: `127.0.0.1:${DEBUG_PORT}` },
    })
    .build();
  // 前端 DOM 就绪 = 真实 IPC（get_data）已走通
  await driver.wait(until.elementLocated(By.css('input.qp-search')), 30_000);
  assert(
    existsSync(join(dataDir, 'data.json')),
    `应用数据应落在隔离目录 ${dataDir}，而非用户真实数据目录`,
  );
});

test('真机全栈：拼音首字母搜索命中首启种子数据', async () => {
  const search = await driver.findElement(By.css('input.qp-search'));
  await search.sendKeys('dmsc');
  await driver.wait(until.elementsLocated(By.css('.qp-list .item')), 10_000);
  const titles = await driver.findElements(By.css('.item-title'));
  assert.ok(titles.length >= 1, '搜索结果不应为空');
  const text = await titles[0].getText();
  assert.match(text, /代码审查/, `拼音 dmsc 应命中「代码审查」，实际: ${text}`);

  const shot = await driver.takeScreenshot();
  writeFileSync(join(ARTIFACTS, 'fullstack-smoke.png'), shot, 'base64');
});
