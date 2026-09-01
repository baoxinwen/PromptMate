<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { listen } from '@tauri-apps/api/event';
import {
  ClipboardList,
  Search,
  Settings as SettingsIcon,
  Pin,
  Copy,
  Sparkles,
} from 'lucide-vue-next';
import { api } from '../lib/api';
import { filterClipboard, filterPrompts, formatTime, preview, highlightSegs } from '../lib/search';
import { hasManualVars } from '../lib/vars';
import { computePanelHeight } from '../lib/panelHeight';
import { categoryColor } from '../lib/categoryColor';
import type { AppData, ClipboardItem, Prompt } from '../types';
import VarDialog from '../components/VarDialog.vue';
import EmptyState from '../components/ui/EmptyState.vue';
import CategoryBadge from '../components/ui/CategoryBadge.vue';
import KeyCap from '../components/ui/KeyCap.vue';
import Segmented from '../components/ui/Segmented.vue';

const data = ref<AppData | null>(null);
const query = ref('');
const mode = ref<'prompts' | 'clipboard'>('prompts');
const category = ref('');
const active = ref(0);
const detailOpen = ref(false);
const searchInput = ref<HTMLInputElement | null>(null);
const listEl = ref<HTMLElement | null>(null);
const listInnerEl = ref<HTMLElement | null>(null);
const rootEl = ref<HTMLElement | null>(null);
const atTop = ref(true);
const atBottom = ref(true);

const modeOptions = [
  { id: 'prompts', label: '提示词' },
  { id: 'clipboard', label: '剪贴板' },
];

function updateEdges() {
  const el = listEl.value;
  if (!el) return;
  atTop.value = el.scrollTop < 8;
  atBottom.value = el.scrollTop + el.clientHeight >= el.scrollHeight - 8;
}

const varDialogPrompt = ref<Prompt | null>(null);
const toast = ref('');
const toastKind = ref<'ok' | 'err'>('ok');
let toastTimer: ReturnType<typeof setTimeout> | undefined;
const busy = ref(false);

/** 图片缩略图惰性加载 */
const thumbs = ref<Record<string, string>>({});
const thumbPending = new Set<string>();
function thumbFor(item: ClipboardItem): string {
  if (item.kind !== 'image' || !item.image) return '';
  const cached = thumbs.value[item.id];
  if (cached) return cached;
  if (!thumbPending.has(item.id)) {
    thumbPending.add(item.id);
    api
      .getImageThumb(item.id)
      .then((url) => (thumbs.value[item.id] = url))
      .catch(() => {});
  }
  return '';
}

const MAX_RENDER = 100;
const prompts = computed(() => {
  const list = data.value ? filterPrompts(data.value.prompts, query.value, category.value) : [];
  // 无搜索词时截断渲染，避免大列表拖慢面板（输入即滤全量）
  return query.value.trim() ? list : list.slice(0, MAX_RENDER);
});
const clips = computed(() => {
  const list = data.value ? filterClipboard(data.value.clipboard, query.value) : [];
  return query.value.trim() ? list : list.slice(0, MAX_RENDER);
});
const promptsTotal = computed(() =>
  data.value ? filterPrompts(data.value.prompts, query.value, category.value).length : 0,
);
const clipsTotal = computed(() =>
  data.value ? filterClipboard(data.value.clipboard, query.value).length : 0,
);
const hiddenCount = computed(
  () => (mode.value === 'prompts' ? promptsTotal.value : clipsTotal.value) - items.value,
);
const items = computed(() => (mode.value === 'prompts' ? prompts.value.length : clips.value.length));
const activeItem = computed(() =>
  mode.value === 'prompts' ? prompts.value[active.value] : clips.value[active.value],
);
const activeText = computed(() => {
  const it = activeItem.value as Prompt | ClipboardItem | undefined;
  if (!it) return '';
  return (it as Prompt).content ?? (it as ClipboardItem).content ?? '';
});

