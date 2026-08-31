# PromptMate 提示词助手

一个本地优先的跨平台桌面提示词管理工具（Tauri 2 + Vue 3），支持 Windows 与 macOS。

## 功能

- **全局快捷键呼出**：任何应用中按 `Alt+Q`（可自定义）弹出快捷面板，搜索 → 回车即自动粘贴到当前输入框
- **分类 + 搜索**：分类筛选、标题/标签/内容全文搜索、**拼音首字母搜索**（输入 `sjsc` 命中「设计审查」），常用提示词可置顶
- **变量占位符**：提示词中写 `{{变量名|说明}}`，调用时弹窗填写后拼接成最终文本
- **使用频率**：自动记录使用次数与最近使用时间，按频率排序
- **提示词独立快捷键**：给任意提示词绑定专属快捷键（如 `Ctrl+Alt+1`），按下即直接粘贴，含变量的自动弹出填写窗
- **快速捕获**：选中任意文字按 `Alt+S`，一键保存为新提示词
- **面板内预览**：快捷面板按 `→` 展开当前条目的完整内容
- **剪贴板历史**：自动记录系统内新复制的文本**与图片**（可开关、可搜索、可再粘贴）；粘贴完成后可选自动恢复原剪贴板
- **WebDAV 云同步**：支持坚果云等任意 WebDAV 网盘，条目级合并（按更新时间取新、删除传播），支持多设备
- **GitHub Gist 云同步**：数据存入你的 secret Gist（不公开、仅凭 Token 可访问、自带版本历史），首次同步自动创建 Gist，同样支持条目级合并与自动同步
- **自动同步**：开启后，启动时与内容变更后会自动在后台合并同步
- **亮色/暗色主题**：暗色、亮色或跟随系统
- **导入导出**：JSON 完整备份 / Markdown / TXT 批量导入导出
- **系统托盘**：关闭窗口不退出，托盘常驻

## 开发

```bash
pnpm install
pnpm tauri dev     # 开发模式
pnpm tauri build   # 产出安装包（src-tauri/target/release/bundle/nsis/）
```

依赖：Node 18+、Rust。

- Windows：需 MSVC 工具链与 VS Build Tools，安装包产出在 `src-tauri/target/release/bundle/nsis/`
- macOS：需 Xcode Command Line Tools，安装包产出在 `src-tauri/target/release/bundle/dmg/`

### macOS 首次使用

1. 全局快捷键呼出与自动粘贴依赖系统「辅助功能」权限：未授权时应用设置页会显示引导，一键打开
   **系统设置 → 隐私与安全性 → 辅助功能**，勾选 PromptMate 后重启应用即可
2. 未做代码签名，若 Gatekeeper 拦截，右键 App →「打开」即可

### CI 构建（自动发版）

推送 `v*` 标签后，CI 会自动构建两份安装包并附加到对应 Release：
Windows NSIS 安装包、macOS Apple Silicon (M 系列芯片) 的 `.dmg`。

## 数据与隐私

- 全部数据保存在本机应用数据目录的 `data.json`（Windows `%APPDATA%\com.promptmate.app\`，macOS `~/Library/Application Support/com.promptmate.app/`；每次保存自动保留上一份备份 `data.json.bak`；若数据文件损坏，应用会将其隔离为 `data.json.corrupt-*` 并以空数据启动，可从 `.bak` 手动恢复）
- 云同步为可选功能，凭据只存在本机，不上传任何第三方服务器（仅与你配置的 WebDAV 网盘或 GitHub API 通信）
- 文本剪贴板历史默认**不**参与云同步；如需同步，在「云同步」页开启「云同步包含剪贴板历史」（多设备间请保持该开关一致）。图片条目始终只存本机

### GitHub Gist 同步配置

1. 登录 GitHub → 右上角头像 → **Settings** → 左侧最底部 **Developer settings**
2. **Personal access tokens** → **Tokens (classic)** → **Generate new token (classic)**
3. 勾选 `gist` 权限（无需其他权限），生成后复制 Token
4. 在 PromptMate「云同步」页切换到 **GitHub Gist**，粘贴 Token → 保存 → 测试连接
5. Gist ID 留空即可，首次同步时自动创建 secret Gist 并回填

## 快捷键

| 快捷键 | 作用 |
| --- | --- |
| `Alt+Q` | 呼出 / 隐藏快捷面板（可自定义） |
| `Alt+S` | 快速捕获：选中文本一键存为提示词（可自定义） |
| 提示词独立快捷键 | 在编辑器中为提示词绑定，按下即粘贴（可自定义） |
| `↑` `↓` | 选择提示词 |
| `→` | 展开 / 收起当前条目完整预览 |
| `Enter` | 粘贴到当前输入框 |
| `Shift+Enter` | 仅复制到剪贴板 |
| `Tab` | 切换 提示词 / 剪贴板 模式 |
| `Esc` | 关闭面板 |
| `Ctrl+K` | 聚焦搜索框（管理窗口；在无搜索框的页签会自动跳回提示词页） |
| `Ctrl+S` | 管理窗口中保存提示词 |

> 图片条目仅保存在本机（应用数据目录的 `images/`，macOS 路径同上），不参与云同步与 JSON 导出。

## 许可证

本项目基于 [MIT License](LICENSE) 开源发布。
