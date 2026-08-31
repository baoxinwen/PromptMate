import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { ref } from 'vue';
import PromptsPane from './PromptsPane.vue';
import { api } from '../lib/api';
import { managerKey, type ManagerCtx } from '../lib/context';
import type { AppData, Prompt } from '../types';

vi.mock('../lib/api', () => ({
  api: {
    savePrompt: vi.fn(async () => undefined),
    deletePrompt: vi.fn(async () => undefined),
    copyText: vi.fn(async () => undefined),
    addCategory: vi.fn(async () => undefined),
    renameCategory: vi.fn(async () => undefined),
    deleteCategory: vi.fn(async () => undefined),
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

const p1 = makePrompt({ id: 'p1', title: '代码审查', createdAt: 2_000 });
const p2 = makePrompt({ id: 'p2', title: '翻译助手', category: '写作', createdAt: 3_000 });

function makeCtx(prompts: Prompt[]) {
  const toasts: Array<{ msg: string; kind?: string; action?: unknown }> = [];
  let refreshes = 0;
  let confirmResult = true;
  const confirmCalls: string[] = [];
  const data = ref<AppData | null>({
    version: 1,
    seeded: true,
    settings: {} as AppData['settings'],
    categories: ['开发', '写作'],
    prompts,
  } as AppData);
  const ctx = {
    data,
    refresh: async () => {
      refreshes++;
    },
    toast: (msg: string, kind?: 'ok' | 'err', action?: { label: string; handler: () => void | Promise<void> }) => {
      toasts.push({ msg, kind, action });
    },
    confirm: async (opts: { title: string }) => {
      confirmCalls.push(opts.title);
      return confirmResult;
    },
    toasts,
    confirmResultRef: () => (confirmResult = !confirmResult),
    refreshCount: () => refreshes,
    confirmCalls,
  } as unknown as ManagerCtx & {
    toasts: typeof toasts;
    refreshCount: () => number;
    confirmCalls: string[];
  };
  return ctx;
}

async function mountPane(prompts: Prompt[] = [p1, p2]) {
  const ctx = makeCtx(prompts);
  const wrapper = mount(PromptsPane, {
    global: { provide: { [managerKey as symbol]: ctx } },
    attachTo: document.body,
  });
  await flushPromises();
  return { wrapper, ctx };
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe('PromptsPane：提示词管理', () => {
  it('初始渲染列表，右侧为空状态（含标题文案）', async () => {
    const { wrapper } = await mountPane();
    expect(wrapper.findAll('.pitem').length).toBe(2);
    expect(wrapper.text()).toContain('从左侧选择提示词');
    expect(wrapper.find('.pp-count').text()).toBe('2');
    wrapper.unmount();
  });

  it('点击列表项载入草稿并高亮', async () => {
    const { wrapper } = await mountPane();
    await wrapper.findAll('.pitem')[0].trigger('click');
    await flushPromises();
    expect(wrapper.find('.pitem.on').exists()).toBe(true);
    const title = wrapper.find('input.d-title').element as HTMLInputElement;
    expect(title.value).toBe('代码审查');
    expect((wrapper.find('textarea.d-content').element as HTMLTextAreaElement).value).toContain(
      '内容-代码审查',
    );
    wrapper.unmount();
  });

  it('编辑标题后出现未保存标记，保存调用 savePrompt 并刷新', async () => {
    const { wrapper, ctx } = await mountPane();
    await wrapper.findAll('.pitem')[0].trigger('click');
    await wrapper.find('input.d-title').setValue('代码审查（改）');
    await flushPromises();
    expect(wrapper.find('.dirty-tag').exists()).toBe(true);

    await wrapper.find('.d-foot .save-btn').trigger('click');
    await flushPromises();

    expect(mockedApi.savePrompt).toHaveBeenCalledTimes(1);
    expect(mockedApi.savePrompt.mock.calls[0][0].title).toBe('代码审查（改）');
    expect(mockedApi.savePrompt.mock.calls[0][0].id).toBe('p1');
    expect(ctx.refreshCount()).toBe(1);
    expect(ctx.toasts[ctx.toasts.length - 1]?.msg).toBe('已保存');
    wrapper.unmount();
  });

  it('空标题禁止保存', async () => {
    const { wrapper, ctx } = await mountPane();
    await wrapper.findAll('.pitem')[0].trigger('click');
    await wrapper.find('input.d-title').setValue('   ');
    await wrapper.find('.d-foot .save-btn').trigger('click');
    await flushPromises();

    expect(mockedApi.savePrompt).not.toHaveBeenCalled();
    expect(ctx.toasts[ctx.toasts.length - 1]).toMatchObject({ msg: '请填写标题', kind: 'err' });
    wrapper.unmount();
  });

  it('置顶切换进入草稿，保存后持久化 pinned', async () => {
    const { wrapper } = await mountPane();
    await wrapper.findAll('.pitem')[1].trigger('click');
    await wrapper.findAll('.mini-btn').find((b) => b.text().includes('置顶'))!.trigger('click');
    await wrapper.find('.d-foot .save-btn').trigger('click');
    await flushPromises();

    expect(mockedApi.savePrompt.mock.calls[0][0].pinned).toBe(true);
    wrapper.unmount();
  });

  it('删除需确认：确认后调用 deletePrompt 并给出可撤销提示', async () => {
    const { wrapper, ctx } = await mountPane();
    await wrapper.findAll('.pitem')[0].trigger('click');
    await wrapper.find('.ghost-btn.danger').trigger('click');
    await flushPromises();

    expect(ctx.confirmCalls[0]).toContain('代码审查');
    expect(mockedApi.deletePrompt).toHaveBeenCalledWith('p1');
    expect(ctx.refreshCount()).toBe(1);
    expect(ctx.toasts[ctx.toasts.length - 1]?.msg).toContain('已删除');
    expect((ctx.toasts[ctx.toasts.length - 1]?.action as { label?: string })?.label).toBe('撤销');
    expect(wrapper.find('.d-title').exists()).toBe(false);
    wrapper.unmount();
  });

  it('删除取消时不调用后端', async () => {
    const { wrapper, ctx } = await mountPane([p1]);
    (ctx as unknown as { confirm: () => Promise<boolean> }).confirm = async () => false;
    await wrapper.findAll('.pitem')[0].trigger('click');
    await wrapper.find('.ghost-btn.danger').trigger('click');
    await flushPromises();

    expect(mockedApi.deletePrompt).not.toHaveBeenCalled();
    expect(wrapper.find('input.d-title').exists()).toBe(true);
    wrapper.unmount();
  });

  it('搜索框过滤列表（子串），无结果显示空状态', async () => {
    const { wrapper } = await mountPane();
    await wrapper.find('.search-box input').setValue('翻译');
    await flushPromises();
    let titles = wrapper.findAll('.pitem-title').map((t) => t.text());
    expect(titles).toHaveLength(1);
    expect(titles[0]).toContain('翻译助手');

    await wrapper.find('.search-box input').setValue('不存在的词');
    await flushPromises();
    expect(wrapper.text()).toContain('暂无提示词');
    wrapper.unmount();
  });

  it('分类 chip 过滤', async () => {
    const { wrapper } = await mountPane();
    const chip = wrapper.findAll('.cat-row .chip').find((c) => c.text().includes('写作'))!;
    await chip.trigger('click');
    await flushPromises();
    expect(wrapper.findAll('.pitem').length).toBe(1);
    expect(wrapper.findAll('.pitem-title')[0].text()).toContain('翻译助手');
    wrapper.unmount();
  });

  it('新建提示词：空白草稿，保存时无 id', async () => {
    const { wrapper } = await mountPane();
    await wrapper.findAll('button').find((b) => b.text().includes('新建'))!.trigger('click');
    await flushPromises();

    const title = wrapper.find('input.d-title').element as HTMLInputElement;
    expect(title.value).toBe('');
    // 新建时删除按钮禁用（还没有 id）
    expect((wrapper.find('.ghost-btn.danger').element as HTMLButtonElement).disabled).toBe(true);

    await wrapper.find('input.d-title').setValue('全新提示词');
    await wrapper.find('textarea.d-content').setValue('正文');
    await wrapper.find('.d-foot .save-btn').trigger('click');
    await flushPromises();

    expect(mockedApi.savePrompt.mock.calls[0][0].id).toBe('');
    expect(mockedApi.savePrompt.mock.calls[0][0].title).toBe('全新提示词');
    wrapper.unmount();
  });

  it('Ctrl+S 快捷键触发保存', async () => {
    const { wrapper } = await mountPane();
    await wrapper.findAll('.pitem')[0].trigger('click');
    await wrapper.find('input.d-title').setValue('快捷键保存');
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 's', ctrlKey: true, bubbles: true }));
    await flushPromises();

    expect(mockedApi.savePrompt).toHaveBeenCalledTimes(1);
    wrapper.unmount();
  });
});
