<script setup lang="ts">
import { computed, inject, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import {
  Plus,
  Search,
  Copy,
  Trash2,
  Pin,
  PinOff,
  Save,
  Sparkles,
  Keyboard,
  Pencil,
  X,
} from 'lucide-vue-next';
import { api } from '../lib/api';
import { managerKey } from '../lib/context';
import { hasVars } from '../lib/vars';
import { preview, matchText, highlightSegs } from '../lib/search';
import { categoryColor } from '../lib/categoryColor';
import { emptyPrompt } from '../types';
import type { Prompt } from '../types';
import HotkeyInput from './HotkeyInput.vue';
import EmptyState from './ui/EmptyState.vue';
import CategoryBadge from './ui/CategoryBadge.vue';
import AccentButton from './ui/AccentButton.vue';
import KeyCap from './ui/KeyCap.vue';

const ctx = inject(managerKey)!;

const varMark = '{{变量}}';

const selectedCategory = ref('');
const query = ref('');
const selectedId = ref('');
const draft = ref<Prompt | null>(null);
const snapshot = ref('');
const dirty = ref(false);
const newCatName = ref('');
const addingCat = ref(false);
const editingCat = ref('');
const editName = ref('');
const renameInput = ref<HTMLInputElement | null>(null);
const searchInput = ref<HTMLInputElement | null>(null);

const allPrompts = computed(() => ctx.data.value?.prompts ?? []);
const categories = computed(() => ctx.data.value?.categories ?? []);

const shown = computed(() => {
  let list = allPrompts.value;
  if (selectedCategory.value) list = list.filter((p) => p.category === selectedCategory.value);
  const q = query.value.trim().toLowerCase();
  if (q) {
    list = list.filter(
      (p) =>
        matchText(p.title, q) ||
        p.content.toLowerCase().includes(q) ||
        p.tags.some((t) => matchText(t, q)),
    );
  }
  const sorted = [...list].sort((a, b) => {
    if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
    return b.updatedAt - a.updatedAt;
  });
  // 大列表截断渲染，避免 DOM 堆积
  return query.value.trim() ? sorted : sorted.slice(0, 200);
});

const catCounts = computed(() => {
  const m = new Map<string, number>();
  for (const p of allPrompts.value) m.set(p.category, (m.get(p.category) ?? 0) + 1);
  return m;
});

const draftColor = computed(() => categoryColor(draft.value?.category ?? ''));

async function openPrompt(p: Prompt) {
  if (!(await confirmDiscard())) return;
  selectedId.value = p.id;
  loadDraft(p);
}

function loadDraft(p: Prompt) {
  draft.value = JSON.parse(JSON.stringify(p));
  snapshot.value = JSON.stringify(draft.value);
  dirty.value = false;
}

async function newPrompt() {
  if (!(await confirmDiscard())) return;
  selectedId.value = '';
  draft.value = emptyPrompt(selectedCategory.value);
  snapshot.value = '';
  dirty.value = false;
}

async function confirmDiscard(): Promise<boolean> {
  if (!dirty.value) return true;
  return ctx.confirm({
    title: '放弃未保存的修改？',
    message: '当前提示词的修改还没有保存。',
    confirmText: '放弃修改',
    danger: true,
  });
}

function markDirty() {
  if (!draft.value) return;
  dirty.value = JSON.stringify(draft.value) !== snapshot.value;
}

async function save() {
  if (!draft.value) return;
  if (!draft.value.title.trim()) {
    ctx.toast('请填写标题', 'err');
    return;
  }
  try {
    await api.savePrompt({ ...draft.value });
    ctx.toast('已保存');
    await ctx.refresh();
    if (selectedId.value) {
      loadDraft(allPrompts.value.find((p) => p.id === selectedId.value) ?? draft.value);
    } else {
      const newest = [...allPrompts.value].sort((a, b) => b.createdAt - a.createdAt)[0];
      if (newest) {
        selectedId.value = newest.id;
        loadDraft(newest);
      }
    }
  } catch (e) {
    ctx.toast(String(e), 'err');
  }
}

async function remove() {
  if (!draft.value?.id) return;
  const removed = draft.value;
  const ok = await ctx.confirm({
    title: `删除「${removed.title}」？`,
    message: '删除后可在通知条中撤销。',
    confirmText: '删除',
    danger: true,
  });
  if (!ok) return;
  try {
    await api.deletePrompt(removed.id);
    selectedId.value = '';
    draft.value = null;
    await ctx.refresh();
    ctx.toast(`已删除「${removed.title}」`, 'ok', {
      label: '撤销',
      handler: async () => {
        try {
          await api.savePrompt(removed);
          await ctx.refresh();
          ctx.toast('已恢复');
        } catch (e) {
          ctx.toast(String(e), 'err');
        }
      },
    });
  } catch (e) {
    ctx.toast(String(e), 'err');
  }
}

async function togglePin() {
  if (!draft.value) return;
  draft.value.pinned = !draft.value.pinned;
  markDirty();
}

function onTagInput(e: Event) {
  if (!draft.value) return;
  const v = (e.target as HTMLInputElement).value;
  draft.value.tags = v
    .split(/[,，]/)
    .map((s) => s.trim())
    .filter(Boolean);
  markDirty();
}

async function copyContent() {
  if (!draft.value?.content) return;
  try {
    await api.copyText(draft.value.content);
    ctx.toast('已复制内容');
  } catch (e) {
    ctx.toast(String(e), 'err');
  }
}

// ---------- 分类管理 ----------

async function addCategory() {
  const name = newCatName.value.trim();
  if (!name) return;
  try {
    await api.addCategory(name);
    newCatName.value = '';
    addingCat.value = false;
    await ctx.refresh();
  } catch (e) {
    ctx.toast(String(e), 'err');
  }
}

function startRename(name: string) {
  editingCat.value = name;
  editName.value = name;
  nextTick(() => renameInput.value?.focus());
}

async function saveRename(oldName: string) {
  if (editingCat.value !== oldName) return;
  editingCat.value = '';
  const name = editName.value.trim();
  if (!name || name === oldName) return;
  try {
    await api.renameCategory(oldName, name);
    if (selectedCategory.value === oldName) selectedCategory.value = name;
    await ctx.refresh();
  } catch (e) {
    ctx.toast(String(e), 'err');
  }
}

async function deleteCategory(name: string) {
  const ok = await ctx.confirm({
    title: `删除分类「${name}」？`,
    message: '其中的提示词将移入「未分类」。',
    confirmText: '删除',
    danger: true,
  });
  if (!ok) return;
  try {
    await api.deleteCategory(name);
    if (selectedCategory.value === name) selectedCategory.value = '';
    await ctx.refresh();
  } catch (e) {
    ctx.toast(String(e), 'err');
  }
}

// ---------- 快捷键 Ctrl+S 保存 / Ctrl+K 聚焦搜索 ----------

function onKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
    e.preventDefault();
    save();
  } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
    e.preventDefault();
    searchInput.value?.focus();
  }
}

