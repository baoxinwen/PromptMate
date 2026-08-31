<script setup lang="ts">
import { inject, onMounted, ref } from 'vue';
import { Monitor, Moon, Sun, FolderOpen, Keyboard, Palette, SlidersHorizontal } from 'lucide-vue-next';
import { api } from '../lib/api';
import { managerKey } from '../lib/context';
import HotkeyInput from './HotkeyInput.vue';

const ctx = inject(managerKey)!;

const hotkey = ref('');
const captureHotkey = ref('');
const restoreClipboard = ref(true);
const pasteAppendEnter = ref(false);
const captureClipboard = ref(true);
const theme = ref('dark');
const autostart = ref(false);

async function load() {
  const s = ctx.data.value?.settings;
  if (s) {
    hotkey.value = s.hotkey;
    captureHotkey.value = s.captureHotkey;
    restoreClipboard.value = s.restoreClipboard;
    pasteAppendEnter.value = s.pasteAppendEnter;
    captureClipboard.value = s.captureClipboard;
    theme.value = ['light', 'auto'].includes(s.theme) ? s.theme : 'dark';
  }
  try {
    autostart.value = await api.getAutostart();
  } catch {
    /* 忽略 */
  }
}

async function saveSettingsPart(mutate: (s: import('../types').Settings) => void, okMsg: string) {
  const s = ctx.data.value?.settings;
  if (!s) return;
  try {
    const next = JSON.parse(JSON.stringify(s)) as import('../types').Settings;
    mutate(next);
    await api.saveSettings(next);
    await ctx.refresh();
    ctx.toast(okMsg);
  } catch (e) {
    ctx.toast(String(e), 'err');
  }
}

function onHotkeyChange(v: string) {
  hotkey.value = v;
  saveSettingsPart((s) => (s.hotkey = v), '主快捷键已更新');
}

function onCaptureHotkeyChange(v: string) {
  captureHotkey.value = v;
  saveSettingsPart((s) => (s.captureHotkey = v), '捕获快捷键已更新');
}

function onThemeChange(v: string) {
  theme.value = v;
  saveSettingsPart((s) => (s.theme = v), '主题已更新');
}

function onRestoreClipboardChange() {
  const next = !restoreClipboard.value;
  restoreClipboard.value = next;
  saveSettingsPart(
    (s) => (s.restoreClipboard = next),
    next ? '已开启粘贴后恢复剪贴板' : '已关闭粘贴后恢复剪贴板',
  );
}

function onPasteAppendEnterChange() {
  const next = !pasteAppendEnter.value;
  pasteAppendEnter.value = next;
  saveSettingsPart(
    (s) => (s.pasteAppendEnter = next),
    next ? '已开启粘贴后自动回车' : '已关闭粘贴后自动回车',
  );
}

function onCaptureClipboardChange() {
  const next = !captureClipboard.value;
  captureClipboard.value = next;
  saveSettingsPart((s) => (s.captureClipboard = next), next ? '已开启剪贴板记录' : '已关闭剪贴板记录');
}

async function toggleAutostart() {
  try {
    await api.setAutostart(!autostart.value);
    autostart.value = !autostart.value;
    ctx.toast(autostart.value ? '已开启开机自启' : '已关闭开机自启');
  } catch (e) {
    ctx.toast(String(e), 'err');
  }
}

const themes = [
  { id: 'dark', label: '暗色', icon: Moon, cls: 'th-dark' },
  { id: 'light', label: '亮色', icon: Sun, cls: 'th-light' },
  { id: 'auto', label: '跟随系统', icon: Monitor, cls: 'th-auto' },
] as const;

onMounted(load);
</script>

