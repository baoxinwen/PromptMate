/** 运行平台检测与快捷键展示的跨平台适配 */
export const isMac =
  typeof navigator !== 'undefined' && /Mac|iPhone|iPad|iPod/i.test(navigator.platform || navigator.userAgent);

/** 修饰键在当前平台的展示名（快捷键内部存储值保持 ctrl/alt/super/shift 不变） */
export const MOD_LABELS: Record<string, string> = isMac
  ? { ctrl: '⌃', alt: '⌥', shift: '⇧', super: '⌘', meta: '⌘' }
  : { ctrl: 'Ctrl', alt: 'Alt', shift: 'Shift', super: 'Win', meta: 'Win' };

/** 快捷键组合提示文案 */
export const hotkeyHint = isMac ? '需包含 Cmd / Option / Control 修饰键' : '需包含 Alt / Ctrl / Win 修饰键';

/** 单独的键位展示名 */
export function keyLabel(k: string): string {
  const key = k.trim().toLowerCase();
  if (MOD_LABELS[key]) return MOD_LABELS[key];
  return key.length === 1 ? key.toUpperCase() : key.charAt(0).toUpperCase() + key.slice(1);
}
