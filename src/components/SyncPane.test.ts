import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { ref } from 'vue';
import SyncPane from './SyncPane.vue';
import { api } from '../lib/api';
import { managerKey, type ManagerCtx } from '../lib/context';
import type { AppData, Settings } from '../types';

vi.mock('../lib/api', () => ({
  api: {
    saveSettings: vi.fn(async () => undefined),
    webdavTest: vi.fn(),
    gistTest: vi.fn(),
    syncNow: vi.fn(),
  },
}));

const mockedApi = vi.mocked(api, true);

const baseSettings: Settings = {
  hotkey: 'alt+q',
  captureHotkey: 'alt+s',
  captureClipboard: true,
  restoreClipboard: true,
  pasteAppendEnter: false,
  theme: 'light',
  webdav: {
    enabled: false,
    autoSync: false,
    url: 'https://dav.example.com/dav/promptmate/',
    username: 'user@example.com',
    password: 'app-password',
  },
  gist: { enabled: false, autoSync: false, token: 'ghp_seed', gistId: 'gist123' },
  syncProvider: 'webdav',
  syncClipboard: false,
};

function makeCtx(settings: Settings) {
  const toasts: Array<{ msg: string; kind?: string }> = [];
  let refreshes = 0;
  let confirmResult = true;
  const confirmCalls: string[] = [];
  const ctx = {
    data: ref<AppData | null>({ settings } as unknown as AppData),
    refresh: async () => {
      refreshes++;
    },
    toast: (msg: string, kind?: 'ok' | 'err') => {
      toasts.push({ msg, kind });
    },
    confirm: async (opts: { title: string }) => {
      confirmCalls.push(opts.title);
      return confirmResult;
    },
    toasts,
    refreshCount: () => refreshes,
    confirmCalls,
    setConfirm(v: boolean) {
      confirmResult = v;
    },
  };
  return ctx as typeof ctx & ManagerCtx;
}

async function mountPane(settings: Settings = JSON.parse(JSON.stringify(baseSettings))) {
  const ctx = makeCtx(settings);
  const wrapper = mount(SyncPane, {
    global: { provide: { [managerKey as symbol]: ctx } },
  });
  await flushPromises();
  return { wrapper, ctx };
}

