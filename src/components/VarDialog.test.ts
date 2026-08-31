import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import VarDialog from './VarDialog.vue';
import { api } from '../lib/api';
import { emptyPrompt } from '../types';

vi.mock('../lib/api', () => ({
  api: {
    getVarMemory: vi.fn(async () => ({})),
    saveVarMemory: vi.fn(async () => undefined),
  },
}));

const mockedApi = vi.mocked(api, true);

function makePrompt(overrides: Partial<ReturnType<typeof emptyPrompt>> = {}) {
  return {
    ...emptyPrompt(),
    id: 'p1',
    title: '自我介绍',
    content: '你好，我是 {{name|你的名字}}，来自 {{city|所在城市}}。剪贴板：{{clipboard}}',
    ...overrides,
  };
}

async function mountDialog(prompt = makePrompt()) {
  const wrapper = mount(VarDialog, { props: { prompt } });
  await flushPromises();
  return wrapper;
}

beforeEach(() => {
  vi.clearAllMocks();
  mockedApi.getVarMemory.mockImplementation(async () => ({}));
});

describe('VarDialog：变量填写表单', () => {
  it('为每个手动变量生成输入框，{{clipboard}} 不出现在表单', async () => {
    const wrapper = await mountDialog();
    const areas = wrapper.findAll('textarea');
    expect(areas.length).toBe(2);
    expect(wrapper.text()).toContain('{{name}}');
    expect(wrapper.text()).toContain('你的名字');
    expect(wrapper.text()).not.toContain('{{clipboard}}');
    expect(wrapper.text()).toContain('2 个变量');
  });

  it('填写后确认：emit 的文本完成替换，未填写的变量替换为空', async () => {
    const wrapper = await mountDialog();
    await wrapper.findAll('textarea')[0].setValue('小明');
    await wrapper.find('.vd-foot button').trigger('click');
    const evt = wrapper.emitted('confirm')![0][0];
    expect(evt).toBe('你好，我是 小明，来自 。剪贴板：');
    expect(mockedApi.saveVarMemory).toHaveBeenCalledWith('p1', { name: '小明', city: '' });
  });

  it('预填上次记忆的变量值', async () => {
    mockedApi.getVarMemory.mockImplementation(async () => ({ name: '上次的值' }));
    const wrapper = await mountDialog();
    expect(wrapper.findAll('textarea')[0].element.value).toBe('上次的值');
    await wrapper.find('.vd-foot button').trigger('click');
    expect(wrapper.emitted('confirm')![0][0]).toContain('我是 上次的值');
  });

  it('无 id 的新提示词不读写变量记忆', async () => {
    const wrapper = await mountDialog(makePrompt({ id: '' }));
    expect(mockedApi.getVarMemory).not.toHaveBeenCalled();
    await wrapper.find('.vd-foot button').trigger('click');
    expect(mockedApi.saveVarMemory).not.toHaveBeenCalled();
    expect(wrapper.emitted('confirm')![0][0]).toBe('你好，我是 ，来自 。剪贴板：');
  });

  it('文本框内 Enter 确认，Esc 取消', async () => {
    const wrapper = await mountDialog();
    const area = wrapper.findAll('textarea')[0];
    await area.trigger('keydown', { key: 'Enter', shiftKey: false });
    expect(wrapper.emitted('confirm')).toHaveLength(1);

    await wrapper.findAll('textarea')[1].trigger('keydown', { key: 'Escape' });
    expect(wrapper.emitted('cancel')).toHaveLength(1);
  });

  it('点击遮罩与返回按钮都会取消', async () => {
    const wrapper = await mountDialog();
    await wrapper.find('.vd-mask').trigger('mousedown');
    expect(wrapper.emitted('cancel')).toHaveLength(1);

    await wrapper.find('.vd-back').trigger('click');
    expect(wrapper.emitted('cancel')).toHaveLength(2);
  });

  it('无变量的提示词确认时原样输出', async () => {
    const wrapper = await mountDialog(makePrompt({ content: '没有任何变量的文本' }));
    expect(wrapper.findAll('textarea').length).toBe(0);
    await wrapper.find('.vd-foot button').trigger('click');
    expect(wrapper.emitted('confirm')![0][0]).toBe('没有任何变量的文本');
  });
});
