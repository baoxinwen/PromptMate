import { test, expect } from "@playwright/test";
import { openApp, sampleFirstTitle } from "./fixtures";

test.beforeEach(async ({ page }) => {
  await openApp(page, "manager");
});

test("管理窗口启动：提示词列表与详情可见", async ({ page }) => {
  await expect(page.locator(".pitem")).toHaveCount(3);
  await expect(page.locator(".pp-head")).toContainText("3");
  await page.screenshot({ path: "e2e-artifacts/manager-home.png", fullPage: true });
});

test("新建提示词：填写保存后入列，重启（reload）后仍在", async ({ page }) => {
  await page.locator(".pp-head button", { hasText: "新建" }).click();
  await page.locator("input.d-title").fill("E2E 冒烟提示词");
  await page.locator("textarea.d-content").fill("这是端到端测试创建的内容");
  await page.locator(".d-foot .save-btn").click();

  await expect(page.locator(".toast")).toHaveText(/已保存/);
  await expect(page.locator(".pitem", { hasText: "E2E 冒烟提示词" })).toHaveCount(1);

  // 模拟应用重启：fake-backend 状态在 localStorage，reload 后数据必须还在
  await page.reload();
  await expect(page.locator(".pitem", { hasText: "E2E 冒烟提示词" })).toHaveCount(1);
  await page.screenshot({ path: "e2e-artifacts/manager-after-reload.png", fullPage: true });
});

test("编辑已有提示词并保存，重启后保留修改", async ({ page }) => {
  await page.locator(".pitem", { hasText: "翻译助手" }).click();
  await page.locator("input.d-title").fill("翻译助手（英译）");
  await page.locator(".d-foot .save-btn").click();
  await expect(page.locator(".toast")).toHaveText(/已保存/);

  await page.reload();
  await expect(page.locator(".pitem", { hasText: "翻译助手（英译）" })).toHaveCount(1);
  await expect(page.locator(".pitem", { hasText: /^翻译助手$/ })).toHaveCount(0);
});

test("删除需确认，删除后可通过通知条撤销", async ({ page }) => {
  await page.locator(".pitem", { hasText: "周报生成" }).click();
  await page.locator(".ghost-btn.danger").click();

  const dialog = page.locator(".cd-card");
  await expect(dialog).toBeVisible();
  await expect(dialog).toContainText("删除「周报生成」？");
  await page.screenshot({ path: "e2e-artifacts/manager-delete-confirm.png" });
  await dialog.locator(".cd-confirm").click();

  await expect(page.locator(".toast")).toHaveText(/已删除「周报生成」/);
  await expect(page.locator(".pitem", { hasText: "周报生成" })).toHaveCount(0);

  // 撤销恢复
  await page.locator(".toast .toast-action", { hasText: "撤销" }).click();
  await expect(page.locator(".pitem", { hasText: "周报生成" })).toHaveCount(1);
});

test("设置页：切换主题立即生效并跨重启保留", async ({ page }) => {
  await page.locator(".nav-btn[aria-label='设置']").click();
  await page.locator(".theme-card.th-light").click();
  await expect(page.locator(".toast")).toHaveText(/主题已更新/);
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");

  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await page.locator(".nav-btn[aria-label='设置']").click();
  await expect(page.locator(".theme-card.on")).toHaveClass(/th-light/);
});

test("数据页：用仓库真实样例文件走导入流程", async ({ page }) => {
  await page.locator(".nav-btn[aria-label='数据']").click();
  await page.locator("button", { hasText: "或点击选择文件" }).click();

  await expect(page.locator(".toast")).toHaveText(/导入完成：新增/);
  await page.screenshot({ path: "e2e-artifacts/manager-import.png", fullPage: true });

  // 回到提示词页，样例文件中的第一条提示词已出现
  await page.locator(".nav-btn[aria-label='提示词']").click();
  await expect(page.locator(".pitem").filter({ hasText: sampleFirstTitle })).toHaveCount(1);
});

test("搜索：按标题过滤列表", async ({ page }) => {
  await page.locator(".search-box input").fill("翻译");
  await expect(page.locator(".pitem")).toHaveCount(1);
  await expect(page.locator(".pitem-title", { hasText: "翻译助手" })).toBeVisible();
});
