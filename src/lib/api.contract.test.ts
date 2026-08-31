import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, it, expect } from 'vitest';

/**
 * 前后端 IPC 契约测试：
 * api.ts 里出现的每一个 invoke 命令名，都必须已在 src-tauri 的
 * invoke_handler 注册表中登记，且在 commands.rs 有对应命令函数。
 * 纯静态文本比对，不依赖 Tauri 运行时。
 */

const apiSrc = readFileSync(fileURLToPath(new URL('./api.ts', import.meta.url)), 'utf8');
const libRs = readFileSync(
  fileURLToPath(new URL('../../src-tauri/src/lib.rs', import.meta.url)),
  'utf8',
);
const commandsRs = readFileSync(
  fileURLToPath(new URL('../../src-tauri/src/commands.rs', import.meta.url)),
  'utf8',
);

const apiCommands = [
  ...apiSrc.matchAll(/invoke(?:<[\s\S]*?>)?\(\s*'([a-z_]+)'/g),
].map((m) => m[1]);

const registered = [...libRs.matchAll(/commands::([a-z_0-9]+)/g)].map((m) => m[1]);

const commandFns = new Set(
  [...commandsRs.matchAll(/\bfn\s+([a-z_0-9]+)\s*\(/g)].map((m) => m[1]),
);

describe('IPC 契约：api.ts ↔ src-tauri 注册表', () => {
  it('api.ts 中确实解析出了命令清单（防正则失效导致空跑）', () => {
    expect(apiCommands.length).toBeGreaterThanOrEqual(30);
    expect(registered.length).toBeGreaterThanOrEqual(30);
  });

  it('api.ts 的命令名不得重复', () => {
    expect(new Set(apiCommands).size).toBe(apiCommands.length);
  });

  it('api.ts 调用的每个命令都必须已在 invoke_handler 注册', () => {
    const missing = apiCommands.filter((c) => !registered.includes(c));
    expect(missing, `未注册的命令: ${missing.join(', ')}`).toEqual([]);
  });

  it('api.ts 调用的每个命令在 commands.rs 都有对应的 fn', () => {
    const missing = apiCommands.filter((c) => !commandFns.has(c));
    expect(missing, `缺少命令函数: ${missing.join(', ')}`).toEqual([]);
  });

  it('invoke_handler 注册的每个命令在 commands.rs 都有对应的 fn', () => {
    const missing = [...new Set(registered)].filter((c) => !commandFns.has(c));
    expect(missing, `注册了但无实现: ${missing.join(', ')}`).toEqual([]);
  });
});