onMounted(() => {
  document.addEventListener('keydown', onKeydown);
  window.addEventListener('pm-focus-search', focusSearch);
});
onBeforeUnmount(() => {
  document.removeEventListener('keydown', onKeydown);
  window.removeEventListener('pm-focus-search', focusSearch);
});

function focusSearch() {
  searchInput.value?.focus();
}

// 列表变化时保持选中项的草稿同步（例如云同步覆盖）
watch(allPrompts, () => {
  if (selectedId.value && !dirty.value) {
    const p = allPrompts.value.find((x) => x.id === selectedId.value);
    if (p) loadDraft(p);
  }
});

function fmtTime(ts: number) {
  return ts ? new Date(ts).toLocaleString() : '—';
}
</script>

<template>
  <div class="pp">
    <!-- 上下文工具栏：页名 + 计数 + 搜索 + 新建 -->
    <header class="pp-head">
      <h2 class="pp-title">
        提示词
        <span class="pp-count tnum">{{ allPrompts.length }}</span>
      </h2>
      <span class="grow" />
      <div class="search-box">
        <Search :size="14" class="search-ico" />
        <input
          ref="searchInput"
          v-model="query"
          type="text"
          placeholder="搜索标题 / 标签 / 内容 / 拼音…"
          spellcheck="false"
        />
        <kbd class="search-hint">Ctrl K</kbd>
      </div>
      <AccentButton @click="newPrompt"><Plus :size="15" style="margin-right: 5px" />新建</AccentButton>
    </header>

    <!-- 双栏：列表 + 详情 -->
    <div class="pp-body">
      <div class="col-list">
        <!-- 分类 chips：只占列表宽度，归还编辑器纵向空间 -->
        <div class="cat-row">
          <button class="chip" :class="{ on: selectedCategory === '' }" @click="selectedCategory = ''">
            全部 <span class="chip-n tnum">{{ allPrompts.length }}</span>
          </button>
          <template v-for="c in categories" :key="c">
            <input
              v-if="editingCat === c"
              ref="renameInput"
              v-model="editName"
              class="chip-input"
              placeholder="新名称，回车确认"
              @keydown.enter="saveRename(c)"
              @keydown.esc="editingCat = ''"
              @blur="saveRename(c)"
            />
            <button
              v-else
              class="chip"
              :class="{ on: selectedCategory === c }"
              @click="selectedCategory = selectedCategory === c ? '' : c"
            >
              <span class="chip-dot" :style="{ background: categoryColor(c).main }" />
              {{ c }}
              <span class="chip-n tnum">{{ catCounts.get(c) ?? 0 }}</span>
              <span class="chip-ops" @click.stop>
                <Pencil :size="11" title="重命名" @click="startRename(c)" />
                <X :size="11" title="删除" @click="deleteCategory(c)" />
              </span>
            </button>
          </template>
          <button class="chip chip-add" title="新建分类" @click="addingCat = !addingCat">
            <Plus :size="12" />
          </button>
          <input
            v-if="addingCat"
            v-model="newCatName"
            class="chip-input"
            placeholder="分类名，回车确认"
            @keydown.enter="addCategory"
            @keydown.esc="addingCat = false"
          />
        </div>

        <!-- 列表：扁平行 -->
        <div class="list-scroll">
          <div
            v-for="p in shown"
            :key="p.id"
            class="pitem"
            :class="{ on: p.id === selectedId }"
            @click="openPrompt(p)"
          >
            <div class="pitem-title">
              <Pin v-if="p.pinned" :size="12" class="pin" />
              <template v-for="(seg, si) in highlightSegs(p.title, query)" :key="si">
                <b v-if="seg.hit" class="hl">{{ seg.t }}</b>
                <template v-else>{{ seg.t }}</template>
              </template>
            </div>
            <div class="pitem-row">
              <CategoryBadge v-if="p.category" :name="p.category" mode="dot" />
              <KeyCap v-if="p.hotkey" :combo="p.hotkey" />
              <span class="grow" />
              <span v-if="p.useCount" class="use-n tnum">{{ p.useCount }} 次</span>
            </div>
            <div class="pitem-preview">{{ preview(p.content, 64) }}</div>
          </div>
          <EmptyState v-if="!shown.length" :icon="Sparkles" title="暂无提示词">
            点右上角「新建」添加第一条
          </EmptyState>
        </div>
      </div>

      <div class="col-detail">
        <template v-if="draft">
          <!-- 编辑器头：分类色竖条 + 大标题 + 等宽元数据 -->
          <div class="d-hero">
            <span class="d-hero-bar" :style="{ background: draftColor.main }" />
            <div class="d-hero-main">
              <div class="d-hero-top">
                <span class="d-cat">{{ draft.category || '未分类' }}</span>
                <span class="grow" />
                <button class="mini-btn" :class="{ on: draft.pinned }" @click="togglePin">
                  <Pin v-if="draft.pinned" :size="13" />
                  <PinOff v-else :size="13" />
                  {{ draft.pinned ? '已置顶' : '置顶' }}
                </button>
              </div>
              <input v-model="draft.title" class="d-title" placeholder="提示词标题" @input="markDirty" />
              <div class="d-meta tnum">
                使用 {{ draft.useCount }} 次 · 最近 {{ fmtTime(draft.lastUsedAt) }}
                <template v-if="hasVars(draft.content)"> · <span class="mono">{{ varMark }}</span> 待填写</template>
              </div>
            </div>
          </div>

          <div class="d-body">
            <div class="d-card d-card-content">
              <div class="d-card-head">
                <span>内容</span>
                <span class="faint tnum">{{ draft.content.length }} 字符</span>
              </div>
              <textarea
                v-model="draft.content"
                class="d-content"
                spellcheck="false"
                placeholder="支持 {{变量|说明}} 占位符；{{clipboard}} 自动填入剪贴板内容"
                @input="markDirty"
              />
            </div>

            <div class="d-grid2">
              <div class="d-card">
                <div class="d-card-head"><span>分类与标签</span></div>
                <select v-model="draft.category" class="d-input" @change="markDirty">
                  <option v-for="c in categories" :key="c" :value="c">{{ c }}</option>
                  <option v-if="!categories.includes(draft.category)" :value="draft.category">
                    {{ draft.category }}
                  </option>
                </select>
                <input
                  class="d-input"
                  :value="draft.tags.join(', ')"
                  placeholder="标签，逗号分隔"
                  @input="onTagInput"
                />
              </div>
              <div class="d-card">
                <div class="d-card-head">
                  <Keyboard :size="13" style="color: var(--muted)" />
                  <span>全局快捷键</span>
                </div>
                <HotkeyInput
                  :model-value="draft.hotkey"
                  placeholder="如 Ctrl+Alt+1；保存后生效"
                  @update:model-value="
                    (v) => {
                      if (!draft) return;
                      draft.hotkey = v;
                      markDirty();
                    }
                  "
                />
                <div class="faint d-tip">按下快捷键即粘贴此提示词</div>
              </div>
            </div>
          </div>

          <div class="d-foot">
            <span v-if="dirty" class="dirty-tag">未保存</span>
            <span class="grow" />
            <button class="ghost-btn" @click="copyContent"><Copy :size="14" /> 复制</button>
            <button class="ghost-btn danger" :disabled="!draft.id" @click="remove">
              <Trash2 :size="14" /> 删除
            </button>
            <AccentButton class="save-btn" @click="save">
              <Save :size="14" style="margin-right: 6px" />保存 <kbd>Ctrl S</kbd>
            </AccentButton>
          </div>
        </template>

        <EmptyState v-else :icon="Sparkles" title="从左侧选择提示词">
          或点击右上角「新建」创建一条
          <template #action>
            <AccentButton @click="newPrompt"><Plus :size="14" style="margin-right: 5px" />新建提示词</AccentButton>
          </template>
        </EmptyState>
      </div>
    </div>
  </div>
