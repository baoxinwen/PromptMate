import { describe, it, expect } from 'vitest';
import {
  computePanelHeight,
  PANEL_LIST_MAX,
  PANEL_MIN,
  LIST_VERTICAL_PADDING,
} from './panelHeight';

// 回归背景 1：syncHeight 曾用根元素 offsetHeight 测量期望高度，
// 而根元素被 max-height:100vh（=当前窗口高度）封顶——窗口一旦变小，
// 测量值永远无法超过它，高度只能缩不能涨（快捷面板重开后只剩两行）。
// 回归背景 2：最小高度曾只在 Rust 侧 clamp(300)，前端 0 结果/单条结果
// 的内容高（约 235）与托底值不一致，短列表下方留一大块空白、观感割裂；
// 统一为前端与 Rust 共用 PANEL_MIN=300，短列表配合垂直居中显示。
describe('computePanelHeight：面板期望高度按内容计算，与当前窗口高度解耦', () => {
  const head = 58;
  const chips = 40;
  const foot = 36; // 实测量级

  it('回归：列表内容远超上限时按 LIST_MAX 封顶——窗口被压缩后仍能涨回满高', () => {
    // 场景：窗口已棘轮缩到 336px，但列表实际内容 840px
    // 修前：测得 336 → set_size(336) → 永远卡在两行
    // 修后：按内容算出 556 → 窗口涨回
    const h = computePanelHeight({ head, chips, listContent: 840, foot });
    expect(h).toBe(head + chips + PANEL_LIST_MAX + foot + 2);
  });

  it('回归：列表区高度包含自身内边距——中等数量结果不出现溢出滚动条', () => {
    // 4 条结果内容高 200，列表盒子需要 200+10 内边距才能完整显示
    // 修前：只算 200 → 列表盒盒高不足 → 溢出 10px → 出现滚动条
    const h = computePanelHeight({ head, chips, listContent: 200, foot });
    expect(h).toBe(head + chips + 200 + LIST_VERTICAL_PADDING + foot + 2);
  });

  it('统一最小高度：0 结果 / 单条结果等不足 PANEL_MIN 的内容托底到 PANEL_MIN', () => {
    // 单条结果内容高约 90 → 自然高 226 → 托底 300（与 Rust clamp 一致）
    const one = computePanelHeight({ head, chips, listContent: 90, foot });
    expect(one).toBe(PANEL_MIN);
    const empty = computePanelHeight({ head, chips, listContent: 0, foot });
    expect(empty).toBe(PANEL_MIN);
  });

  it('超过 PANEL_MIN 且未到上限时按内容收缩', () => {
    const h = computePanelHeight({ head, chips, listContent: 300, foot });
    expect(h).toBe(head + chips + 300 + LIST_VERTICAL_PADDING + foot + 2);
  });

  it('内容恰好等于上限时取上限', () => {
    const h = computePanelHeight({ head, chips, listContent: PANEL_LIST_MAX, foot });
    expect(h).toBe(head + chips + PANEL_LIST_MAX + foot + 2);
  });

  it('listMax 可覆盖（供紧凑模式等场景复用），覆盖后仍不低于 PANEL_MIN', () => {
    const h = computePanelHeight({ head, chips, listContent: 500, foot, listMax: 100 });
    expect(h).toBe(PANEL_MIN);
  });
});
