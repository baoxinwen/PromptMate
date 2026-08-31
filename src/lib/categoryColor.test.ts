import { describe, it, expect } from 'vitest';
import { categoryColor } from './categoryColor';

const PALETTE_MAINS = [
  '#f28ac2',
  '#82a8ff',
  '#4fd6a5',
  '#ffa268',
  '#b49aff',
  '#5cd6e8',
  '#ffd252',
  '#ff8ba0',
  '#4dd6c4',
  '#c9b18f',
];

describe('categoryColor', () => {
  it('空分类名返回中性色', () => {
    expect(categoryColor('')).toEqual({ main: '#9a9cb0', soft: 'rgba(154, 156, 176, 0.14)' });
  });

  it('同名分类颜色稳定（缓存一致性）', () => {
    const first = categoryColor('开发');
    const second = categoryColor('开发');
    expect(second).toEqual(first);
  });

  it('返回值必须属于 10 色板之一', () => {
    for (const name of ['开发', '写作', '设计', 'project-a', '分类九']) {
      const { main } = categoryColor(name);
      expect(PALETTE_MAINS, `「${name}」的主色 ${main} 应在色板内`).toContain(main);
    }
  });

  it('不同分类名映射到不同颜色（哈希分散性抽查）', () => {
    expect(categoryColor('a').main).not.toBe(categoryColor('b').main);
    expect(categoryColor('开发').main).not.toBe(categoryColor('写作').main);
  });

  it('中英文与特殊字符分类名均可用', () => {
    for (const name of ['中文分类', 'english', 'with space', '🎉emoji', 'a'.repeat(100)]) {
      const c = categoryColor(name);
      expect(c.main).toMatch(/^#[0-9a-f]{6}$/);
      expect(c.soft).toContain('0.14');
    }
  });
});
