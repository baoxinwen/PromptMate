<script setup lang="ts">
/** 分段选择器 */
defineProps<{ options: { id: string; label: string }[]; modelValue: string }>();
const emit = defineEmits<{ (e: 'update:modelValue', v: string): void }>();
</script>

<template>
  <div class="seg">
    <button
      v-for="o in options"
      :key="o.id"
      type="button"
      class="seg-item"
      :class="{ on: modelValue === o.id }"
      @click="emit('update:modelValue', o.id)"
    >
      {{ o.label }}
    </button>
  </div>
</template>

<style scoped>
.seg {
  display: inline-flex;
  gap: 2px;
  padding: 3px;
  background: var(--bg-soft);
  border: 1px solid var(--border);
  border-radius: var(--r-sm);
}

.seg-item {
  border: none;
  background: transparent;
  padding: 5px 16px;
  border-radius: 6px;
  color: var(--muted);
  font-size: 12.5px;
  transition: background var(--t-fast), color var(--t-fast), box-shadow var(--t-fast);
}

.seg-item:hover {
  color: var(--text);
  background: transparent;
}

.seg-item.on {
  background: var(--panel-3);
  color: var(--text);
  font-weight: 600;
  box-shadow: var(--shadow-1), inset 0 1px 0 rgba(255, 255, 255, 0.04);
}

:root[data-theme='light'] .seg-item.on {
  box-shadow: var(--shadow-1);
}
</style>
