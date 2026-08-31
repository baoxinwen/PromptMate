<script setup lang="ts">
import { computed, onMounted, reactive } from 'vue';
import { ArrowLeft, CornerDownLeft } from 'lucide-vue-next';
import type { Prompt, VarField } from '../types';
import { applyVars, extractVars, isAutoVar } from '../lib/vars';
import { api } from '../lib/api';
import AccentButton from './ui/AccentButton.vue';

const props = defineProps<{ prompt: Prompt }>();
const emit = defineEmits<{
  (e: 'confirm', text: string): void;
  (e: 'cancel'): void;
}>();

/** 自动变量（如 {{clipboard}}）不进表单，由粘贴链路自动填充 */
const fields = computed<VarField[]>(() =>
  extractVars(props.prompt.content).filter((f) => !isAutoVar(f.name)),
);
const values = reactive<Record<string, string>>({});
const inputEls: HTMLElement[] = [];

function varChip(name: string) {
  return '{{' + name + '}}';
}

onMounted(async () => {
  for (const f of fields.value) values[f.name] = '';
  // 变量值记忆：预填上次填写的内容
  if (props.prompt.id) {
    try {
      const mem = await api.getVarMemory(props.prompt.id);
      for (const f of fields.value) {
        if (mem[f.name]) values[f.name] = mem[f.name];
      }
    } catch {
      /* 记忆读取失败不影响使用 */
    }
  }
  inputEls[0]?.focus();
});

function confirm() {
  emit('confirm', applyVars(props.prompt.content, values));
  // 保存变量值记忆（未保存的新提示词没有 id，跳过）
  if (props.prompt.id && fields.value.length) {
    const snap: Record<string, string> = {};
    for (const f of fields.value) snap[f.name] = values[f.name] ?? '';
    api.saveVarMemory(props.prompt.id, snap).catch(() => {});
  }
}

function onKeydown(e: KeyboardEvent, index: number) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    confirm();
  } else if (e.key === 'Escape') {
    e.preventDefault();
    e.stopPropagation();
    emit('cancel');
  } else if (e.key === 'Tab') {
    e.preventDefault();
    const dir = e.shiftKey ? -1 : 1;
    const next = (index + dir + fields.value.length) % fields.value.length;
    inputEls[next]?.focus();
  }
}
</script>

<template>
  <div class="vd-mask" @mousedown.self="emit('cancel')">
    <div class="vd fade-up">
      <div class="vd-head">
        <button class="vd-back" title="返回" @click="emit('cancel')">
          <ArrowLeft :size="15" />
        </button>
        <span class="vd-title">{{ prompt.title }}</span>
        <span class="vd-count tnum">{{ fields.length }} 个变量</span>
      </div>

      <div class="vd-body">
        <label v-for="(f, i) in fields" :key="f.name" class="vd-field">
          <div class="vd-field-head">
            <span class="vd-name mono">{{ varChip(f.name) }}</span>
            <span v-if="f.hint" class="vd-hint">{{ f.hint }}</span>
          </div>
          <textarea
            :ref="(el) => { if (el) inputEls[i] = el as HTMLElement }"
            v-model="values[f.name]"
            rows="3"
            :placeholder="f.hint || `填入 ${f.name}`"
            @keydown="onKeydown($event, i)"
          />
        </label>
      </div>

      <div class="vd-foot">
        <span class="faint vd-tips">
          <kbd>Enter</kbd> 确认粘贴 · <kbd>Esc</kbd> 取消 · <kbd>Tab</kbd> 切换
        </span>
        <AccentButton @click="confirm">
          <CornerDownLeft :size="14" style="margin-right: 6px" />粘贴
        </AccentButton>
      </div>
    </div>
  </div>
</template>

<style scoped>
.vd-mask {
  position: absolute;
  inset: 0;
  background: var(--mask-bg);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 50;
  padding: 16px;
}

.vd {
  width: min(540px, 100%);
  max-height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--panel);
  border: 1px solid var(--border-strong);
  border-radius: var(--r-lg);
  box-shadow: var(--shadow-2);
  overflow: hidden;
}

.vd-head {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 14px;
  border-bottom: 1px solid var(--border);
  flex: none;
}

.vd-back {
  width: 28px;
  height: 28px;
  padding: 0;
  border-radius: var(--r-sm);
  color: var(--muted);
}

.vd-title {
  font-size: 14.5px;
  font-weight: 650;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.vd-count {
  margin-left: auto;
  font-size: 10.5px;
  color: var(--brand);
  background: var(--brand-soft);
  padding: 2.5px 10px;
  border-radius: 999px;
  flex: none;
}

.vd-body {
  padding: 14px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 13px;
  min-height: 0;
}

.vd-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.vd-field-head {
  display: flex;
  align-items: baseline;
  gap: 8px;
}

.vd-name {
  font-size: 12px;
  font-weight: 600;
  color: var(--brand);
  background: var(--brand-soft);
  border: 1px solid var(--brand-soft-2);
  border-radius: 6px;
  padding: 2px 7px;
  flex: none;
}

.vd-hint {
  font-size: 11.5px;
  color: var(--faint);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.vd-foot {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 14px;
  border-top: 1px solid var(--border);
  flex: none;
}

.vd-tips {
  font-size: 11.5px;
  margin-right: auto;
}

.vd-tips kbd {
  font-size: 9.5px;
  padding: 2px 5px;
}
</style>
