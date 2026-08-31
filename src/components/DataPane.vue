<script setup lang="ts">
import { inject, onBeforeUnmount, onMounted, ref } from 'vue';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { FileUp, FileDown, FileJson, FileType2 } from 'lucide-vue-next';
import { api } from '../lib/api';
import { managerKey } from '../lib/context';
import AccentButton from './ui/AccentButton.vue';

const ctx = inject(managerKey)!;
const includeClipboard = ref(false);
const exporting = ref(false);
const importing = ref(false);
const dragging = ref(false);

async function exportData(kind: 'json' | 'markdown') {
  exporting.value = true;
  try {
    const path = await api.exportData(kind, includeClipboard.value);
    if (path) ctx.toast(`已导出到 ${path}`);
  } catch (e) {
    ctx.toast(String(e), 'err');
  } finally {
    exporting.value = false;
  }
}

async function importData() {
  importing.value = true;
  try {
    const r = await api.importData();
    if (r.message !== '已取消') {
      ctx.toast(r.message, r.added > 0 ? 'ok' : 'err');
      await ctx.refresh();
    }
  } catch (e) {
    ctx.toast(String(e), 'err');
  } finally {
    importing.value = false;
  }
}

async function importDropped(paths: string[]) {
  try {
    const r = await api.importPaths(paths);
    ctx.toast(r.message, r.added > 0 ? 'ok' : 'err');
    await ctx.refresh();
  } catch (e) {
    ctx.toast(String(e), 'err');
  }
}

let unlistenDrag: (() => void) | undefined;
onMounted(async () => {
  unlistenDrag = await getCurrentWebview().onDragDropEvent((event) => {
    if (event.payload.type === 'over') {
      dragging.value = true;
    } else if (event.payload.type === 'drop') {
      dragging.value = false;
      importDropped(event.payload.paths);
    } else {
      dragging.value = false;
    }
  });
});
onBeforeUnmount(() => unlistenDrag?.());
</script>

<template>
  <div class="de">
    <header class="de-head">
      <h2 class="de-title">数据</h2>
      <span class="grow" />
    </header>

    <div class="de-body">
      <!-- 导入：大拖拽落区 -->
      <div
        class="dropzone"
        :class="{ drag: dragging }"
        @dragover.prevent="dragging = true"
        @dragleave="dragging = false"
        @drop.prevent="dragging = false"
      >
        <div class="dz-mark mono" aria-hidden="true">{{ '{' + '{ 拖进来 }' + '}' }}</div>
        <FileUp :size="26" :stroke-width="1.6" class="dz-ico" />
        <h3>把备份文件拖到这里</h3>
        <p class="muted desc">
          支持 JSON 备份、Markdown（# 分类 / ## 标题）、TXT（文件名作标题），可多选
        </p>
        <AccentButton :disabled="importing" @click="importData">
          {{ importing ? '导入中…' : '或点击选择文件…' }}
        </AccentButton>
      </div>

      <!-- 导出 -->
      <div class="card export-card">
        <div class="ex-head">
          <FileDown :size="16" class="ex-ico" />
          <h3>导出</h3>
          <span class="muted ex-desc">将提示词导出为文件备份，或在其他工具中使用</span>
        </div>
        <div class="row wrap">
          <AccentButton :disabled="exporting" @click="exportData('json')">
            <FileJson :size="14" style="margin-right: 6px" />JSON 备份
          </AccentButton>
          <button :disabled="exporting" @click="exportData('markdown')">
            <FileType2 :size="14" /> Markdown
          </button>
        </div>
        <label class="row chk">
          <input v-model="includeClipboard" type="checkbox" />
          <span class="muted">JSON 中包含剪贴板历史（不含图片）</span>
        </label>
      </div>
    </div>

    <div class="note faint">
      提示：数据文件保存在本机（设置页可打开目录）。开启云同步后无需手动备份。
      开启自动同步后，内容变更会在后台自动合并到云端。
    </div>
  </div>
</template>

<style scoped>
.de {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.de-head {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 18px;
  height: 52px;
  flex: none;
  border-bottom: 1px solid var(--border);
}

.de-title {
  font-size: var(--fs-lg);
  font-weight: 650;
  letter-spacing: -0.01em;
}

.de-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 18px 22px;
  display: flex;
  flex-direction: column;
  gap: 14px;
  max-width: 760px;
  width: 100%;
}

/* 大拖拽落区 */
.dropzone {
  border: 1.5px dashed var(--border-strong);
  border-radius: var(--r-lg);
  background: var(--bg-soft);
  padding: 34px 24px 30px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 9px;
  text-align: center;
  transition: border-color var(--t-fast), background var(--t-fast), box-shadow var(--t-fast);
}

.dropzone:hover {
  border-color: var(--brand);
}

.dropzone.drag {
  border-color: var(--brand);
  background: var(--brand-soft);
  box-shadow: var(--glow);
}

.dz-mark {
  font-size: 12.5px;
  color: var(--faint);
  letter-spacing: 1px;
  user-select: none;
}

.dz-ico {
  color: var(--brand);
  margin-top: 2px;
}

.dropzone h3 {
  font-size: var(--fs-lg);
  font-weight: 650;
}

.desc {
  font-size: 12.5px;
  line-height: 1.65;
  max-width: 380px;
}

.dropzone .ab {
  margin-top: 8px;
}

/* 导出卡 */
.export-card {
  padding: 16px 18px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.ex-head {
  display: flex;
  align-items: center;
  gap: 8px;
}

.ex-head h3 {
  font-size: 14px;
  font-weight: 650;
}

.ex-ico {
  color: var(--ok);
}

.ex-desc {
  font-size: 12px;
}

.wrap {
  flex-wrap: wrap;
  gap: 8px;
}

.chk {
  gap: 6px;
  font-size: 12px;
}

.chk input {
  accent-color: var(--brand-btn);
}

.note {
  font-size: 11.5px;
  line-height: 1.7;
  padding: 12px 22px 16px;
  flex: none;
}
</style>
