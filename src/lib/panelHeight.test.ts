import { describe, it, expect } from 'vitest';
import { computePanelHeight, PANEL_LIST_MAX } from './panelHeight';

// 回归背景：syncHeight 曾用根元素 offsetHeight 测量期望高度，
// 而根元素被 max-height:100vh（=当前窗口高度）封顶——窗口一旦变小，
// 测量值永远无法超过它，高度只能缩不能涨（快捷面板重开后只剩两行）。
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

  it('列表内容少于上限时按内容收缩', () => {
    const h = computePanelHeight({ head, chips, listContent: 100, foot });
    expect(h).toBe(head + chips + 100 + foot + 2);
  });

  it('内容恰好等于上限时取上限', () => {
    const h = computePanelHeight({ head, chips, listContent: PANEL_LIST_MAX, foot });
    expect(h).toBe(head + chips + PANEL_LIST_MAX + foot + 2);
  });

  it('边界：空列表（scrollHeight 为 0）时高度为固定区块之和', () => {
    const h = computePanelHeight({ head, chips, listContent: 0, foot });
    expect(h).toBe(head + chips + 0 + foot + 2);
  });

  it('listMax 可覆盖（供紧凑模式等场景复用）', () => {
    const h = computePanelHeight({ head, chips, listContent: 500, foot, listMax: 200 });
    expect(h).toBe(head + chips + 200 + foot + 2);
  });
});