function showToast(msg: string, kind: 'ok' | 'err' = 'ok') {
  toast.value = msg;
  toastKind.value = kind;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (toast.value = ''), 1600);
}

async function load() {
  try {
    data.value = await api.getData();
  } catch (e) {
    showToast(String(e), 'err');
  }
}

watch([query, mode, category], () => {
  active.value = 0;
  detailOpen.value = false;
  nextTick(() => listEl.value?.scrollTo({ top: 0 }));
});

// 后台剪贴板事件导致列表长度变化时：只收敛越界的选中项，
// 不重置会话，否则用户正在阅读的全文浮层会被无关的复制动作关掉
watch(items, () => {
  if (active.value >= items.value) {
    active.value = Math.max(0, items.value - 1);
  }
});

watch(active, () => {
  detailOpen.value = false;
  nextTick(() => {
    listEl.value?.querySelector('.item.active')?.scrollIntoView({ block: 'nearest' });
  });
});

/** 面板高度自适应：内容变化后按内容计算期望高度并通知 Rust 调整窗口。
 *  必须用列表内层的自然内容高（.qp-list-inner）而非根元素 offsetHeight——
 *  根元素被 max-height:100vh 封顶（=当前窗口高度），窗口变小后测量值
 *  随之变小，高度将只缩不涨（重开面板只剩两行的根因）。 */
let resizeObs: ResizeObserver | undefined;
let heightTimer: ReturnType<typeof setTimeout> | undefined;
function syncHeight() {
  clearTimeout(heightTimer);
  heightTimer = setTimeout(() => {
    const head = rootEl.value?.querySelector<HTMLElement>('.qp-head');
    const chips = rootEl.value?.querySelector<HTMLElement>('.qp-chips');
    const foot = rootEl.value?.querySelector<HTMLElement>('.qp-foot');
    if (!head || !chips || !foot || !listInnerEl.value) return;
    const h = computePanelHeight({
      head: head.offsetHeight,
      chips: chips.offsetHeight,
      listContent: listInnerEl.value.offsetHeight,
      foot: foot.offsetHeight,
    });
    api.setPanelHeight(h).catch(() => {});
  }, 16);
}

/** {{clipboard}} 自动变量：粘贴/复制前用当前剪贴板文本填充 */
async function fillClipboardVar(text: string): Promise<string> {
  if (!/\{\{\s*clipboard\s*\}\}/i.test(text)) return text;
  const clip = (await api.getClipboardText().catch(() => '')) ?? '';
  if (!clip) showToast('剪贴板为空，{{clipboard}} 已留空', 'err');
  return text.replace(/\{\{\s*clipboard\s*\}\}/gi, clip);
}

async function doPaste(text: string, promptId?: string) {
  if (!text || busy.value) return;
  busy.value = true;
  try {
    const finalText = await fillClipboardVar(text);
    await api.invokePaste(finalText, promptId);
  } catch (e) {
    showToast(String(e), 'err');
  } finally {
    busy.value = false;
  }
}

async function doPasteImage(id: string) {
  if (busy.value) return;
  busy.value = true;
  try {
    await api.pasteImage(id);
  } catch (e) {
    showToast(String(e), 'err');
  } finally {
    busy.value = false;
  }
}

async function doCopy(text: string, promptId?: string) {
  try {
    const finalText = await fillClipboardVar(text);
    await api.copyText(finalText);
    if (promptId) await api.recordUse(promptId).catch(() => {});
    showToast('已复制到剪贴板');
    setTimeout(() => api.hideQuick(), 350);
  } catch (e) {
    showToast(String(e), 'err');
  }
}

function activate(item: Prompt | ClipboardItem, copyOnly: boolean) {
  if (mode.value === 'clipboard') {
    const c = item as ClipboardItem;
    if (c.kind === 'image') {
      if (!copyOnly) doPasteImage(c.id);
      return;
    }
    copyOnly ? doCopy(c.content) : doPaste(c.content);
    return;
  }
  const p = item as Prompt;
  if (!copyOnly && hasManualVars(p.content)) {
    varDialogPrompt.value = p;
    return;
  }
  copyOnly ? doCopy(p.content, p.id) : doPaste(p.content, p.id);
}