function inputByLabel(wrapper: ReturnType<typeof mount>, label: string) {
  const field = wrapper.findAll('label.field').find((l) => l.text().includes(label));
  if (!field) throw new Error(`找不到输入项: ${label}`);
  return field.find('input');
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe('SyncPane：云同步设置', () => {
  it('挂载时回显后端配置（WebDAV 地址/账号/密码、未启用徽章）', async () => {
    const { wrapper } = await mountPane();
    expect((inputByLabel(wrapper, '服务器地址').element as HTMLInputElement).value).toBe(
      'https://dav.example.com/dav/promptmate/',
    );
    expect((inputByLabel(wrapper, '账号').element as HTMLInputElement).value).toBe('user@example.com');
    expect((inputByLabel(wrapper, '密码').element as HTMLInputElement).type).toBe('password');
    expect(wrapper.find('.on-badge').text()).toContain('未启用');
    expect(wrapper.text()).not.toContain('GitHub Token');
  });

  it('启用后徽章变为已启用；http 地址显示明文警告', async () => {
    const settings = JSON.parse(JSON.stringify(baseSettings));
    settings.webdav.enabled = true;
    const { wrapper } = await mountPane(settings);
    expect(wrapper.find('.on-badge').text()).toContain('已启用');

    const davSwitch = wrapper
      .findAll('label.row.opt')
      .find((l) => l.text().includes('启用 WebDAV 同步'))!
      .find('input');
    await davSwitch.setValue(true);
    expect(wrapper.find('.on-badge').text()).toContain('已启用');

    await inputByLabel(wrapper, '服务器地址').setValue('http://insecure.example.com/dav/');
    await flushPromises();
    expect(wrapper.find('.warn-hint').exists()).toBe(true);
  });

  it('切换 provider：显示 Gist 表单并隐藏 WebDAV 表单', async () => {
    const { wrapper } = await mountPane();
    await wrapper.findAll('.seg-item').find((b) => b.text().includes('GitHub Gist'))!.trigger('click');
    await flushPromises();

    expect(wrapper.text()).toContain('GitHub Token');
    expect(wrapper.text()).not.toContain('服务器地址');
    expect(wrapper.find('.on-badge').text()).toContain('未启用');
  });

  it('保存配置：合并当前表单写入 settings（含 trim）并提示刷新', async () => {
    const { wrapper, ctx } = await mountPane();
    await inputByLabel(wrapper, '服务器地址').setValue('  https://trimmed.example.com/dav/  ');
    await wrapper.find('.btns button').trigger('click');
    await flushPromises();

    const payload = mockedApi.saveSettings.mock.calls[0][0];
    expect(payload.syncProvider).toBe('webdav');
    expect(payload.webdav.url).toBe('https://trimmed.example.com/dav/');
    expect(payload.webdav.password).toBe('app-password');
    expect(payload.gist.token).toBe('ghp_seed');
    expect(ctx.toasts[ctx.toasts.length - 1]?.msg).toBe('同步配置已保存');
    expect(ctx.refreshCount()).toBe(1);
  });

  it('测试连接（WebDAV）：成功显示 ✓，失败显示 ✗', async () => {
    mockedApi.webdavTest.mockResolvedValueOnce('连接成功，目录可读写');
    const { wrapper } = await mountPane();
    await wrapper.findAll('.btns button').find((b) => b.text().includes('测试连接'))!.trigger('click');
    await flushPromises();

    expect(mockedApi.webdavTest).toHaveBeenCalledWith(
      'https://dav.example.com/dav/promptmate/',
      'user@example.com',
      'app-password',
    );
    const status = wrapper.find('.status');
    expect(status.classes()).toContain('ok');
    expect(status.text()).toContain('✓ 连接成功');

    mockedApi.webdavTest.mockRejectedValueOnce('创建网盘目录失败 (HTTP 403)');
    await wrapper.findAll('.btns button').find((b) => b.text().includes('测试连接'))!.trigger('click');
    await flushPromises();
    const errStatus = wrapper.find('.status');
    expect(errStatus.classes()).toContain('err');
    expect(errStatus.text()).toContain('✗ 创建网盘目录失败 (HTTP 403)');
  });

  it('测试连接（Gist）：按当前 provider 调用 gistTest', async () => {
    mockedApi.gistTest.mockResolvedValueOnce('Token 有效（账号 octocat），Gist 可访问');
    const { wrapper } = await mountPane();
    await wrapper.findAll('.seg-item').find((b) => b.text().includes('GitHub Gist'))!.trigger('click');
    await wrapper.findAll('.btns button').find((b) => b.text().includes('测试连接'))!.trigger('click');
    await flushPromises();

    expect(mockedApi.gistTest).toHaveBeenCalledWith('ghp_seed', 'gist123');
    expect(wrapper.find('.status').text()).toContain('✓ Token 有效');
  });

  it('立即同步：先静默保存再调用 syncNow("merge")', async () => {
    mockedApi.syncNow.mockResolvedValueOnce({ added: 1, updated: 2, removed: 0, message: '同步完成' });
    const { wrapper, ctx } = await mountPane();
    await wrapper.findAll('button.ob').find((b) => b.text().includes('立即同步'))!.trigger('click');
    await flushPromises();

    expect(mockedApi.saveSettings).toHaveBeenCalledTimes(1);
    expect(mockedApi.syncNow).toHaveBeenCalledWith('merge');
    expect(wrapper.find('.status').text()).toContain('✓ 同步完成');
    expect(ctx.confirmCalls.length).toBe(0);
  });

  it('仅上传/仅下载是破坏性操作：必须确认，取消则不调用 syncNow', async () => {
    const { wrapper, ctx } = await mountPane();
    ctx.setConfirm(false);

    await wrapper.findAll('button.ob').find((b) => b.text().includes('仅上传'))!.trigger('click');
    await flushPromises();
    expect(ctx.confirmCalls[0]).toContain('仅上传：用本机数据覆盖云端');
    expect(mockedApi.syncNow).not.toHaveBeenCalled();

    await wrapper.findAll('button.ob').find((b) => b.text().includes('仅下载'))!.trigger('click');
    await flushPromises();
    expect(ctx.confirmCalls[1]).toContain('仅下载：用云端数据覆盖本机');
    expect(mockedApi.syncNow).not.toHaveBeenCalled();

    ctx.setConfirm(true);
    mockedApi.syncNow.mockResolvedValue({ added: 0, updated: 0, removed: 0, message: '已上传' });
    await wrapper.findAll('button.ob').find((b) => b.text().includes('仅上传'))!.trigger('click');
    await flushPromises();
    expect(mockedApi.syncNow).toHaveBeenCalledWith('push');
  });

  it('同步失败时状态条显示错误', async () => {
    mockedApi.syncNow.mockRejectedValueOnce('GitHub Token 无效');
    const { wrapper } = await mountPane();
    await wrapper.findAll('button.ob').find((b) => b.text().includes('立即同步'))!.trigger('click');
    await flushPromises();

    expect(wrapper.find('.status').classes()).toContain('err');
    expect(wrapper.find('.status').text()).toContain('✗ GitHub Token 无效');
  });

  it('同步范围开关：syncClipboard 写入保存载荷', async () => {
    const { wrapper } = await mountPane();
    const scope = wrapper
      .findAll('label.row.opt')
      .find((l) => l.text().includes('云同步包含剪贴板历史'))!
      .find('input');
    await scope.setValue(true);

    await wrapper.find('.btns button').trigger('click');
    await flushPromises();
    expect(mockedApi.saveSettings.mock.calls[0][0].syncClipboard).toBe(true);
  });

  it('保存失败时以 err 提示且不触发同步', async () => {
    mockedApi.saveSettings.mockRejectedValueOnce('磁盘写入失败');
    const { wrapper, ctx } = await mountPane();
    await wrapper.findAll('button.ob').find((b) => b.text().includes('立即同步'))!.trigger('click');
    await flushPromises();

    expect(ctx.toasts[ctx.toasts.length - 1]).toMatchObject({ kind: 'err' });
    expect(mockedApi.syncNow).not.toHaveBeenCalled();
  });
});
