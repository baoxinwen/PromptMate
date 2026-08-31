import type { InjectionKey, Ref } from 'vue';
import type { AppData } from '../types';

export interface ToastAction {
  label: string;
  handler: () => void | Promise<void>;
}

export interface ManagerCtx {
  data: Ref<AppData | null>;
  refresh: () => Promise<void>;
  /** 轻提示；action 可选（如「撤销」） */
  toast: (msg: string, kind?: 'ok' | 'err', action?: ToastAction) => void;
  /** 应用内确认框（替换原生 window.confirm），resolve true=确认 */
  confirm: (options: {
    title: string;
    message?: string;
    confirmText?: string;
    danger?: boolean;
  }) => Promise<boolean>;
}

export const managerKey: InjectionKey<ManagerCtx> = Symbol('manager');
