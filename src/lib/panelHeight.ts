/**
 * 快捷面板期望高度：固定区块 + min(列表内容高, 上限) + 根元素上下边框 2px。
 *
 * 必须用列表内容高度（scrollHeight）计算，而不是根元素 offsetHeight：
 * 根元素被 max-height:100vh 封顶（即当前窗口高度），窗口一旦因搜索过滤
 * 或重开时的瞬态渲染变小，测量值就永远无法超过它——高度只能缩不能涨
 * （历史 bug：快捷键重开面板后只剩两行）。
 */
export const PANEL_LIST_MAX = 420;

export function computePanelHeight(parts: {
  head: number;
  chips: number;
  listContent: number;
  foot: number;
  listMax?: number;
}): number {
  const listMax = parts.listMax ?? PANEL_LIST_MAX;
  return parts.head + parts.chips + Math.min(parts.listContent, listMax) + parts.foot + 2;
}