<template>
  <div class="st">
    <header class="st-head">
      <h2 class="st-title">设置</h2>
      <span class="grow" />
    </header>

    <div class="st-body">
      <div class="st-card">
        <div class="sec-head">
          <span class="sec-ico"><Keyboard :size="14" /></span>
          <h3>快捷键</h3>
        </div>
        <label class="opt col">
          <span>
            呼出快捷面板
            <small class="faint">在任何应用中按下即可呼出面板，Esc 或再按一次隐藏</small>
          </span>
          <HotkeyInput :model-value="hotkey" @update:model-value="onHotkeyChange" />
        </label>
        <label class="opt col">
          <span>
            快速捕获
            <small class="faint">选中文本后按下，直接把选中内容保存为新提示词</small>
          </span>
          <HotkeyInput :model-value="captureHotkey" @update:model-value="onCaptureHotkeyChange" />
        </label>
      </div>

      <div class="st-card">
        <div class="sec-head">
          <span class="sec-ico"><Palette :size="14" /></span>
          <h3>外观</h3>
        </div>
        <div class="theme-cards">
          <button
            v-for="t in themes"
            :key="t.id"
            class="theme-card"
            :class="[{ on: theme === t.id }, t.cls]"
            @click="onThemeChange(t.id)"
          >
            <div class="th-preview"><span class="th-bar" /><span class="th-line" /><span class="th-line short" /></div>
            <span class="th-label"><component :is="t.icon" :size="13" /> {{ t.label }}</span>
          </button>
        </div>
      </div>

      <div class="st-card">
        <div class="sec-head">
          <span class="sec-ico"><SlidersHorizontal :size="14" /></span>
          <h3>通用</h3>
        </div>
        <label class="opt">
          <span>
            粘贴后恢复原剪贴板
            <small class="faint">自动粘贴完成后，把你之前复制的内容悄悄放回剪贴板</small>
          </span>
          <span class="switch">
            <input type="checkbox" :checked="restoreClipboard" @change="onRestoreClipboardChange" />
            <span class="track"><span class="thumb" /></span>
          </span>
        </label>
        <label class="opt">
          <span>
            粘贴后自动回车
            <small class="faint">粘贴到网页 AI 对话框后自动发送；代码编辑器场景建议关闭</small>
          </span>
          <span class="switch">
            <input type="checkbox" :checked="pasteAppendEnter" @change="onPasteAppendEnterChange" />
            <span class="track"><span class="thumb" /></span>
          </span>
        </label>
        <label class="opt">
          <span>
            开机自动启动
            <small class="faint">登录 Windows 后在后台运行（托盘图标）</small>
          </span>
          <span class="switch">
            <input type="checkbox" :checked="autostart" @change="toggleAutostart" />
            <span class="track"><span class="thumb" /></span>
          </span>
        </label>
        <label class="opt">
          <span>
            记录剪贴板历史
            <small class="faint">自动记录系统中新复制的文本与图片</small>
          </span>
          <span class="switch">
            <input type="checkbox" :checked="captureClipboard" @change="onCaptureClipboardChange" />
            <span class="track"><span class="thumb" /></span>
          </span>
        </label>
      </div>

      <div class="st-card">
        <div class="sec-head">
          <span class="sec-ico"><FolderOpen :size="14" /></span>
          <h3>数据</h3>
        </div>
        <label class="opt">
          <span>数据目录<small class="faint">data.json 保存了全部提示词与配置</small></span>
          <button @click="api.openDataDir()"><FolderOpen :size="13" /> 打开目录</button>
        </label>
      </div>

      <div class="about faint">
        <span class="mono">PromptMate</span> · <span class="tnum">v0.2.0</span> · 本地优先的提示词管理工具
      </div>
    </div>
  </div>
</template>

<style scoped>
.st {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.st-head {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 18px;
  height: 52px;
  flex: none;
  border-bottom: 1px solid var(--border);
}

.st-title {
  font-size: var(--fs-lg);
  font-weight: 650;
  letter-spacing: -0.01em;
}

.st-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 18px 22px;
  display: flex;
  flex-direction: column;
  gap: 14px;
  max-width: 680px;
  width: 100%;
}

.st-card {
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: var(--r-md);
  padding: 6px 18px 12px;
}

.sec-head {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 12px 0 10px;
}

.sec-ico {
  width: 26px;
  height: 26px;
  border-radius: 7px;
  display: grid;
  place-items: center;
  background: var(--brand-soft);
  color: var(--brand);
  flex: none;
}

h3 {
  font-size: 13.5px;
  font-weight: 650;
}

.opt {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 11px 0;
  border-top: 1px solid var(--border);
  cursor: pointer;
}

.st-card .opt:first-of-type {
  border-top: none;
}

.opt.col {
  flex-direction: column;
  align-items: flex-start;
  gap: 8px;
}

.opt span:first-child {
  display: flex;
  flex-direction: column;
  gap: 3px;
  font-size: 13px;
}

.opt small {
  font-size: 11.5px;
  font-weight: 400;
}

.theme-cards {
  display: flex;
  gap: 10px;
  padding: 4px 0 10px;
}

.theme-card {
  flex: 1;
  flex-direction: column;
  align-items: stretch;
  gap: 0;
  padding: 0;
  overflow: hidden;
  border: 1.5px solid var(--border);
  border-radius: var(--r-md);
}

.theme-card:hover {
  border-color: var(--border-strong);
  background: var(--panel);
}

.theme-card.on {
  border-color: var(--brand);
  box-shadow: 0 0 0 3px var(--brand-soft);
}

.th-preview {
  height: 64px;
  padding: 11px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.th-dark .th-preview {
  background: #0e0e12;
}

.th-light .th-preview {
  background: #f6f6f8;
}

.th-auto .th-preview {
  background: linear-gradient(105deg, #0e0e12 50%, #f6f6f8 50%);
}

.th-bar {
  width: 55%;
  height: 8px;
  border-radius: 4px;
  background: var(--brand-grad);
}

.th-line {
  width: 85%;
  height: 6px;
  border-radius: 3px;
  background: rgba(128, 128, 148, 0.35);
}

.th-line.short {
  width: 55%;
}

.th-label {
  display: flex;
  align-items: center;
  gap: 6px;
  justify-content: center;
  padding: 7px 0;
  font-size: 12px;
  color: var(--text-2);
  border-top: 1px solid var(--border);
  background: var(--panel-2);
}

.theme-card.on .th-label {
  color: var(--brand);
  font-weight: 600;
}

.about {
  font-size: 11.5px;
  padding: 2px 0 12px;
}
</style>
