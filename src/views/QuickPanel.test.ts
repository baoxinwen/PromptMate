import { describe, it, expect, vi, beforeEach, afterEach, beforeAll } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import QuickPanel from './QuickPanel.vue';
import VarDialog from '../components/VarDialog.vue';
import { api } from '../lib/api';
import type { AppData, Prompt } from '../types';

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async () => vi.fn()),
}));

vi.mock('../lib/api', () => ({
  api: {
    getData: vi.fn(),
    invokePaste: vi.fn(async () => undefined),
    copyText: vi.fn(async () => undefined),
    recordUse: vi.fn(async () => undefined),
    hideQuick: vi.fn(async () => undefined),
    getClipboardText: vi.fn(async () => ''),
    setPanelHeight: vi.fn(async () => undefined),
    openManager: vi.fn(async () => undefined),
    getImageThumb: vi.fn(async () => ''),
    pasteImage: vi.fn(async () => undefined),
    getVarMemory: vi.fn(async () => ({})),
    saveVarMemory: vi.fn(async () => undefined),
  },
}));

const mockedApi = vi.mocked(api, true);

function makePrompt(overrides: Partial<Prompt> & { id: string; title: string }): Prompt {
  return {
    content: `内容-${overrides.title}`,
    category: '开发',
    tags: [],
    pinned: false,
    hotkey: '',
    useCount: 0,
    lastUsedAt: 0,
    createdAt: 1_000,
    updatedAt: 1_000,
    ...overrides,
  };
}

const fixturePrompts: Prompt[] = [
  makePrompt({ id: 'p1', title: '代码审查', content: '请审查这段代码' }),
  makePrompt({ id: 'p2', title: '周报生成', content: '本周完成：{{本周工作|工作内容}}' }),
  makePrompt({ id: 'p3', title: '翻译助手', category: '写作' }),
];

const fixture: AppData = {
  version: 1,
  seeded: true,
  settings: {} as AppData['settings'],
  categories: ['开发', '写作'],
  prompts: fixturePrompts,
  clipboard: [
    { id: 'c1', content: '复制过的文本', copiedAt: 1_790_000_000_000, kind: 'text' },
  ],
  tombstones: [],
};

async function mountPanel(data: AppData = fixture) {
  mockedApi.getData.mockResolvedValue(data);
  const wrapper = mount(QuickPanel, { attachTo: document.body });
  await flushPromises();
  return wrapper;
}

beforeAll(() => {
  if (!('ResizeObserver' in globalThis)) {
    vi.stubGlobal(
      'ResizeObserver',
      class {
        observe() {}
        unobserve() {}
        disconnect() {}
      },
    );
  }
});

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  vi.useRealTimers();
});

