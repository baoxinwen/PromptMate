<script setup lang="ts">
/** 主操作按钮：全应用唯一的实心靛蓝（渐变只留给品牌标与命令栏光晕） */
defineProps<{ disabled?: boolean }>();
</script>

<template>
  <button class="ab" :disabled="disabled">
    <slot />
  </button>
</template>

<style scoped>
.ab {
  background: var(--brand-btn);
  border: 1px solid transparent;
  color: var(--on-brand);
  font-weight: 600;
  padding: 7px 18px;
  border-radius: var(--r-sm);
  cursor: pointer;
  box-shadow: var(--shadow-brand);
  transition: background var(--t-fast), transform var(--t-fast), box-shadow var(--t-fast);
}

.ab:hover {
  background: var(--brand-btn-hover);
  box-shadow: 0 6px 22px rgba(97, 109, 242, 0.36);
}

.ab:active {
  transform: scale(0.97);
}

.ab:disabled {
  opacity: 0.45;
  pointer-events: none;
}
</style>
