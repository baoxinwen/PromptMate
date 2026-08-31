<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, provide, ref } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { FileText, ClipboardList, Cloud, HardDriveDownload, Settings, Zap } from 'lucide-vue-next';
import { api } from '../lib/api';
import { managerKey } from '../lib/context';
import type { AppData } from '../types';
import ConfirmDialog from '../components/ui/ConfirmDialog.vue';
import type { ToastAction } from '../lib/context';
import PromptsPane from '../components/PromptsPane.vue';
import ClipboardPane from '../components/ClipboardPane.vue';
import SyncPane from '../components/SyncPane.vue';
import DataPane from '../components/DataPane.vue';
import SettingsPane from '../components/SettingsPane.vue';

const data = ref<AppData | null>(null);
const tab = ref<'prompts' | 'clipboard' | 'sync' | 'data' | 'settings'>('prompts');
const toastMsg = ref('');
const toastKind = ref<'ok' | 'err'>('ok');
const toastAction = ref<ToastAction | null>(null);
let toastTimer: ReturnType<typeof setTimeout> | undefined;

const confirmOpen = ref(false);
const confirmOpts = ref<{ title: string; message?: string; confirmText?: string; danger?: boolean }>({
  title: '',
});
let confirmResolve: ((v: boolean) => void) | undefined;

function confirm(options: {
  title: string;
  message?: string;
  confirmText?: string;
  danger?: boolean;
}): Promise<boolean> {
  confirmOpts.value = options;
  confirmOpen.value = true;
  return new Promise((resolve) => (confirmResolve = resolve));
}

function settleConfirm(v: boolean) {
  confirmOpen.value = false;
  confirmResolve?.(v);
  confirmResolve = undefined;
}

const syncLabel = computed(() => {
  const s = data.value?.settings;
  if (!s) return '未配置';
  if (s.syncProvider === 'gist') return s.gist.enabled ? 'GitHub 同步已启用' : 'GitHub 未启用';
  return s.webdav.enabled ? 'WebDAV 已启用' : '云同步未启用';
});
const syncOn = computed(() => {
  const s = data.value?.settings;
  if (!s) return false;
  return s.syncProvider === 'gist' ? s.gist.enabled : s.webdav.enabled;
});

async function refresh() {
  try {
    data.value = await api.getData();
  } catch (e) {
    toast(String(e), 'err');
  }
}

function toast(msg: string, kind: 'ok' | 'err' = 'ok', action?: ToastAction, ms?: number) {
  toastMsg.value = msg;
  toastKind.value = kind;
  toastAction.value = action ?? null;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(
    () => {
      toastMsg.value = '';
      toastAction.value = null;
    },
    ms ?? (action ? 5000 : 2200),
  );
}

provide(managerKey, { data, refresh, toast, confirm });

let unlisten: (() => void) | undefined;
let unlistenSync: (() => void) | undefined;

/** Ctrl+K 聚焦搜索：当前页没有搜索框时先切回提示词页 */
function onKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
    e.preventDefault();
    if (tab.value !== 'prompts' && tab.value !== 'clipboard') {
      tab.value = 'prompts';
      nextTick(() => window.dispatchEvent(new CustomEvent('pm-focus-search')));
    } else {
      window.dispatchEvent(new CustomEvent('pm-focus-search'));
    }
  }
}

/** 自动同步失败的去重节流：同一错误 60s 内只提示一次，避免每个轮询周期刷屏 */
let lastAutoSyncErr = '';
let lastAutoSyncErrAt = 0;

onMounted(async () => {
  document.addEventListener('keydown', onKeydown);
  await refresh();
  // 数据文件损坏的恢复提示：挂载后主动拉取（后端取后即清）。
  // 不用事件：窗口创建初期事件先于监听注册，必然丢失
  try {
    const notice = await api.getRecoveryNotice();
    if (notice) toast(notice, 'err', undefined, 8000);
  } catch {
    /* 忽略 */
  }
  unlisten = await listen('data-changed', refresh);
  // 云同步结果反馈（自动同步失败 / 托盘手动同步成败）
  unlistenSync = await listen<{ message: string; ok?: boolean; auto?: boolean }>(
    'sync-done',
    (e) => {
      const p = e.payload;
      if (!p?.message) return;
      if (p.ok === false) {
        if (p.auto) {
          const now = Date.now();
          if (p.message === lastAutoSyncErr && now - lastAutoSyncErrAt < 60000) return;
          lastAutoSyncErr = p.message;
          lastAutoSyncErrAt = now;
        }
        toast(`云同步失败：${p.message}`, 'err', undefined, 5000);
      } else {
        lastAutoSyncErr = '';
        toast(p.message);
      }
    },
  );
});
onBeforeUnmount(() => {
  document.removeEventListener('keydown', onKeydown);
  unlisten?.();
  unlistenSync?.();
});

