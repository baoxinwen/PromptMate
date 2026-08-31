import { describe, it, expect, vi, afterEach } from 'vitest';
import { matchText, filterPrompts, filterClipboard, formatTime, preview, highlightSegs } from './search';
import type { Prompt } from '../types';

function prompt(overrides: Partial<Prompt> & { id: string; title: string }): Prompt {
  return {
    content: '',
    category: '',
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

describe('matchText：子串与拼音匹配', () => {
  // 注意契约：matchText 的 q 参数需已 toLowerCase（见源码注释），调用方负责小写化
  it('英文子串命中（小写查询）', () => {
    expect(matchText('Code Review', 'code')).toBe(true);
    expect(matchText('Code Review', 'review')).toBe(true);
  });

  it('数字与字母数字混合查询按子串处理', () => {
    expect(matchText('v2.0 发布说明', '20')).toBe(true);
    expect(matchText('issue#42 处理', '#42')).toBe(true);
  });

  it('拼音全拼命中中文标题', () => {
    expect(matchText('代码审查', 'daima')).toBe(true);
    expect(matchText('代码审查', 'daimashencha')).toBe(true);
  });

  it('拼音首字母命中中文标题', () => {
    expect(matchText('代码审查', 'dm')).toBe(true);
    expect(matchText('代码审查', 'dmsc')).toBe(true);
  });

  it('非 ASCII 查询不走拼音索引，只做子串', () => {
    expect(matchText('代码审查', '审查')).toBe(true);
    expect(matchText('代码审查', '评审')).toBe(false);
  });

  it('子串未命中且拼音未命中时返回 false', () => {
    expect(matchText('代码审查', 'zhaosheng')).toBe(false);
    expect(matchText('代码审查', 'xyz')).toBe(false);
  });

  it('空查询视为命中（includes 空串恒真）', () => {
    expect(matchText('任意内容', '')).toBe(true);
  });
});

describe('filterPrompts：过滤与排序', () => {
  it('按分类过滤', () => {
    const list = [
      prompt({ id: 'a', title: '甲', category: '开发' }),
      prompt({ id: 'b', title: '乙', category: '写作' }),
      prompt({ id: 'c', title: '丙', category: '开发' }),
    ];
    const ids = filterPrompts(list, '', '开发').map((p) => p.id);
    expect(ids).toEqual(['a', 'c']);
  });

  it('无查询时排序：置顶 > lastUsedAt > useCount > updatedAt', () => {
    const list = [
      prompt({ id: 'd', title: '低使用', lastUsedAt: 100, useCount: 2, updatedAt: 999 }),
      prompt({ id: 'a', title: '置顶但最旧', pinned: true, lastUsedAt: 0, updatedAt: 1 }),
      prompt({ id: 'c', title: '高使用', lastUsedAt: 100, useCount: 9, updatedAt: 5 }),
      prompt({ id: 'b', title: '最近使用', lastUsedAt: 500, useCount: 1, updatedAt: 2 }),
    ];
    const ids = filterPrompts(list, '', '').map((p) => p.id);
    expect(ids).toEqual(['a', 'b', 'c', 'd']);
  });

  it('有查询时按打分排序：标题命中 > 标签命中 > 正文命中', () => {
    const list = [
      prompt({ id: 'content-only', title: '周报模板', content: '重构相关的说明' }),
      prompt({ id: 'tag-hit', title: '笔记', content: '无关', tags: ['重构'] }),
      prompt({ id: 'title-hit', title: '重构指南', content: '无关' }),
    ];
    const ids = filterPrompts(list, '重构', '').map((p) => p.id);
    expect(ids).toEqual(['title-hit', 'tag-hit', 'content-only']);
  });

  it('置顶压过打分：置顶的正文命中排在未置顶的标题命中前', () => {
    const list = [
      prompt({ id: 'title-hit', title: '重构指南', content: '无关' }),
      prompt({ id: 'pinned-content', title: '普通', content: '重构', pinned: true }),
    ];
    const ids = filterPrompts(list, '重构', '').map((p) => p.id);
    expect(ids).toEqual(['pinned-content', 'title-hit']);
  });

  it('查询为英文大写时公共入口仍大小写不敏感（入口负责小写化）', () => {
    const list = [prompt({ id: 'a', title: 'Note', content: 'review notes' })];
    expect(filterPrompts(list, 'REVIEW', '').map((p) => p.id)).toEqual(['a']);
  });

  it('查询无命中返回空数组；空输入返回空数组', () => {
    const list = [prompt({ id: 'a', title: '标题', content: '正文' })];
    expect(filterPrompts(list, '完全无关词', '')).toEqual([]);
    expect(filterPrompts([], '', '')).toEqual([]);
  });

  it('分类过滤后无匹配返回空数组', () => {
    const list = [prompt({ id: 'a', title: '甲', category: '开发' })];
    expect(filterPrompts(list, '', '设计')).toEqual([]);
  });
});

describe('filterClipboard', () => {
  const items = [
    { id: '1', content: 'Hello World', copiedAt: 1 },
    { id: '2', content: '第二段内容', copiedAt: 2 },
  ];

  it('空查询原样返回全部', () => {
    expect(filterClipboard(items, '')).toEqual(items);
    expect(filterClipboard(items, '   ')).toEqual(items);
  });

  it('大小写不敏感过滤', () => {
    expect(filterClipboard(items, 'hello')).toEqual([items[0]]);
    expect(filterClipboard(items, 'WORLD')).toEqual([items[0]]);
  });

  it('无命中返回空数组', () => {
    expect(filterClipboard(items, '不存在')).toEqual([]);
  });
});

describe('highlightSegs', () => {
  it('单命中：命中段与前后未命中段正确切分', () => {
    expect(highlightSegs('代码重构指南', '重构')).toEqual([
      { t: '代码', hit: false },
      { t: '重构', hit: true },
      { t: '指南', hit: false },
    ]);
  });

  it('大小写不敏感高亮', () => {
    expect(highlightSegs('ABC', 'b')).toEqual([
      { t: 'A', hit: false },
      { t: 'B', hit: true },
      { t: 'C', hit: false },
    ]);
  });

  it('重复相邻命中逐段切分', () => {
    expect(highlightSegs('aaaa', 'aa')).toEqual([
      { t: 'aa', hit: true },
      { t: 'aa', hit: true },
    ]);
  });

  it('无命中与空查询返回单段', () => {
    expect(highlightSegs('纯文本', 'xyz')).toEqual([{ t: '纯文本', hit: false }]);
    expect(highlightSegs('纯文本', '')).toEqual([{ t: '纯文本', hit: false }]);
    expect(highlightSegs('纯文本', '   ')).toEqual([{ t: '纯文本', hit: false }]);
  });

  it('病态输入护栏：超过 32 段后剩余部分合为一段', () => {
    const segs = highlightSegs('x'.repeat(40), 'x');
    expect(segs.length).toBe(33);
    expect(segs.filter((s) => s.hit).length).toBe(32);
    expect(segs[segs.length - 1]).toEqual({ t: 'xxxxxxxx', hit: false });
  });
});

describe('preview', () => {
  it('短文本原样返回', () => {
    expect(preview('短文本')).toBe('短文本');
  });

  it('连续空白折叠为单个空格并去首尾', () => {
    expect(preview('  a   b\n\tc  ')).toBe('a b c');
  });

  it('恰好等于上限不截断，超过截断加省略号', () => {
    expect(preview('x'.repeat(120))).toBe('x'.repeat(120));
    expect(preview('x'.repeat(121))).toBe('x'.repeat(120) + '…');
  });

  it('自定义上限生效', () => {
    expect(preview('abcdef', 3)).toBe('abc…');
  });
});

describe('formatTime', () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it('0 视为空时间返回空串', () => {
    expect(formatTime(0)).toBe('');
  });

  it('当天只显示时分（两位补零）', () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 7, 31, 15, 0));
    expect(formatTime(new Date(2026, 7, 31, 14, 30).getTime())).toBe('14:30');
    expect(formatTime(new Date(2026, 7, 31, 9, 5).getTime())).toBe('09:05');
  });

  it('跨天显示 月/日 时:分', () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 7, 31, 0, 5));
    expect(formatTime(new Date(2026, 7, 30, 9, 5).getTime())).toBe('8/30 09:05');
    expect(formatTime(new Date(2025, 11, 1, 23, 59).getTime())).toBe('12/1 23:59');
  });
});
