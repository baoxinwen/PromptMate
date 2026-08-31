<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { Zap, X } from 'lucide-vue-next';
import { api } from '../lib/api';
import { emptyPrompt } from '../types';
import type { AppData } from '../types';
import AccentButton from './ui/AccentButton.vue';

const data = ref<AppData | null>(null);
const title = ref('');
const content = ref('');
const category = ref('未分类');
const saving = ref(false);
const saveErr = ref('');
const titleInput = ref<HTMLInputElement | null>(null);

async function load() {
  try {
    data.value = await api.getData();
  } catch {
    /* 忽略 */
  }
}

function reset(text: string) {
  content.value = text;
  const firstLine = text.split('\n')[0]?.trim() ?? '';
  title.value = firstLine.slice(0, 20);
  if (!category.value) category.value = data.value?.categories[0] ?? '未分类';
}

async function save() {
  if (saving.value) return;
  if (!content.value.trim()) {
    closeWindow();
    return;
  }
  saving.value = true;
  saveErr.value = '';
  try {
    const cat = category.value || data.value?.categories[0] || '未分类';
    await api.savePrompt({
      ...emptyPrompt(cat),
      title: title.value.trim() || content.value.trim().slice(0, 20),
      content: content.value,
    });
    closeWindow();
  } catch (e) {
    // 窗口保持打开，把失败显式告诉用户，避免误以为已保存
    saveErr.value = String(e);
  } finally {
    saving.value = false;
  }
}

function closeWindow() {
  api.closeCapture();
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    e.preventDefault();
    closeWindow();
  } else if ((e.ctrlKey || e.metaKey) && (e.key === 's' || e.key === 'Enter')) {
    e.preventDefault();
    save();
  }
}

let unlistenText: (() => void) | undefined;
let unlistenData: (() => void) | undefined;

onMounted(async () => {
  await load();
  titleInput.value?.focus();
  unlistenText = await listen<string>('capture-text', (e) => {
    reset(e.payload ?? '');
    titleInput.value?.focus();
  });
  unlistenData = await listen('data-changed', load);
});
onBeforeUnmount(() => {
  unlistenText?.();
  unlistenData?.();
});
</script>

<template>
  <div class="cv" @keydown="onKeydown">
    <div class="cv-head">
      <span class="cv-logo"><Zap :size="12" :stroke-width="2.4" /></span>
      <span class="cv-title">快速捕获</span>
      <span class="grow" />
      <button class="icon-x" title="关闭" aria-label="关闭快速捕获" @click="closeWindow()">
        <X :size="14" />
      </button>
    </div>
    <input
      ref="titleInput"
      v-model="title"
      class="cv-title-input"
      type="text"
      placeholder="标题（默认取首行）"
      spellcheck="false"
    />
    <textarea
      v-model="content"
      class="cv-content mono"
      placeholder="选中文本会自动填到这里，也可手动输入"
      spellcheck="false"
    />
    <div class="cv-foot">
      <select v-model="category">
        <option v-for="c in data?.categories ?? []" :key="c" :value="c">{{ c }}</option>
        <option value="未分类">未分类</option>
      </select>
      <span v-if="saveErr" class="cv-err">{{ saveErr }}</span>
      <span class="grow" />
      <button class="ghost-btn" @click="closeWindow()">取消</button>
      <AccentButton @click="save">保存 <kbd>Ctrl S</kbd></AccentButton>
    </div>
  </div>
</template>

<style scoped>
.cv {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: var(--panel);
  border: 1px solid var(--border-strong);
  border-radius: var(--r-lg);
  overflow: hidden;
  gap: 9px;
}

.cv-head {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 11px 12px 0;
}

.cv-logo {
  width: 22px;
  height: 22px;
  border-radius: 7px;
  display: grid;
  place-items: center;
  background: var(--brand-grad);
  color: #fff;
  flex: none;
}

.cv-title {
  font-weight: 650;
  font-size: 13px;
}

.grow {
  flex: 1;
}

.icon-x {
  width: 26px;
  height: 26px;
  padding: 0;
  border-color: transparent;
  background: transparent;
  color: var(--muted);
}

.cv-title-input {
  margin: 0 12px;
  font-size: 13.5px;
  padding: 7px 10px;
}

.cv-content {
  flex: 1;
  min-height: 0;
  resize: none;
  font-size: 12px;
  line-height: 1.6;
  white-space: pre-wrap;
  margin: 0 12px;
  background: var(--bg-soft);
}

.cv-foot {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 12px 12px;
}

.cv-foot .ab kbd {
  margin-left: 7px;
  background: rgba(255, 255, 255, 0.16);
  border-color: rgba(255, 255, 255, 0.28);
  box-shadow: none;
  color: #fff;
  font-size: 9.5px;
  padding: 2px 5px;
}

select {
  font-size: 12px;
  padding: 5px 8px;
}

.ghost-btn {
  background: transparent;
  font-size: 12px;
  color: var(--muted);
}

.cv-err {
  font-size: 11px;
  color: var(--danger);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ghost-btn:hover {
  color: var(--text);
}
</style>