function onKeydown(e: KeyboardEvent) {
  if (varDialogPrompt.value) return;
  if (detailOpen.value) {
    // 全文浮层打开时：Esc / ← 关闭，其余不响应
    if (e.key === 'Escape' || e.key === 'ArrowLeft') {
      e.preventDefault();
      detailOpen.value = false;
    } else if (e.key !== 'Tab') {
      e.preventDefault();
    }
    return;
  }
  if (e.key === 'Escape') {
    e.preventDefault();
    api.hideQuick();
  } else if (e.key === 'ArrowDown') {
    e.preventDefault();
    if (items.value > 0) active.value = (active.value + 1) % items.value;
  } else if (e.key === 'ArrowUp') {
    e.preventDefault();
    if (items.value > 0) active.value = (active.value - 1 + items.value) % items.value;
  } else if (e.key === 'Tab') {
    e.preventDefault();
    mode.value = mode.value === 'prompts' ? 'clipboard' : 'prompts';
  } else if (e.key === 'ArrowRight') {
    const el = e.target as HTMLInputElement;
    const atEnd = el.selectionStart === el.value.length && el.selectionEnd === el.value.length;
    if (atEnd && activeText.value) {
      e.preventDefault();
      detailOpen.value = true;
    }
  } else if (e.key === 'Enter') {
    e.preventDefault();
    const copyOnly = e.shiftKey;
    if (mode.value === 'clipboard') {
      const c = clips.value[active.value];
      if (c) activate(c, copyOnly);
    } else {
      const p = prompts.value[active.value];
      if (p) activate(p, copyOnly);
    }
  }
}

let unlisten: (() => void) | undefined;
let unlistenOpen: (() => void) | undefined;


/** 每次呼出重置为全新搜索会话 */
function resetSession() {
  query.value = '';
  mode.value = 'prompts';
  category.value = '';
  active.value = 0;
  detailOpen.value = false;
  varDialogPrompt.value = null;
  searchInput.value?.focus();
  syncHeight();
}

// 变量窗卸载时焦点随其输入框消失（落到 body 上），键盘事件不再经过
// 面板根元素，↑↓/Enter/Esc 将全部失效；关闭（确认或取消）后必须把
// 焦点还给搜索框
watch(varDialogPrompt, (v) => {
  if (!v) nextTick(() => searchInput.value?.focus());
});

onMounted(async () => {
  window.addEventListener('pm-panel-shown', resetSession);
  await load();
  searchInput.value?.focus();
  unlisten = await listen('data-changed', load);
  
  unlistenOpen = await listen<string>('open-prompt', async (e) => {
    // 冷启动时事件可能先于首次 load 到达，确保数据已就绪再查找
    if (!data.value) await load();
    const p = data.value?.prompts.find((x) => x.id === e.payload);
    if (p) varDialogPrompt.value = p;
  });
  // 高度自适应：同时观察根元素（窗口变化）与列表内层（内容变化）——
  // 窗口被最小高度托底且内容再增长时根元素尺寸不变，只有内层会变
  resizeObs = new ResizeObserver(syncHeight);
  if (rootEl.value) resizeObs.observe(rootEl.value);
  if (listInnerEl.value) resizeObs.observe(listInnerEl.value);
  syncHeight();
});

onBeforeUnmount(() => {
  unlisten?.();
  unlistenOpen?.();
  window.removeEventListener('pm-panel-shown', resetSession);
  resizeObs?.disconnect();
  clearTimeout(heightTimer);
});
</script>

