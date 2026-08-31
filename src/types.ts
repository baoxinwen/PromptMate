export interface Prompt {
  id: string;
  title: string;
  content: string;
  category: string;
  tags: string[];
  pinned: boolean;
  /** 独立全局快捷键（如 "ctrl+alt+1"，空串表示未绑定） */
  hotkey: string;
  useCount: number;
  lastUsedAt: number;
  createdAt: number;
  updatedAt: number;
}

export interface ClipboardImageRef {
  file: string;
  width: number;
  height: number;
}

export interface ClipboardItem {
  id: string;
  content: string;
  copiedAt: number;
  /** 条目类型："text" | "image"（旧数据无此字段视为 text） */
  kind?: 'text' | 'image';
  /** 图片条目的文件引用（图片本体存本机磁盘，不参与云同步） */
  image?: ClipboardImageRef | null;
}

export interface WebDavConfig {
  enabled: boolean;
  autoSync: boolean;
  url: string;
  username: string;
  password: string;
}

export interface GistConfig {
  enabled: boolean;
  autoSync: boolean;
  /** GitHub Personal Access Token（需 gist 权限） */
  token: string;
  /** secret Gist id，留空则在首次同步时自动创建 */
  gistId: string;
}

export interface Settings {
  hotkey: string;
  /** 快速捕获快捷键（选中文本存为提示词） */
  captureHotkey: string;
  captureClipboard: boolean;
  /** 粘贴完成后恢复用户原来的剪贴板内容 */
  restoreClipboard: boolean;
  /** 粘贴后自动追加回车（适合网页 AI 对话框直接发送） */
  pasteAppendEnter: boolean;
  /** 界面主题："dark" | "light" | "auto" */
  theme: string;
  /** 一次性主题迁移标记（后端字段，保存设置时必须原样带回，否则会重复触发迁移） */
  themeMigrated?: boolean;
  /** 一次性快捷键迁移标记（后端字段，同上必须原样带回） */
  hotkeyMigrated?: boolean;
  webdav: WebDavConfig;
  gist: GistConfig;
  /** 当前使用的同步后端："webdav" | "gist" */
  syncProvider: string;
  /** 云同步是否包含文本剪贴板历史（默认关：避免密码等敏感复制内容上云） */
  syncClipboard: boolean;
}

export interface AppData {
  version: number;
  settings: Settings;
  categories: string[];
  prompts: Prompt[];
  clipboard: ClipboardItem[];
  /** 已删除条目的墓碑（云同步删除传播用），前端仅透传 */
  tombstones?: { id: string; at: number }[];
  seeded: boolean;
}

export interface SyncReport {
  added: number;
  updated: number;
  removed: number;
  message: string;
}

export interface ImportSummary {
  added: number;
  skipped: number;
  message: string;
}

export interface VarField {
  name: string;
  hint: string;
}

export function emptyPrompt(category = ''): Prompt {
  const now = Date.now();
  return {
    id: '',
    title: '',
    content: '',
    category,
    tags: [],
    pinned: false,
    hotkey: '',
    useCount: 0,
    lastUsedAt: 0,
    createdAt: now,
    updatedAt: now,
  };
}