</template>

<style scoped>
.pp {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

/* 上下文工具栏 */
.pp-head {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 16px;
  height: 52px;
  flex: none;
  border-bottom: 1px solid var(--border);
}

.pp-title {
  display: flex;
  align-items: baseline;
  gap: 8px;
  font-size: var(--fs-lg);
  font-weight: 650;
  letter-spacing: -0.01em;
}

.pp-count {
  font-size: 11px;
  color: var(--faint);
  background: var(--panel-2);
  border: 1px solid var(--border);
  padding: 1px 7px;
  border-radius: 999px;
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
  width: 230px;
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

/* 双栏 */
.pp-body {
  flex: 1;
  min-height: 0;
  display: flex;
}

.col-list {
  width: 320px;
  flex: none;
  border-right: 1px solid var(--border);
  background: var(--bg-soft);
  display: flex;
  flex-direction: column;
  min-height: 0;
}

/* 分类 chips 行 */
.cat-row {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
  padding: 10px 12px;
  border-bottom: 1px solid var(--border);
  flex: none;
}

.chip {
  padding: 3.5px 10px;
  border-radius: 999px;
  font-size: 11.5px;
  color: var(--muted);
  background: var(--panel);
  gap: 6px;
}

.chip:hover {
  background: var(--panel-2);
  border-color: var(--border-strong);
}

.chip-n {
  font-size: 10px;
  color: var(--faint);
}

.chip.on {
  background: var(--brand-soft-2);
  border-color: var(--brand);
  color: var(--text);
}

.chip.on .chip-n {
  color: var(--brand);
}

.chip-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  display: inline-block;
}

.chip-ops {
  display: none;
  gap: 4px;
  color: var(--faint);
}

.chip:hover .chip-ops {
  display: inline-flex;
}

.chip-ops svg:hover {
  color: var(--text);
}

.chip-add {
  padding: 3.5px 8px;
}

.chip-input {
  width: 132px;
  padding: 4px 10px;
  font-size: 11.5px;
}

/* 列表：扁平行 */
.list-scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 8px 8px 12px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.pitem {
  position: relative;
  padding: 9px 11px;
  border-radius: var(--r-sm);
  cursor: pointer;
  transition: background var(--t-fast);
}

.pitem:hover {
  background: var(--panel-2);
}

.pitem.on {
  background: var(--panel);
  box-shadow: inset 0 0 0 1px var(--border-strong);
}

.pitem.on::before {
  content: '';
  position: absolute;
  left: 0;
  top: 8px;
  bottom: 8px;
  width: 2.5px;
  border-radius: 3px;
  background: var(--brand);
}

.pitem-title {
  display: flex;
  align-items: center;
  gap: 5px;
  font-weight: 600;
  font-size: 13px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.pin {
  color: var(--warn);
  flex: none;
}

.hl {
  color: var(--brand);
  font-weight: 700;
}

.pitem-row {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 4px;
}

.use-n {
  font-size: 10.5px;
  color: var(--faint);
}

.pitem-preview {
  color: var(--faint);
  font-size: 11.5px;
  margin-top: 4px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* 详情 */
.col-detail {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  min-height: 0;
  padding: 16px 18px 14px;
}

/* 编辑器头：分类色竖条 */
.d-hero {
  display: flex;
  gap: 14px;
  border: 1px solid var(--border);
  border-radius: var(--r-md);
  background: var(--panel);
  padding: 14px 16px;
  flex: none;
  overflow: hidden;
}

.d-hero-bar {
  width: 3px;
  border-radius: 3px;
  flex: none;
  align-self: stretch;
}

.d-hero-main {
  flex: 1;
  min-width: 0;
}

.d-hero-top {
  display: flex;
  align-items: center;
  gap: 8px;
}

.d-cat {
  font-size: 11.5px;
  font-weight: 650;
  letter-spacing: 0.5px;
  color: var(--muted);
}

.mini-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 11.5px;
  padding: 3.5px 10px;
  border-radius: 999px;
  color: var(--muted);
}

.mini-btn.on {
  color: var(--warn);
  border-color: var(--warn);
  background: var(--warn-soft);
}

.d-title {
  border: none;
  background: transparent;
  font-size: 20px;
  font-weight: 700;
  letter-spacing: -0.01em;
  padding: 4px 0 2px;
  color: var(--text);
}

.d-title:focus {
  box-shadow: none;
  background: transparent;
}

.d-meta {
  font-size: 11px;
  color: var(--faint);
  margin-top: 2px;
}

.d-meta .mono {
  color: var(--brand);
}

.d-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 12px 0;
}

.d-card {
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: var(--r-md);
  padding: 12px 14px;
  display: flex;
  flex-direction: column;
  gap: 9px;
}

.d-card-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  font-size: 12px;
  font-weight: 600;
  color: var(--muted);
}

.d-card-content {
  flex: 1;
  min-height: 220px;
}

.d-content {
  flex: 1;
  min-height: 150px;
  font-family: var(--font-mono);
  font-size: 12.5px;
  line-height: 1.65;
  white-space: pre-wrap;
  background: var(--bg-soft);
}

.d-grid2 {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
  flex: none;
}

.d-input {
  font-size: 12.5px;
  padding: 7px 10px;
}

.d-tip {
  font-size: 11px;
}

.d-foot {
  display: flex;
  align-items: center;
  gap: 8px;
  padding-top: 10px;
  flex: none;
}

.dirty-tag {
  font-size: 11px;
  color: var(--warn);
  background: var(--warn-soft);
  padding: 3px 10px;
  border-radius: 999px;
}

.ghost-btn {
  background: transparent;
  border-color: var(--border);
  color: var(--text-2);
  font-size: 12.5px;
  padding: 7px 14px;
}

.ghost-btn.danger:hover {
  color: var(--danger);
}

.save-btn kbd {
  margin-left: 7px;
  background: rgba(255, 255, 255, 0.16);
  border-color: rgba(255, 255, 255, 0.28);
  box-shadow: none;
  color: #fff;
  font-size: 9.5px;
  padding: 2px 5px;
}
</style>
