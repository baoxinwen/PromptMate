<script setup lang="ts">
/** 键帽视觉（显示快捷键组合），按平台显示 Win 风格或 macOS 符号 */
import { keyLabel } from '../../lib/platform';

defineProps<{ combo: string }>();

function keys(combo: string): string[] {
  if (!combo) return [];
  return combo.split('+').map(keyLabel);
}
</script>

<template>
  <span class="kc">
    <kbd v-for="k in keys(combo)" :key="k">{{ k }}</kbd>
  </span>
</template>

<style scoped>
.kc {
  display: inline-flex;
  gap: 4px;
  align-items: center;
}
</style>
