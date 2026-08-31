import { invoke } from '@tauri-apps/api/core';
import type { AppData, ImportSummary, Prompt, Settings, SyncReport } from '../types';

export const api = {
  getData: () => invoke<AppData>('get_data'),
  getRecoveryNotice: () => invoke<string | null>('get_recovery_notice'),
  savePrompt: (prompt: Prompt) => invoke<void>('save_prompt', { prompt }),
  deletePrompt: (id: string) => invoke<void>('delete_prompt', { id }),
  recordUse: (id: string) => invoke<void>('record_prompt_use', { id }),

  addCategory: (name: string) => invoke<void>('add_category', { name }),
  renameCategory: (oldName: string, newName: string) =>
    invoke<void>('rename_category', { oldName, newName }),
  deleteCategory: (name: string) => invoke<void>('delete_category', { name }),

  copyText: (text: string) => invoke<void>('copy_text', { text }),
  invokePaste: (text: string, promptId?: string) =>
    invoke<void>('invoke_paste', { text, promptId: promptId ?? null }),
  pasteTextDirect: (text: string) => invoke<void>('paste_text_direct', { text }),

  deleteHistoryItem: (id: string) => invoke<void>('delete_history_item', { id }),
  clearHistory: () => invoke<void>('clear_history'),

  getImageThumb: (id: string) => invoke<string>('get_image_thumb', { id }),
  pasteImage: (id: string) => invoke<void>('paste_image', { id }),

  getClipboardText: () => invoke<string | null>('get_clipboard_text'),
  getVarMemory: (promptId: string) =>
    invoke<Record<string, string>>('get_var_memory', { promptId }),
  saveVarMemory: (promptId: string, values: Record<string, string>) =>
    invoke<void>('save_var_memory', { promptId, values }),

  hideQuick: () => invoke<void>('hide_quick'),
  openManager: () => invoke<void>('open_manager'),
  closeCapture: () => invoke<void>('close_capture'),
  setPanelHeight: (height: number) => invoke<void>('set_panel_height', { height }),

  saveSettings: (settings: Settings) => invoke<void>('save_settings', { settings }),
  getAutostart: () => invoke<boolean>('get_autostart'),
  setAutostart: (enable: boolean) => invoke<void>('set_autostart', { enable }),
  openDataDir: () => invoke<void>('open_data_dir'),

  exportData: (kind: 'json' | 'markdown', includeClipboard: boolean) =>
    invoke<string>('export_data', { kind, includeClipboard }),
  importData: () => invoke<ImportSummary>('import_data'),
  importPaths: (paths: string[]) => invoke<ImportSummary>('import_paths', { paths }),

  webdavTest: (url: string, username: string, password: string) =>
    invoke<string>('webdav_test', { url, username, password }),
  gistTest: (token: string, gistId: string) =>
    invoke<string>('gist_test', { token, gistId }),
  syncNow: (direction: 'merge' | 'push' | 'pull') =>
    invoke<SyncReport>('sync_now', { direction }),
};