describe('QuickPanel：快捷面板', () => {
  it('加载后渲染提示词列表，首项默认选中', async () => {
    const wrapper = await mountPanel();
    expect(mockedApi.getData).toHaveBeenCalledTimes(1);
    const items = wrapper.findAll('.qp-list .item');
    expect(items.length).toBe(3);
    expect(items[0].classes()).toContain('active');
    expect(items[0].text()).toContain('代码审查');
    wrapper.unmount();
  });

  it('输入关键词联动过滤（含拼音首字母命中）', async () => {
    const wrapper = await mountPanel();
    await wrapper.find('input.qp-search').setValue('dm');
    await flushPromises();
    let titles = wrapper.findAll('.qp-list .item .item-title').map((t) => t.text());
    expect(titles.some((t) => t.includes('代码审查'))).toBe(true);

    await wrapper.find('input.qp-search').setValue('翻译');
    await flushPromises();
    titles = wrapper.findAll('.qp-list .item .item-title').map((t) => t.text());
    expect(titles.length).toBe(1);
    expect(titles[0]).toContain('翻译助手');
    wrapper.unmount();
  });

  it('无匹配时显示空状态', async () => {
    const wrapper = await mountPanel();
    await wrapper.find('input.qp-search').setValue('完全无关的查询词');
    await flushPromises();
    expect(wrapper.text()).toContain('没有匹配的提示词');
    wrapper.unmount();
  });

  it('分类 chip 过滤列表', async () => {
    const wrapper = await mountPanel();
    const chip = wrapper.findAll('.qp-chips .chip').find((c) => c.text().includes('写作'))!;
    await chip.trigger('click');
    await flushPromises();
    const titles = wrapper.findAll('.qp-list .item .item-title').map((t) => t.text());
    expect(titles.length).toBe(1);
    expect(titles[0]).toContain('翻译助手');
    wrapper.unmount();
  });

  it('方向键循环移动选中项', async () => {
    const wrapper = await mountPanel();
    const root = wrapper.find('.qp');
    await root.trigger('keydown', { key: 'ArrowDown' });
    expect(wrapper.findAll('.qp-list .item')[1].classes()).toContain('active');
    await root.trigger('keydown', { key: 'ArrowUp', ...{} });
    expect(wrapper.findAll('.qp-list .item')[0].classes()).toContain('active');
    // 从第 0 项上移应环绕到最后一项
    await root.trigger('keydown', { key: 'ArrowUp' });
    const items = wrapper.findAll('.qp-list .item');
    expect(items[items.length - 1].classes()).toContain('active');
    wrapper.unmount();
  });

  it('Enter 粘贴无变量提示词：invokePaste 收到原文与 id', async () => {
    const wrapper = await mountPanel();
    await wrapper.find('.qp').trigger('keydown', { key: 'Enter' });
    await flushPromises();
    expect(mockedApi.invokePaste).toHaveBeenCalledWith('请审查这段代码', 'p1');
    wrapper.unmount();
  });

  it('Enter 在有变量提示词上弹出变量表单，填写确认后粘贴替换结果', async () => {
    const wrapper = await mountPanel();
    // 下移一次选中 p2（带 {{本周工作}} 变量）
    await wrapper.find('.qp').trigger('keydown', { key: 'ArrowDown' });
    await wrapper.find('.qp').trigger('keydown', { key: 'Enter' });
    await flushPromises();

    const dialog = wrapper.findComponent(VarDialog);
    expect(dialog.exists()).toBe(true);
    expect(mockedApi.invokePaste).not.toHaveBeenCalled();

    await dialog.findAll('textarea')[0].setValue('写完了三个模块');
    await dialog.find('.vd-foot button').trigger('click');
    await flushPromises();

    expect(mockedApi.invokePaste).toHaveBeenCalledWith('本周完成：写完了三个模块', 'p2');
    expect(wrapper.findComponent(VarDialog).exists()).toBe(false);
    wrapper.unmount();
  });

  it('Shift+Enter 复制：recordUse 后延时隐藏面板', async () => {
    vi.useFakeTimers();
    const wrapper = await mountPanel();
    await wrapper.find('.qp').trigger('keydown', { key: 'Enter', shiftKey: true });
    await flushPromises();

    expect(mockedApi.copyText).toHaveBeenCalledWith('请审查这段代码');
    expect(mockedApi.recordUse).toHaveBeenCalledWith('p1');
    expect(mockedApi.hideQuick).not.toHaveBeenCalled();
    vi.advanceTimersByTime(400);
    expect(mockedApi.hideQuick).toHaveBeenCalledTimes(1);
    wrapper.unmount();
  });

  it('Esc 隐藏面板', async () => {
    const wrapper = await mountPanel();
    await wrapper.find('.qp').trigger('keydown', { key: 'Escape' });
    await flushPromises();
    expect(mockedApi.hideQuick).toHaveBeenCalledTimes(1);
    wrapper.unmount();
  });

  it('Tab 切换到剪贴板模式并渲染历史', async () => {
    const wrapper = await mountPanel();
    await wrapper.find('.qp').trigger('keydown', { key: 'Tab' });
    await flushPromises();
    expect(wrapper.findAll('.qp-list .item').length).toBe(1);
    expect(wrapper.findAll('.qp-list .item')[0].text()).toContain('复制过的文本');

    // 剪贴板模式下 Enter 直接粘贴
    await wrapper.find('.qp').trigger('keydown', { key: 'Enter' });
    await flushPromises();
    expect(mockedApi.invokePaste).toHaveBeenCalledWith('复制过的文本', undefined);
    wrapper.unmount();
  });

  it('pm-panel-shown 事件重置搜索会话', async () => {
    const wrapper = await mountPanel();
    await wrapper.find('input.qp-search').setValue('代码');
    window.dispatchEvent(new CustomEvent('pm-panel-shown'));
    await flushPromises();
    expect((wrapper.find('input.qp-search').element as HTMLInputElement).value).toBe('');
    wrapper.unmount();
  });

  it('回归：变量窗取消后焦点回到搜索框，↑↓ 导航恢复', async () => {
    const wrapper = await mountPanel();
    // 通过 open-prompt 事件打开变量窗（p2 含 {{本周工作}}）
    const { listen } = await import('@tauri-apps/api/event');
    const openCall = vi
      .mocked(listen)
      .mock.calls.find(([name]) => name === 'open-prompt')!;
    (openCall[1] as (e: { payload: string }) => void)({ payload: 'p2' });
    await flushPromises();

    const dialog = wrapper.findComponent(VarDialog);
    expect(dialog.exists()).toBe(true);
    // 变量窗挂载后焦点在其输入框上
    expect(document.activeElement?.tagName).toBe('TEXTAREA');

    // Esc 取消 → 变量窗卸载
    await dialog.findAll('textarea')[0].trigger('keydown', { key: 'Escape' });
    await flushPromises();
    expect(wrapper.findComponent(VarDialog).exists()).toBe(false);

    // 修复目标：焦点必须回到搜索框——否则键盘事件落在 body 上，
    // 不再经过面板根元素，↑↓/Enter/Esc 全部失效（修前红：activeElement 是 body）
    expect(document.activeElement).toBe(wrapper.find('input.qp-search').element);

    // 用户可见行为：方向键导航恢复
    await wrapper.find('.qp').trigger('keydown', { key: 'ArrowDown' });
    expect(wrapper.findAll('.qp-list .item')[1].classes()).toContain('active');
    wrapper.unmount();
  });
});