<template>
  <div ref="rootEl" class="qp" tabindex="-1" @keydown="onKeydown">
    <!-- 命令栏：全应用唯一的渐变光晕时刻 -->
    <div class="qp-head">
      <div class="cmd">
        <Search :size="17" class="cmd-ico" />
        <input
          ref="searchInput"
          v-model="query"
          class="qp-search"
          type="text"
          :placeholder="mode === 'prompts' ? '搜索提示词，回车即粘贴…' : '搜索剪贴板历史…'"
          spellcheck="false"
        />
      </div>
      <Segmented v-model="mode" :options="modeOptions" />
      <button class="icon-btn" title="管理窗口" aria-label="打开管理窗口" @click="api.openManager()">
        <SettingsIcon :size="16" />
      </button>
    </div>

    <div class="qp-chips">
      <template v-if="mode === 'prompts' && data">
        <button class="chip" :class="{ on: category === '' }" @click="category = ''">全部</button>
        <button
          v-for="c in data.categories"
          :key="c"
          class="chip"
          :class="{ on: category === c }"
          @click="category = category === c ? '' : c"
        >
          <span class="chip-dot" :style="{ background: categoryColor(c).main }" />{{ c }}
        </button>
      </template>
      <span v-else class="chips-hint muted">最近复制的内容 · 图片可直接粘贴</span>
    </div>

    <div
      ref="listEl"
      class="qp-list"
      :class="{ 'fade-top': !atTop, 'fade-bottom': !atBottom }"
      @scroll="updateEdges"
    >
      <div ref="listInnerEl" class="qp-list-inner">
      <template v-if="mode === 'prompts'">
        <div
          v-for="(p, i) in prompts"
          :key="p.id"
          class="item"
          :class="{ active: i === active }"
          @mouseenter="active = i"
          @click="activate(p, false)"
        >
          <div class="item-line">
            <span class="item-title">
              <Pin v-if="p.pinned" :size="13" class="pin" />
              <template v-for="(seg, si) in highlightSegs(p.title, query)" :key="si">
                <b v-if="seg.hit" class="hl">{{ seg.t }}</b>
                <template v-else>{{ seg.t }}</template>
              </template>
            </span>
            <span class="item-meta">
              <KeyCap v-if="p.hotkey" :combo="p.hotkey" />
              <CategoryBadge v-if="p.category" :name="p.category" mode="badge" />
              <span v-if="p.useCount" class="count tnum">{{ p.useCount }} 次</span>
            </span>
          </div>
          <div class="item-preview" :class="{ two: i === active }">
            {{ preview(p.content, i === active ? 160 : 90) }}
          </div>
        </div>
        <div v-if="hiddenCount > 0" class="truncated muted">
          还有 {{ hiddenCount }} 条未显示，输入关键词继续筛选
        </div>
        <EmptyState
          v-if="!prompts.length"
          :icon="Sparkles"
          :title="data && data.prompts.length ? '没有匹配的提示词' : '还没有提示词'"
        >
          <template v-if="data && data.prompts.length">
            换个关键词试试，或按 <kbd>Tab</kbd> 切换到剪贴板
          </template>
          <template v-else>点击右上角 ⚙ 打开管理窗口添加</template>
        </EmptyState>
      </template>

      <template v-else>
        <div
          v-for="(c, i) in clips"
          :key="c.id"
          class="item"
          :class="{ active: i === active }"
          @mouseenter="active = i"
          @click="activate(c, false)"
        >
          <div class="item-line">
            <span class="item-meta"><span class="faint">{{ formatTime(c.copiedAt) }}</span></span>
          </div>
          <img
            v-if="c.kind === 'image' && c.image"
            :src="thumbFor(c)"
            class="item-img"
            alt="剪贴板图片"
          />
          <div v-else class="item-preview" :class="{ two: i === active }">
            {{ preview(c.content, i === active ? 200 : 100) }}
          </div>
        </div>
        <div v-if="hiddenCount > 0" class="truncated muted">
          还有 {{ hiddenCount }} 条未显示，输入关键词继续筛选
        </div>
        <EmptyState v-if="!clips.length" :icon="ClipboardList" title="暂无剪贴板历史">
          在任意程序里复制的文本和截图都会出现在这里
        </EmptyState>
      </template>
      </div>
    </div>

    <div class="qp-foot">
      <span class="hint"><kbd>↑</kbd><kbd>↓</kbd> 选择</span>
      <span class="hint"><kbd>Enter</kbd> 粘贴</span>
      <span class="hint"><kbd>Shift</kbd><kbd>Enter</kbd> 复制</span>
      <span class="hint"><kbd>→</kbd> 全文</span>
      <span class="hint"><kbd>Tab</kbd> 剪贴板</span>
      <span class="grow" />
      <span class="count-all tnum">{{ items }} 项</span>
    </div>

    <!-- 全文浮层：居中阅读卡 -->
    <Transition name="detail">
      <div v-if="detailOpen" class="detail-mask" @click.self="detailOpen = false">
        <div class="detail fade-up">
          <div class="detail-title">
            {{
              mode === 'prompts'
                ? (activeItem as Prompt)?.title
                : '剪贴板内容 · ' + formatTime((activeItem as ClipboardItem)?.copiedAt ?? 0)
            }}
          </div>
          <pre class="detail-body">{{ activeText }}</pre>
          <div class="detail-foot">
            <span class="faint"><kbd>←</kbd> 或 <kbd>Esc</kbd> 返回</span>
            <span class="grow" />
            <button class="sm" :disabled="!activeItem" @click="activeItem && activate(activeItem, true)">
              <Copy :size="13" /> 复制
            </button>
          </div>
        </div>
      </div>
    </Transition>

    <VarDialog
      v-if="varDialogPrompt"
      :prompt="varDialogPrompt"
      @confirm="
        (text) => {
          const id = varDialogPrompt?.id;
          varDialogPrompt = null;
          doPaste(text, id);
        }
      "
      @cancel="varDialogPrompt = null"
    />

    <div v-if="toast" class="toast" :class="toastKind">{{ toast }}</div>
  </div>