const tabs = [
  { id: 'prompts', label: '提示词', icon: FileText },
  { id: 'clipboard', label: '剪贴板', icon: ClipboardList },
  { id: 'sync', label: '云同步', icon: Cloud },
  { id: 'data', label: '数据', icon: HardDriveDownload },
  { id: 'settings', label: '设置', icon: Settings },
] as const;
</script>

<template>
  <div class="mg">
    <!-- 图标栏 -->
    <aside class="mg-side">
      <div class="brand-mark" title="PromptMate">
        <Zap :size="17" :stroke-width="2.4" />
      </div>

      <nav class="mg-nav">
        <button
          v-for="t in tabs"
          :key="t.id"
          class="nav-btn"
          :class="{ on: tab === t.id }"
          :title="t.label"
          :aria-label="t.label"
          @click="tab = t.id"
        >
          <component :is="t.icon" :size="18" :stroke-width="1.9" />
        </button>
      </nav>

      <div class="side-foot">
        <span class="sync-dot" :class="{ on: syncOn }" :title="syncLabel" />
      </div>
    </aside>

    <!-- 内容区 -->
    <main class="mg-main">
      <PromptsPane v-if="tab === 'prompts'" />
      <ClipboardPane v-else-if="tab === 'clipboard'" />
      <SyncPane v-else-if="tab === 'sync'" />
      <DataPane v-else-if="tab === 'data'" />
      <SettingsPane v-else />
    </main>

    <div v-if="toastMsg" class="toast" :class="toastKind" aria-live="polite">
      {{ toastMsg }}
      <button
        v-if="toastAction"
        class="toast-action"
        @click="
          () => {
            const a = toastAction;
            toastMsg = '';
            toastAction = null;
            a?.handler();
          }
        "
      >
        {{ toastAction.label }}
      </button>
    </div>

    <ConfirmDialog
      :open="confirmOpen"
      :title="confirmOpts.title"
      :message="confirmOpts.message"
      :confirm-text="confirmOpts.confirmText"
      :danger="confirmOpts.danger"
      @confirm="settleConfirm(true)"
      @cancel="settleConfirm(false)"
    />
  </div>
</template>

<style scoped>
.mg {
  display: flex;
  height: 100vh;
  background: var(--bg);
}

/* 图标栏：品牌标 + 图标导航 + 同步状态灯 */
.mg-side {
  width: 64px;
  flex: none;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 12px 0 14px;
  border-right: 1px solid var(--border);
  background: var(--bg-soft);
  gap: 6px;
}

.brand-mark {
  width: 34px;
  height: 34px;
  border-radius: 10px;
  display: grid;
  place-items: center;
  background: var(--brand-grad);
  color: #fff;
  box-shadow: var(--shadow-brand);
  margin-bottom: 14px;
  flex: none;
}

.mg-nav {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.nav-btn {
  position: relative;
  width: 40px;
  height: 40px;
  padding: 0;
  border: none;
  background: transparent;
  border-radius: 11px;
  color: var(--muted);
}

.nav-btn:hover {
  background: var(--panel-2);
  color: var(--text);
}

.nav-btn.on {
  background: var(--brand-soft);
  color: var(--brand);
}

.nav-btn.on::before {
  content: '';
  position: absolute;
  left: -12px;
  top: 9px;
  bottom: 9px;
  width: 3px;
  border-radius: 3px;
  background: var(--brand);
}

.side-foot {
  margin-top: auto;
  padding-top: 8px;
}

.sync-dot {
  display: block;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--faint);
  cursor: default;
  transition: background var(--t-med), box-shadow var(--t-med);
}

.sync-dot.on {
  background: var(--ok);
  box-shadow: 0 0 8px var(--ok);
}

.mg-main {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

/* toast 靠右下，避开编辑器底部操作区 */
.mg .toast {
  left: auto;
  right: 18px;
  transform: none;
  bottom: 18px;
}

@starting-style {
  .mg .toast {
    opacity: 0;
    transform: translateY(8px);
  }
}

.toast-action {
  margin-left: 10px;
  padding: 2px 12px;
  font-size: 12px;
  font-weight: 600;
  border-radius: 999px;
  background: var(--brand-soft);
  border-color: transparent;
  color: var(--brand);
}
</style>
