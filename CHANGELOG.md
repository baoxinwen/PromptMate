# Changelog

本项目所有显著变更记录于此文件。
格式参考 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
版本号遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

## [Unreleased]

## [0.1.2] - 2026-09-01

### 修复
- JSON 导入：宽松模式保留条目原始 id，修复同 id 提示词重复导入的问题；去重集合同步随收录回写，同一导入文件内的重复 id / 重复（分类，标题）条目也会被正确去重
- 快捷面板 / 管理窗口：修复列表为空时「空状态标题」从不显示的问题
- 管理窗口：修复保存携带已知 id 的提示词时分类未登记（条目游离在分类筛选之外）

### 新增（开发侧）
- 自动化测试体系：Rust 单元/集成测试 78 个、前端单测与组件测试 119 个、浏览器 E2E 12 个、tauri-driver 全栈真机 E2E 2 个
- CI 新增测试门禁 job：Vitest、cargo test 与 vue-tsc 构建全部通过才发版
- 应用支持 PROMPTMATE_E2E / PROMPTMATE_DATA_DIR 环境变量，供自动化测试隔离运行

## [0.1.1] - 2026-08-31

### 新增
- macOS（Apple Silicon）支持，与 Windows 版功能一致
- 快捷键展示按平台适配，macOS 下显示 ⌘ / ⌥ / ⌃
- 全新应用图标

## [0.1.0] - 2026-08-31

### 新增
- 首个公开版本（Windows）
- 全局快捷键呼出快捷面板，搜索 → 回车自动粘贴到当前输入框
- 分类 / 全文 / 拼音首字母搜索，变量占位符，使用频率记录
- 提示词独立快捷键，选中文本快速捕获为新提示词
- 剪贴板历史（文本 + 图片），粘贴后可选自动恢复原剪贴板
- WebDAV / GitHub Gist 云同步，JSON / Markdown / TXT 导入导出，系统托盘常驻

[Unreleased]: https://github.com/baoxinwen/PromptMate/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/baoxinwen/PromptMate/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/baoxinwen/PromptMate/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/baoxinwen/PromptMate/releases/tag/v0.1.0
