<script setup lang="ts">
import { computed, inject, onBeforeUnmount, onMounted, ref } from 'vue';
import { Copy, Trash2, Search, ClipboardList } from 'lucide-vue-next';
import { api } from '../lib/api';
import { managerKey } from '../lib/context';
import { filterClipboard, formatTime } from '../lib/search';
import EmptyState from './ui/EmptyState.vue';

const ctx = inject(managerKey)!;
const query = ref('');
const searchInput = ref<HTMLInputElement | null>(null);

const MAX_RENDER = 200;
const allItems = computed(() => filterClipboard(ctx.data.value?.clipboard ?? [], query.value));
const items = computed(() =>
  query.value.trim() ? allItems.value : allItems.value.slice(0, MAX_RENDER),
);
const hiddenCount = computed(() => allItems.value.length - items.value.length);
const atTop = ref(true);
const atBottom = ref(true);

function updateEdges(e: Event) {
  const el = e.target as HTMLElement;
  atTop.value = el.scrollTop < 8;
  atBottom.value = el.scrollTop + el.clientHeight >= el.scrollHeight - 8;
}
const todayCount = computed(() => {
  const today = new Date().toDateString();
  return (ctx.data.value?.clipboard ?? []).filter(
    (i) => new Date(i.copiedAt).toDateString() === today,
  ).length;
});

/** 图片缩略图惰性加载 */
const thumbs = ref<Record<string, string>>({});
const thumbPending = new Set<string>();
function thumbFor(item: { id: string; kind?: string; image?: unknown }): string {
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

async function copy(item: { content: string }) {
  try {
    await api.copyText(item.content);
    ctx.toast('已复制');
  } catch (e) {
    ctx.toast(String(e), 'err');
  }
}

async function removeItem(id: string) {
  try {
    await api.deleteHistoryItem(id);
    await ctx.refresh();
  } catch (e) {
    ctx.toast(String(e), 'err');
  }
}

async function clearAll() {
  // 用全量条数（渲染层最多截断 200 条），避免确认框显示的数字与实际清空数不符
  if (!allItems.value.length) return;
  const ok = await ctx.confirm({
    title: `清空全部 ${allItems.value.length} 条剪贴板历史？`,
    message: '清空后无法恢复。',
    confirmText: '清空',
    danger: true,
  });
  if (!ok) return;
  try {
    await api.clearHistory();
    await ctx.refresh();
    ctx.toast('已清空');
  } catch (e) {
    ctx.toast(String(e), 'err');
  }
}

async function toggleCapture() {
  const s = ctx.data.value?.settings;
  if (!s) return;
  try {
    await api.saveSettings({ ...s, captureClipboard: !s.captureClipboard });
    await ctx.refresh();
  } catch (e) {
    ctx.toast(String(e), 'err');
  }
}

// Ctrl+K 聚焦搜索
function onKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
    e.preventDefault();
    searchInput.value?.focus();
  }
}

function focusSearch() {
  searchInput.value?.focus();
}

onMounted(() => {
  document.addEventListener('keydown', onKeydown);
  window.addEventListener('pm-focus-search', focusSearch);
});
onBeforeUnmount(() => {
  document.removeEventListener('keydown', onKeydown);
  window.removeEventListener('pm-focus-search', focusSearch);
});
</script>

