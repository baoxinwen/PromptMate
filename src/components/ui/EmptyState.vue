<script setup lang="ts">
/** 空状态：{{ }} 记号 + 图标 + 标题 + 描述 + 可选动作 */
import type { Component } from 'vue';
import { Sparkles } from 'lucide-vue-next';

withDefaults(defineProps<{ icon?: Component }>(), { icon: () => Sparkles });
</script>

<template>
  <div class="empty">
    <div class="empty-mark mono" aria-hidden="true">{{ '{' + '{ … }' + '}' }}</div>
    <div class="empty-icon">
      <component :is="icon" :size="24" :stroke-width="1.8" />
    </div>
    <div class="empty-title"><slot name="title" /></div>
    <div class="empty-desc"><slot /></div>
    <div v-if="$slots.action" class="empty-action"><slot name="action" /></div>
  </div>
</template>

<style scoped>
.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 40px 20px;
  text-align: center;
}

.empty-mark {
  font-size: 13px;
  color: var(--faint);
  letter-spacing: 1px;
  opacity: 0.85;
  user-select: none;
}

.empty-icon {
  width: 50px;
  height: 50px;
  border-radius: 14px;
  display: grid;
  place-items: center;
  background: var(--panel-2);
  border: 1px dashed var(--border-strong);
  color: var(--brand);
  margin-bottom: 2px;
}

.empty-title {
  font-weight: 600;
  font-size: 14px;
}

.empty-desc {
  color: var(--muted);
  font-size: 12.5px;
  max-width: 300px;
  line-height: 1.6;
}

.empty-action {
  margin-top: 6px;
}
</style>
