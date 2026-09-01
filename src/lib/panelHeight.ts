/**
 * 快捷面板期望高度：固定区块 + min(列表内容高+容器内边距, 上限) + 根元素上下边框 2px，
 * 最后托底到 PANEL_MIN。
 *
 * 必须用列表内层元素（.qp-list-inner）的自然高度测量，而不是根元素
 * offsetHeight：根元素被 max-height:100vh 封顶（即当前窗口高度），窗口一旦
 * 因搜索过滤或重开时的瞬态渲染变小，测量值就永远无法超过它——高度只能缩
 * 不能涨（历史 bug：快捷键重开面板后只剩两行）。
 *
 * 相关基准（改动时需跨端同步）：PANEL_MIN=300 对应 Rust 侧
 * set_panel_height 的 clamp 下限（src-tauri/src/commands.rs）；
 * PANEL_LIST_MAX=420 对应 .qp-list 的 CSS max-height（QuickPanel.vue）。
 */
export const PANEL_LIST_MAX = 420;
export const PANEL_MIN = 300;
/** .qp-list 的上下内边距（padding: 2px 8px 8px → 10px），列表盒子需要
 *  内层高度 + 内边距才能完整显示内容，否则中等数量结果会出现溢出滚动条 */
export const LIST_VERTICAL_PADDING = 10;

export function computePanelHeight(parts: {
  head: number;
  chips: number;
  listContent: number;
  foot: number;
  listMax?: number;
}): number {
  const listMax = parts.listMax ?? PANEL_LIST_MAX;
  const list = Math.min(parts.listContent + LIST_VERTICAL_PADDING, listMax);
  const raw = parts.head + parts.chips + list + parts.foot + 2;
  return Math.max(raw, PANEL_MIN);
}