<template>
  <div class="cb">
    <header class="cb-head">
      <h2 class="cb-title">
        剪贴板
        <span class="cb-count tnum">{{ ctx.data.value?.clipboard.length ?? 0 }}</span>
      </h2>
      <span class="cb-sub tnum">今日 {{ todayCount }} 条</span>
      <span class="grow" />
      <label class="row cap" title="关闭后将不再记录系统复制的内容">
        <span class="muted">记录新内容</span>
        <span class="switch">
          <input
            type="checkbox"
            :checked="ctx.data.value?.settings.captureClipboard"
            @change="toggleCapture"
          />
          <span class="track"><span class="thumb" /></span>
        </span>
      </label>
      <button class="ghost-btn danger" :disabled="!items.length" @click="clearAll">
        <Trash2 :size="13" /> 清空
      </button>
      <div class="search-box">
        <Search :size="14" class="search-ico" />
        <input
          ref="searchInput"
          v-model="query"
          type="text"
          placeholder="搜索剪贴板…"
          spellcheck="false"
        />
        <kbd class="search-hint">Ctrl K</kbd>
      </div>
    </header>

    <div
      class="list"
      :class="{ 'fade-top': !atTop, 'fade-bottom': !atBottom }"
      @scroll="updateEdges"
    >
      <div v-for="c in items" :key="c.id" class="row-item" :class="{ img: c.kind === 'image' }">
        <img
          v-if="c.kind === 'image' && c.image"
          :src="thumbFor(c)"
          class="clip-img"
          alt="剪贴板图片"
        />
        <div v-else class="content">{{ c.content }}</div>
        <div class="row-side">
          <span class="time tnum">{{ formatTime(c.copiedAt) }}</span>
          <span class="ops">
            <button v-if="c.kind !== 'image'" class="ghost-btn sm" title="复制" @click="copy(c)">
              <Copy :size="12" />
            </button>
            <button class="ghost-btn sm danger" title="删除" @click="removeItem(c.id)">
              <Trash2 :size="12" />
            </button>
          </span>
        </div>
      </div>
      <div v-if="hiddenCount > 0" class="truncated muted">
        还有 {{ hiddenCount }} 条未显示，输入关键词继续筛选
      </div>
      <EmptyState v-if="!items.length" :icon="ClipboardList" title="暂无记录">
        在任意程序里复制的文本和截图都会出现在这里
      </EmptyState>
    </div>
  </div>
</template>

<style scoped>
.cb {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.cb-head {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 0 16px;
  height: 52px;
  border-bottom: 1px solid var(--border);
  flex: none;
}

.cb-title {
  display: flex;
  align-items: baseline;
  gap: 8px;
  font-size: var(--fs-lg);
  font-weight: 650;
  letter-spacing: -0.01em;
}

.cb-count {
  font-size: 11px;
  color: var(--faint);
  background: var(--panel-2);
  border: 1px solid var(--border);
  padding: 1px 7px;
  border-radius: 999px;
}

.cb-sub {
  font-size: 11px;
  color: var(--faint);
}

.cap {
  gap: 8px;
  font-size: 12px;
  flex: none;
}

.search-box {
  position: relative;
  display: flex;
  align-items: center;
}

.search-ico {
  position: absolute;
  left: 9px;
  color: var(--faint);
  pointer-events: none;
}

.search-box input {
  padding: 6px 52px 6px 28px;
  font-size: 12.5px;
  width: 190px;
}

.search-hint {
  position: absolute;
  right: 7px;
  font-size: 9.5px;
  padding: 2px 5px;
  pointer-events: none;
  color: var(--faint);
  background: var(--panel-2);
  transition: opacity var(--t-fast);
}

.search-box:focus-within .search-hint {
  opacity: 0;
}

.list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 10px 16px 14px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.list.fade-top {
  mask-image: linear-gradient(to bottom, transparent 0, #000 14px);
}

.list.fade-bottom {
  mask-image: linear-gradient(to bottom, #000 calc(100% - 14px), transparent 100%);
}

.list.fade-top.fade-bottom {
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
  padding: 6px 0;
}

/* 扁平行 */
.row-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 9px 12px;
  border-radius: var(--r-sm);
  transition: background var(--t-fast);
}

.row-item:hover {
  background: var(--panel-2);
}

.content {
  flex: 1;
  min-width: 0;
  user-select: text;
  color: var(--text-2);
  font-size: 12.5px;
  line-height: 1.55;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  word-break: break-all;
}

.row-item:hover .content {
  -webkit-line-clamp: 6;
}

.clip-img {
  height: 52px;
  max-width: 180px;
  object-fit: cover;
  border: 1px solid var(--border);
  border-radius: var(--r-xs);
  background: var(--panel-2);
  flex: none;
}

.row-side {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: none;
}

.time {
  font-size: 10.5px;
  color: var(--faint);
}

.ops {
  display: flex;
  gap: 2px;
  opacity: 0;
  transition: opacity var(--t-fast);
}

.row-item:hover .ops {
  opacity: 1;
}

.ghost-btn {
  background: transparent;
  border-color: transparent;
  color: var(--muted);
  font-size: 12px;
  padding: 4px 8px;
}

.ghost-btn:hover {
  color: var(--text);
  background: var(--panel-3);
  border-color: var(--border);
}

.ghost-btn.danger:hover {
  color: var(--danger);
}
</style>
