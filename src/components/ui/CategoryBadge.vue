<script setup lang="ts">
/** 彩色分类圆点/徽章：颜色由分类名稳定分配 */
import { computed } from 'vue';
import { categoryColor } from '../../lib/categoryColor';

const props = withDefaults(defineProps<{ name: string; mode?: 'dot' | 'badge' }>(), {
  mode: 'dot',
});

const color = computed(() => categoryColor(props.name));
</script>

<template>
  <span
    v-if="mode === 'dot'"
    class="dot"
    :style="{ background: color.main, boxShadow: `0 0 6px ${color.main}66` }"
  />
  <span
    v-else
    class="badge"
    :style="{ color: color.main, background: color.soft, borderColor: `${color.main}33` }"
  >{{ name }}</span>
</template>

<style scoped>
.dot {
  display: inline-block;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex: none;
}

.badge {
  display: inline-flex;
  align-items: center;
  padding: 1px 8px;
  border-radius: 999px;
  border: 1px solid;
  font-size: 11px;
  line-height: 1.6;
  white-space: nowrap;
}
</style>
