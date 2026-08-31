<script setup lang="ts">
import { onBeforeUnmount, onMounted } from 'vue';
import { AlertTriangle, X } from 'lucide-vue-next';

const props = withDefaults(
  defineProps<{
    open: boolean;
    title: string;
    message?: string;
    confirmText?: string;
    cancelText?: string;
    danger?: boolean;
  }>(),
  { confirmText: '确认', cancelText: '取消', danger: false, message: '' },
);

const emit = defineEmits<{ (e: 'confirm'): void; (e: 'cancel'): void }>();

function onKeydown(e: KeyboardEvent) {
  if (!props.open) return;
  if (e.key === 'Escape') {
    e.preventDefault();
    e.stopPropagation();
    emit('cancel');
  } else if (e.key === 'Enter') {
    e.preventDefault();
    emit('confirm');
  }
}

onMounted(() => window.addEventListener('keydown', onKeydown, true));
onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown, true));
</script>

<template>
  <Teleport to="body">
    <Transition name="cd">
      <div v-if="open" class="cd-mask" @mousedown.self="emit('cancel')">
        <div class="cd-card" role="alertdialog" aria-modal="true">
          <div class="cd-head">
            <span v-if="danger" class="cd-warn-ico"><AlertTriangle :size="15" /></span>
            <span class="cd-title">{{ title }}</span>
            <button class="cd-x" aria-label="关闭" @click="emit('cancel')">
              <X :size="14" />
            </button>
          </div>
          <div v-if="message" class="cd-msg">{{ message }}</div>
          <div class="cd-foot">
            <button class="cd-btn" @click="emit('cancel')">{{ cancelText }} <kbd>Esc</kbd></button>
            <button
              class="cd-btn cd-confirm"
              :class="{ danger }"
              @click="emit('confirm')"
            >
              {{ confirmText }} <kbd>Enter</kbd>
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.cd-mask {
  position: fixed;
  inset: 0;
  background: var(--mask-bg);
  display: grid;
  place-items: center;
  z-index: var(--z-overlay, 40);
}

.cd-card {
  width: min(400px, calc(100vw - 48px));
  background: var(--panel);
  border: 1px solid var(--border-strong);
  border-radius: var(--r-lg);
  box-shadow: var(--shadow-2);
  padding: 18px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.cd-head {
  display: flex;
  align-items: center;
  gap: 9px;
}

.cd-warn-ico {
  width: 28px;
  height: 28px;
  border-radius: 8px;
  display: grid;
  place-items: center;
  background: var(--danger-soft);
  color: var(--danger);
  flex: none;
}

.cd-title {
  font-weight: 650;
  font-size: var(--fs-lg);
  flex: 1;
}

.cd-x {
  width: 26px;
  height: 26px;
  padding: 0;
  border-color: transparent;
  background: transparent;
  color: var(--muted);
}

.cd-msg {
  color: var(--text-2);
  font-size: var(--fs-base);
  line-height: 1.6;
}

.cd-foot {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 4px;
}

.cd-btn {
  padding: 7px 14px;
  font-size: var(--fs-base);
}

.cd-btn kbd {
  margin-left: 6px;
  opacity: 0.75;
}

.cd-confirm {
  background: var(--panel-2);
  font-weight: 600;
}

.cd-confirm.danger {
  background: var(--danger-btn);
  border-color: transparent;
  color: #fff;
}

.cd-confirm.danger:hover {
  background: var(--danger);
}

/* 过渡：200ms ease-out，scale 0.96 + opacity */
.cd-enter-active,
.cd-leave-active {
  transition: opacity 200ms var(--ease);
}

.cd-enter-active .cd-card,
.cd-leave-active .cd-card {
  transition: transform 200ms var(--ease);
}

.cd-enter-from,
.cd-leave-to {
  opacity: 0;
}

.cd-enter-from .cd-card,
.cd-leave-to .cd-card {
  transform: scale(0.96);
}
</style>
