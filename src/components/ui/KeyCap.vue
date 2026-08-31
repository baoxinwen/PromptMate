<script setup lang="ts">
/** 键帽视觉（显示快捷键组合） */
defineProps<{ combo: string }>();

const LABELS: Record<string, string> = {
  ctrl: 'Ctrl',
  alt: 'Alt',
  shift: 'Shift',
  super: 'Win',
  meta: 'Win',
  space: 'Space',
  enter: 'Enter',
  esc: 'Esc',
  tab: 'Tab',
};

function keys(combo: string): string[] {
  if (!combo) return [];
  return combo.split('+').map((p) => {
    const k = p.trim().toLowerCase();
    if (LABELS[k]) return LABELS[k];
    return k.length === 1 ? k.toUpperCase() : k.charAt(0).toUpperCase() + k.slice(1);
  });
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