</template>

<style scoped>
.qp {
  position: relative;
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: var(--panel);
  border: 1px solid var(--border-strong);
  border-radius: var(--r-lg);
  overflow: hidden;
  box-shadow: var(--shadow-2);
}

/* 命令栏 */
.qp-head {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 10px 6px 10px;
  flex: none;
}

.cmd {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 0 12px;
  background: var(--input-bg);
  border: 1px solid var(--border);
  border-radius: var(--r-md);
  height: 42px;
  transition: border-color var(--t-med), box-shadow var(--t-med);
}

.cmd:focus-within {
  border-color: transparent;
  box-shadow: var(--glow);
}

.cmd-ico {
  color: var(--faint);
  flex: none;
}

.cmd:focus-within .cmd-ico {
  color: var(--brand);
}

.qp-search {
  flex: 1;
  min-width: 0;
  font-size: 14.5px;
  padding: 10px 0;
  background: transparent;
  border: none;
  outline: none;
  color: var(--text);
}

.qp-search::placeholder {
  color: var(--faint);
}

.qp-search:focus {
  background: transparent;
  border: none;
  box-shadow: none;
}

.icon-btn {
  width: 34px;
  height: 34px;
  padding: 0;
  border-radius: var(--r-sm);
  color: var(--muted);
  border-color: transparent;
  background: transparent;
  flex: none;
}

.icon-btn:hover {
  background: var(--panel-2);
  color: var(--text);
}

.qp-chips {
  display: flex;
  gap: 6px;
  padding: 4px 14px 10px;
  overflow-x: auto;
  flex: none;
  align-items: center;
}

.chips-hint {
  font-size: 12px;
}

.chip {
  padding: 4px 12px;
  border-radius: 999px;
  font-size: 12px;
  color: var(--muted);
  background: transparent;
  border: 1px solid var(--border);
  flex: none;
  gap: 6px;
}

.chip-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  display: inline-block;
}

.chip:hover {
  color: var(--text);
  background: var(--panel-2);
}

.chip.on {
  background: var(--brand-soft-2);
  border-color: var(--brand);
  color: var(--text);
}

