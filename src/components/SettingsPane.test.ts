import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { ref } from 'vue';
import SettingsPane from './SettingsPane.vue';
import HotkeyInput from './HotkeyInput.vue';
import { api } from '../lib/api';
import { managerKey, type ManagerCtx } from '../lib/context';
import type { AppData, Settings } from '../types';

vi.mock('../lib/api', () => ({
  api: {
    saveSettings: vi.fn(async () => undefined),
    getAutostart: vi.fn(async () => false),
    setAutostart: vi.fn(async () => undefined),
    openDataDir: vi.fn(async () => undefined),
    checkAccessibility: vi.fn(async () => true),
  },
}));

const mockedApi = vi.mocked(api, true);

const baseSettings: Settings = {
  hotkey: 'ctrl+shift+q',
  captureHotkey: 'alt+s',
  captureClipboard: true,
  restoreClipboard: true,
  pasteAppendEnter: false,
  theme: 'light',
  webdav: { enabled: false, autoSync: false, url: '', username: '', password: '' },
  gist: { enabled: false, autoSync: false, token: '', gistId: '' },
  syncProvider: 'webdav',
  syncClipboard: false,
};

function makeCtx(settings: Settings) {
  const toasts: Array<{ msg: string; kind?: string }> = [];
  let refreshes = 0;
  const ctx = {
    data: ref<AppData | null>({ settings } as unknown as AppData),
    refresh: async () => {
      refreshes++;
    },
    toast: (msg: string, kind?: 'ok' | 'err') => {
      toasts.push({ msg, kind });
    },
    confirm: async () => true,
    toasts,
    refreshCount: () => refreshes,
  } satisfies ManagerCtx & { toasts: unknown; refreshCount: () => number };
  return ctx;
}

async function mountPane(settings: Settings = { ...baseSettings }) {
  const ctx = makeCtx(settings);
  const wrapper = mount(SettingsPane, {
    global: { provide: { [managerKey as symbol]: ctx } },
  });
  await flushPromises();
  return { wrapper, ctx };
}

function switchByLabel(wrapper: ReturnType<typeof mount>, text: string) {
  const label = wrapper.findAll('label.opt').find((l) => l.text().includes(text));
  if (!label) throw new Error(`找不到开关: ${text}`);
  return label.find('input[type="checkbox"]');
}

beforeEach(() => {
  vi.clearAllMocks();
  mockedApi.getAutostart.mockResolvedValue(false);
});

describe('SettingsPane：设置读写', () => {
  it('初始回显：主快捷键与主题来自后端设置', async () => {
    const { wrapper } = await mountPane();
    const kbds = wrapper.findComponent(HotkeyInput).findAll('kbd').map((k) => k.text());
    expect(kbds).toEqual(['Ctrl', 'Shift', 'Q']);
    const active = wrapper.find('.theme-card.on');
    expect(active.classes()).toContain('th-light');
    expect(mockedApi.getAutostart).toHaveBeenCalledTimes(1);
  });

  it('点击主题卡：保存的是克隆后的设置（theme 变更且不污染原对象），并轻提示', async () => {
    const settings: Settings = { ...baseSettings };
    const { wrapper, ctx } = await mountPane(settings);

    await wrapper.find('.theme-card.th-dark').trigger('click');
    await flushPromises();

    expect(mockedApi.saveSettings).toHaveBeenCalledTimes(1);
    const payload = mockedApi.saveSettings.mock.calls[0][0];
    expect(payload.theme).toBe('dark');
    expect(payload).not.toBe(settings);
    expect(settings.theme).toBe('light');
    expect(payload.restoreClipboard).toBe(settings.restoreClipboard);
    expect(ctx.toasts[ctx.toasts.length - 1]?.msg).toBe('主题已更新');
    expect(ctx.refreshCount()).toBe(1);
  });

  it('切换「粘贴后恢复原剪贴板」开关：取反并保存', async () => {
    const { wrapper, ctx } = await mountPane();
    const sw = switchByLabel(wrapper, '粘贴后恢复原剪贴板');
    await sw.setValue(false);
    await flushPromises();

    expect(mockedApi.saveSettings.mock.calls[0][0].restoreClipboard).toBe(false);
    expect(ctx.toasts[ctx.toasts.length - 1]?.msg).toBe('已关闭粘贴后恢复剪贴板');
  });

  it('切换「粘贴后自动回车」开关', async () => {
    const { wrapper } = await mountPane();
    const sw = switchByLabel(wrapper, '粘贴后自动回车');
    await sw.setValue(true);
    await flushPromises();

    expect(mockedApi.saveSettings.mock.calls[0][0].pasteAppendEnter).toBe(true);
  });

  it('开机自启：读取真实状态，切换时调用 setAutostart 取反值', async () => {
    mockedApi.getAutostart.mockResolvedValue(true);
    const { wrapper, ctx } = await mountPane();
    const sw = switchByLabel(wrapper, '开机自动启动');
    expect((sw.element as HTMLInputElement).checked).toBe(true);

    await sw.setValue(false);
    await flushPromises();
    expect(mockedApi.setAutostart).toHaveBeenCalledWith(false);
    expect(ctx.toasts[ctx.toasts.length - 1]?.msg).toBe('已关闭开机自启');
  });

  it('修改主快捷键：通过 HotkeyInput 事件保存', async () => {
    const { wrapper } = await mountPane();
    await wrapper.findComponent(HotkeyInput).vm.$emit('update:modelValue', 'ctrl+alt+k');
    await flushPromises();

    expect(mockedApi.saveSettings.mock.calls[0][0].hotkey).toBe('ctrl+alt+k');
  });

  it('保存失败时以 err 提示，不误报成功', async () => {
    mockedApi.saveSettings.mockRejectedValueOnce('磁盘写入失败');
    const { wrapper, ctx } = await mountPane();
    await wrapper.find('.theme-card.th-auto').trigger('click');
    await flushPromises();

    expect(ctx.toasts[ctx.toasts.length - 1]).toMatchObject({
      msg: expect.stringContaining('磁盘写入失败'),
      kind: 'err',
    });
  });
});
