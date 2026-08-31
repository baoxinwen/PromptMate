/** 分类专属色彩：按分类名从品牌色板稳定分配（chips/徽章/列表点/详情头带全局一致） */

export interface CategoryColor {
  main: string;
  soft: string;
}

const PALETTE: { main: string; soft: string }[] = [
  { main: '#f28ac2', soft: 'rgba(242, 138, 194, 0.14)' },
  { main: '#82a8ff', soft: 'rgba(130, 168, 255, 0.14)' },
  { main: '#4fd6a5', soft: 'rgba(79, 214, 165, 0.14)' },
  { main: '#ffa268', soft: 'rgba(255, 162, 104, 0.14)' },
  { main: '#b49aff', soft: 'rgba(180, 154, 255, 0.14)' },
  { main: '#5cd6e8', soft: 'rgba(92, 214, 232, 0.14)' },
  { main: '#ffd252', soft: 'rgba(255, 210, 82, 0.14)' },
  { main: '#ff8ba0', soft: 'rgba(255, 139, 160, 0.14)' },
  { main: '#4dd6c4', soft: 'rgba(77, 214, 196, 0.14)' },
  { main: '#c9b18f', soft: 'rgba(201, 177, 143, 0.14)' },
];

const NEUTRAL: CategoryColor = { main: '#9a9cb0', soft: 'rgba(154, 156, 176, 0.14)' };

const cache = new Map<string, CategoryColor>();

export function categoryColor(name: string): CategoryColor {
  if (!name) return NEUTRAL;
  const hit = cache.get(name);
  if (hit) return hit;
  let h = 0;
  for (let i = 0; i < name.length; i++) h = (h * 31 + name.charCodeAt(i)) >>> 0;
  const c = PALETTE[h % PALETTE.length];
  cache.set(name, c);
  return c;
}
