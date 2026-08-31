import type { VarField } from '../types';

const VAR_RE = /\{\{\s*([^{}|]+?)\s*(?:\|([^{}]*))?\}\}/g;

/** 提取提示词中的 {{变量}}，按出现顺序去重 */
export function extractVars(content: string): VarField[] {
  const seen = new Map<string, VarField>();
  let m: RegExpExecArray | null;
  VAR_RE.lastIndex = 0;
  while ((m = VAR_RE.exec(content)) !== null) {
    const name = m[1].trim();
    if (!name || seen.has(name)) continue;
    seen.set(name, { name, hint: (m[2] ?? '').trim() });
  }
  return [...seen.values()];
}

export function hasVars(content: string): boolean {
  return extractVars(content).length > 0;
}

/** 内置自动变量：调用时自动填充，不出现在填写表单 */
export const AUTO_VARS = new Set(['clipboard']);

export function isAutoVar(name: string): boolean {
  return AUTO_VARS.has(name.trim().toLowerCase());
}

/** 是否存在需要手动填写的变量（排除 {{clipboard}} 等自动变量） */
export function hasManualVars(content: string): boolean {
  return extractVars(content).some((v) => !isAutoVar(v.name));
}

/** 用填写的值替换 {{变量}}；未填写的变量替换为空串 */
export function applyVars(content: string, values: Record<string, string>): string {
  return content.replace(VAR_RE, (_match, rawName: string) => {
    const name = String(rawName).trim();
    return values[name] ?? '';
  });
}
