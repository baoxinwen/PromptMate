<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue';
import KeyCap from './ui/KeyCap.vue';
import { hotkeyHint, isMac } from '../lib/platform';

defineProps<{ modelValue: string; placeholder?: string }>();
const emit = defineEmits<{ (e: 'update:modelValue', v: string): void }>();

const capturing = ref(false);

function onCaptureKeydown(e: KeyboardEvent) {
  if (!capturing.value) return;
  e.preventDefault();
  e.stopPropagation();
  if (e.key === 'Escape') {
    capturing.value = false;
    return;
  }
  if (['Alt', 'Control', 'Shift', 'Meta'].includes(e.key)) return;
  if (!e.altKey && !e.ctrlKey && !e.metaKey) {
    return; // 必须带修饰键，忽略纯字母/数字按键
  }
  // 系统保留组合键不允许绑定（否则 Alt+F4 会被全局快捷键抢走，关不掉窗口）
  if (!isMac && e.altKey && e.key === 'F4') return;
  const parts: string[] = [];
  if (e.ctrlKey) parts.push('ctrl');
  if (e.altKey) parts.push('alt');
  if (e.metaKey) parts.push('super');
  if (e.shiftKey) parts.push('shift');
  let key = e.key;
  // 数字键用 e.code 归一：Shift+数字时 e.key 是符号（如 !），accelerator 无法注册；
  // 字母键在 macOS 上 Option+字母 产生组合字符（如 π），同样用 e.code 归一
  if (/^Digit[0-9]$/.test(e.code)) key = e.code.slice(5);
  else if (/^Key[A-Z]$/.test(e.code)) key = e.code.slice(3);
  else if (/^[a-z]$/i.test(key)) key = key.toUpperCase();
  else if (/^F\d{1,2}$/.test(key)) key = key.toUpperCase();
  else if (key === ' ') key = 'space';
  else if (key.startsWith('Arrow')) key = key.toLowerCase();
  parts.push(key);
  capturing.value = false;
  emit('update:modelValue', parts.join('+'));
}

function clear() {
  emit('update:modelValue', '');
}

onMounted(() => window.addEventListener('keydown', onCaptureKeydown, true));
onBeforeUnmount(() => window.removeEventListener('keydown', onCaptureKeydown, true));
</script>

<template>
  <span class="hi">
    <span
      class="key-slot"
      :class="{ capturing, empty: !modelValue }"
      role="button"
      tabindex="0"
      aria-label="点击修改快捷键"
      @click="capturing = true"
      @keydown.enter="capturing = true"
    >
      <KeyCap v-if="modelValue" :combo="modelValue" />
      <span v-else class="key-empty">{{ capturing ? '按下新快捷键…' : '未设置' }}</span>
    </span>
    <button class="sm" type="button" @click="capturing = true">修改</button>
    <button v-if="modelValue" class="sm ghost" type="button" title="清除" @click="clear">×</button>
  </span>

  <Teleport to="body">
    <div v-if="capturing" class="cap-mask" @mousedown.self="capturing = false">
      <div class="cap-card fade-up">
        <div class="cap-title">请按下新的快捷键</div>
        <div class="cap-hint mono">{{ placeholder || hotkeyHint }}</div>
        <div class="cap-keys"><kbd>Esc</kbd> 取消</div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.hi {
  display: inline-flex;
  align-items: center;
  gap: 8px;
}

.key-slot {
  display: inline-flex;
  align-items: center;
  min-width: 150px;
  min-height: 34px;
  padding: 4px 12px;
  border: 1px dashed var(--border-strong);
  border-radius: var(--r-sm);
  background: var(--input-bg);
  cursor: pointer;
  transition: border-color var(--t-fast), background var(--t-fast);
}

.key-slot:hover {
  border-color: var(--brand);
}

.key-slot.capturing {
  border-style: solid;
  border-color: var(--brand);
  box-shadow: 0 0 0 3px var(--brand-soft);
}

.key-slot.empty {
  color: var(--faint);
}

.key-empty {
  font-size: 12px;
  width: 100%;
  text-align: center;
}

.sm {
  padding: 4px 10px;
  font-size: 12px;
}

.cap-mask {
  position: fixed;
  inset: 0;
  background: var(--mask-bg);
  display: grid;
  place-items: center;
  z-index: 60;
}

.cap-card {
  background: var(--panel);
  border: 1px solid var(--border-strong);
  border-radius: var(--r-md);
  padding: 24px 36px;
  text-align: center;
  display: flex;
  flex-direction: column;
  gap: 8px;
  box-shadow: var(--shadow-2);
}

.cap-title {
  font-size: var(--fs-lg);
  font-weight: 650;
}

.cap-hint {
  font-size: 11.5px;
  color: var(--muted);
}

.cap-keys {
  margin-top: 6px;
  font-size: 12px;
  color: var(--muted);
}

.cap-keys kbd {
  margin-right: 4px;
}
</style>
