import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import HotkeyInput from './HotkeyInput.vue';

function keyEvent(init: Partial<KeyboardEvent> & { key: string }) {
  return new KeyboardEvent('keydown', {
    bubbles: true,
    cancelable: true,
    ...init,
  } as KeyboardEventInit);
}

describe('HotkeyInput：快捷键录制', () => {
  it('按 KeyCap 展示当前组合（非 mac 平台标签）', () => {
    const wrapper = mount(HotkeyInput, { props: { modelValue: 'alt+q' } });
    const caps = wrapper.findAll('kbd').map((k) => k.text());
    expect(caps).toEqual(['Alt', 'Q']);
  });

  it('未设置时显示占位文案', () => {
    const wrapper = mount(HotkeyInput, { props: { modelValue: '' } });
    expect(wrapper.text()).toContain('未设置');
    expect(wrapper.find('kbd').exists()).toBe(false);
  });

  it('点击修改后进入录制态，捕获 Ctrl+J 归一为 e.code 大写键名（由后端 normalize 再转小写注册）', async () => {
    const wrapper = mount(HotkeyInput, { props: { modelValue: '' }, attachTo: document.body });
    await wrapper.find('button.sm').trigger('click');
    expect(document.querySelector('.cap-mask')).toBeTruthy();

    window.dispatchEvent(keyEvent({ key: 'j', code: 'KeyJ', ctrlKey: true }));
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted('update:modelValue')![0]).toEqual(['ctrl+J']);
    expect(document.querySelector('.cap-mask')).toBeNull();
    wrapper.unmount();
  });

  it('Ctrl+Shift+数字按 e.code 归一（% → 5）；Shift 单独不算有效修饰键', async () => {
    const wrapper = mount(HotkeyInput, { props: { modelValue: '' }, attachTo: document.body });
    await wrapper.find('button.sm').trigger('click');
    window.dispatchEvent(keyEvent({ key: '%', code: 'Digit5', ctrlKey: true, shiftKey: true }));
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted('update:modelValue')![0]).toEqual(['ctrl+shift+5']);
    wrapper.unmount();
  });

  it('仅 Shift 修饰不满足绑定要求（必须 Alt/Ctrl/Super）', async () => {
    const wrapper = mount(HotkeyInput, { props: { modelValue: '' }, attachTo: document.body });
    await wrapper.find('button.sm').trigger('click');
    window.dispatchEvent(keyEvent({ key: '5', code: 'Digit5', shiftKey: true }));
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted('update:modelValue')).toBeUndefined();
    expect(document.querySelector('.cap-mask')).toBeTruthy();
    wrapper.unmount();
  });

  it('纯字母（无修饰键）不录制，保持等待输入', async () => {
    const wrapper = mount(HotkeyInput, { props: { modelValue: '' }, attachTo: document.body });
    await wrapper.find('button.sm').trigger('click');
    window.dispatchEvent(keyEvent({ key: 'j', code: 'KeyJ' }));
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted('update:modelValue')).toBeUndefined();
    expect(document.querySelector('.cap-mask')).toBeTruthy();
    wrapper.unmount();
  });

  it('只按修饰键不录制', async () => {
    const wrapper = mount(HotkeyInput, { props: { modelValue: '' }, attachTo: document.body });
    await wrapper.find('button.sm').trigger('click');
    window.dispatchEvent(keyEvent({ key: 'Control', ctrlKey: true }));
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted('update:modelValue')).toBeUndefined();
    expect(document.querySelector('.cap-mask')).toBeTruthy();
    wrapper.unmount();
  });

  it('系统保留组合 Alt+F4 不允许绑定（Windows）', async () => {
    const wrapper = mount(HotkeyInput, { props: { modelValue: '' }, attachTo: document.body });
    await wrapper.find('button.sm').trigger('click');
    window.dispatchEvent(keyEvent({ key: 'F4', code: 'F4', altKey: true }));
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted('update:modelValue')).toBeUndefined();
    wrapper.unmount();
  });

  it('Esc 取消录制，不产生任何值', async () => {
    const wrapper = mount(HotkeyInput, { props: { modelValue: '' }, attachTo: document.body });
    await wrapper.find('button.sm').trigger('click');
    window.dispatchEvent(keyEvent({ key: 'Escape' }));
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted('update:modelValue')).toBeUndefined();
    expect(document.querySelector('.cap-mask')).toBeNull();
    wrapper.unmount();
  });

  it('清除按钮发出空值', async () => {
    const wrapper = mount(HotkeyInput, { props: { modelValue: 'ctrl+k' } });
    await wrapper.find('button.ghost').trigger('click');
    expect(wrapper.emitted('update:modelValue')![0]).toEqual(['']);
  });
});