.qp-list {
  overflow-y: auto;
  padding: 2px 8px 8px;
  min-height: 0;
  max-height: 420px;
  flex: 1 1 auto;
  display: flex;
  flex-direction: column;
}

/* 内容不足时在列表区垂直居中（与空态观感一致）；内容溢出时 margin
   自动归零，不影响滚动与顶部可达性 */
.qp-list-inner {
  margin: auto 0;
}

.qp-list.fade-top {
  mask-image: linear-gradient(to bottom, transparent 0, #000 14px);
}

.qp-list.fade-bottom {
  mask-image: linear-gradient(to bottom, #000 calc(100% - 14px), transparent 100%);
}

.qp-list.fade-top.fade-bottom {
  mask-image: linear-gradient(
    to bottom,
    transparent 0,
    #000 14px,
    #000 calc(100% - 14px),
    transparent 100%
  );
}

.truncated {
  text-align: center;
  font-size: 11.5px;
  padding: 8px 0 4px;
}

.item {
  position: relative;
  padding: 9px 12px 9px 16px;
  border-radius: var(--r-sm);
  cursor: pointer;
  overflow: hidden;
  transition: background var(--t-fast);
}

.item:hover {
  background: var(--panel-2);
}

.item.active {
  background: var(--brand-soft);
}

.item.active::before {
  content: '';
  position: absolute;
  left: 0;
  top: 8px;
  bottom: 8px;
  width: 2.5px;
  border-radius: 3px;
  background: var(--brand);
}

.hl {
  color: var(--brand);
  font-weight: 700;
}

.item-line {
  display: flex;
  align-items: center;
  gap: 8px;
}

.item-title {
  display: flex;
  align-items: center;
  gap: 5px;
  font-weight: 600;
  font-size: 13px;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
}

.pin {
  color: var(--warn);
  flex: none;
}

.item-meta {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  flex: none;
}

.count {
  font-size: 10.5px;
  color: var(--faint);
}

.item-preview {
  color: var(--muted);
  font-size: 12px;
  margin-top: 4px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.item-preview.two {
  white-space: normal;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  line-height: 1.5;
}

.item-img {
  margin-top: 6px;
  max-height: 120px;
  border-radius: var(--r-xs);
  border: 1px solid var(--border);
}

/* 底栏键位图例 */
.qp-foot {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 8px 14px;
  border-top: 1px solid var(--border);
  background: var(--bg-soft);
  flex: none;
  font-size: 11.5px;
  color: var(--muted);
}

.hint {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  white-space: nowrap;
}

.hint kbd {
  font-size: 9.5px;
  padding: 2px 4.5px;
  min-width: 0;
}

.count-all {
  font-size: 10.5px;
  color: var(--faint);
}

/* 全文浮层 */
.detail-mask {
  position: absolute;
  inset: 0;
  background: var(--mask-bg);
  display: grid;
  place-items: center;
  z-index: var(--z-overlay, 40);
  padding: 22px;
}

.detail {
  width: min(640px, 100%);
  max-height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--panel);
  border: 1px solid var(--border-strong);
  border-radius: var(--r-md);
  box-shadow: var(--shadow-2);
  padding: 14px 16px;
  gap: 10px;
}

.detail-title {
  font-weight: 650;
  font-size: var(--fs-lg);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.detail-body {
  font-family: var(--font-mono);
  font-size: 12.5px;
  line-height: 1.7;
  white-space: pre-wrap;
  word-break: break-word;
  overflow-y: auto;
  color: var(--text-2);
  max-width: 65ch;
  user-select: text;
}

.detail-foot {
  display: flex;
  align-items: center;
  font-size: 12px;
}

.detail-foot kbd {
  font-size: 9.5px;
}

.sm {
  padding: 5px 12px;
  font-size: 12px;
}

/* 过渡 */
.detail-enter-active,
.detail-leave-active {
  transition: opacity 180ms var(--ease);
}

.detail-enter-from,
.detail-leave-to {
  opacity: 0;
}
</style>
