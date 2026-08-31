use serde::{Deserialize, Serialize};

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

/// 剪贴板历史上限（监听记录、云同步合并共用，避免两处数值漂移）
pub const MAX_CLIPBOARD_ITEMS: usize = 1000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    pub id: String,
    pub title: String,
    pub content: String,
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub pinned: bool,
    /// 独立全局快捷键（tauri accelerator 格式，空串表示未绑定）
    #[serde(default)]
    pub hotkey: String,
    #[serde(default)]
    pub use_count: u64,
    #[serde(default)]
    pub last_used_at: u64,
    #[serde(default = "now_ms")]
    pub created_at: u64,
    #[serde(default = "now_ms")]
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageRef {
    /// 相对 images 目录的文件名（如 "<id>.png"）
    pub file: String,
    pub width: u32,
    pub height: u32,
}

fn default_kind() -> String {
    "text".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardItem {
    pub id: String,
    #[serde(default)]
    pub content: String,
    #[serde(default = "now_ms")]
    pub copied_at: u64,
    /// 条目类型："text" | "image"
    #[serde(default = "default_kind")]
    pub kind: String,
    /// 图片条目的文件引用（图片本体存磁盘，不入 data.json）
    #[serde(default)]
    pub image: Option<ImageRef>,
}

impl ClipboardItem {
    pub fn is_image(&self) -> bool {
        self.kind == "image"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tombstone {
    pub id: String,
    pub at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebDavConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub auto_sync: bool,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
}

impl Default for WebDavConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_sync: false,
            url: String::new(),
            username: String::new(),
            password: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GistConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub auto_sync: bool,
    /// GitHub Personal Access Token（需 gist 权限）
    #[serde(default)]
    pub token: String,
    /// secret Gist id，留空则在首次同步时自动创建
    #[serde(default)]
    pub gist_id: String,
}

impl Default for GistConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_sync: false,
            token: String::new(),
            gist_id: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// 全局快捷键（tauri accelerator 格式，如 "alt+q"）
    #[serde(default = "default_hotkey")]
    pub hotkey: String,
    /// 快速捕获快捷键（选中文本存为提示词）
    #[serde(default = "default_capture_hotkey")]
    pub capture_hotkey: String,
    #[serde(default = "default_true")]
    pub capture_clipboard: bool,
    /// 粘贴完成后恢复用户原来的剪贴板内容
    #[serde(default = "default_true")]
    pub restore_clipboard: bool,
    /// 粘贴后自动追加回车（适合网页 AI 对话框直接发送）
    #[serde(default)]
    pub paste_append_enter: bool,
    /// 界面主题："dark" | "light" | "auto"
    #[serde(default = "default_theme")]
    pub theme: String,
    /// 一次性主题迁移标记（uTools 风格改版：dark -> light）
    #[serde(default)]
    pub theme_migrated: bool,
    /// 一次性快捷键迁移标记（旧默认 alt+p -> alt+q）。
    /// 必须只执行一次：否则用户主动把主键设为 Alt+P 后会被每次启动强制改回
    #[serde(default)]
    pub hotkey_migrated: bool,
    #[serde(default)]
    pub webdav: WebDavConfig,
    #[serde(default)]
    pub gist: GistConfig,
    /// 当前使用的同步后端："webdav" | "gist"
    #[serde(default = "default_provider")]
    pub sync_provider: String,
    /// 云同步是否包含文本剪贴板历史。默认关闭：剪贴板里常出现密码等敏感内容，
    /// 上传到用户网盘/Gist 属于超出预期的数据外流，须显式开启
    #[serde(default)]
    pub sync_clipboard: bool,
}

fn default_hotkey() -> String {
    "alt+q".into()
}

/// 迁移：旧默认快捷键冲突、主题切换为 uTools 风格亮色（各自只执行一次）
pub fn migrate(data: &mut AppData) {
    if !data.settings.hotkey_migrated {
        if data.settings.hotkey.trim().to_lowercase() == "alt+p" {
            data.settings.hotkey = default_hotkey();
        }
        data.settings.hotkey_migrated = true;
    }
    if !data.settings.theme_migrated && data.settings.theme == "dark" {
        data.settings.theme = "light".into();
        data.settings.theme_migrated = true;
    }
}

fn default_capture_hotkey() -> String {
    "alt+s".into()
}

fn default_true() -> bool {
    true
}

fn default_theme() -> String {
    "light".into()
}

fn default_provider() -> String {
    "webdav".into()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: default_hotkey(),
            capture_hotkey: default_capture_hotkey(),
            capture_clipboard: true,
            restore_clipboard: true,
            paste_append_enter: false,
            theme: default_theme(),
            theme_migrated: false,
            hotkey_migrated: false,
            webdav: WebDavConfig::default(),
            gist: GistConfig::default(),
            sync_provider: default_provider(),
            sync_clipboard: false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppData {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub prompts: Vec<Prompt>,
    #[serde(default)]
    pub clipboard: Vec<ClipboardItem>,
    /// 已删除条目的墓碑，用于云同步时传播删除操作
    #[serde(default)]
    pub tombstones: Vec<Tombstone>,
    #[serde(default)]
    pub seeded: bool,
}

impl AppData {
    pub fn tombstone(&mut self, id: &str) {
        if id.is_empty() {
            return;
        }
        let at = now_ms();
        match self.tombstones.iter_mut().find(|t| t.id == id) {
            Some(t) => t.at = t.at.max(at),
            None => self.tombstones.push(Tombstone { id: id.to_string(), at }),
        }
        if self.tombstones.len() > 5000 {
            self.tombstones.drain(..self.tombstones.len() - 5000);
        }
    }

    pub fn ensure_category(&mut self, name: &str) {
        if !name.is_empty() && !self.categories.iter().any(|c| c == name) {
            self.categories.push(name.to_string());
        }
    }

    /// 首次启动时填充示例数据
    pub fn seed_if_empty(&mut self) {
        if self.seeded {
            return;
        }
        self.seeded = true;
        if self.categories.is_empty() {
            self.categories = ["设计", "开发", "写作"].iter().map(|s| s.to_string()).collect();
        }
        let now = now_ms();
        let samples: Vec<(&str, &str, &str, Vec<&str>)> = vec![
            (
                "UI 设计审查",
                "设计",
                "请从以下维度审查这个设计方案/页面，给出具体、可执行的改进建议：\n\n1. 视觉层级与信息架构\n2. 布局对齐、间距与节奏\n3. 配色与对比度（可读性）\n4. 交互与可用性问题\n5. 组件、文案与风格的一致性\n\n输出格式：按「问题 → 影响 → 建议」逐条列出。\n\n待审查内容：\n{{设计描述或截图说明}}",
                vec!["设计", "审查"],
            ),
            (
                "代码审查",
                "开发",
                "请审查以下代码，重点关注：\n\n1. 正确性与潜在 bug\n2. 边界条件与异常处理\n3. 性能问题\n4. 可读性与命名\n5. 安全隐患\n\n按严重程度（高/中/低）列出问题，并给出修改建议。\n\n```\n{{代码}}\n```",
                vec!["代码", "审查"],
            ),
            (
                "性能优化建议",
                "开发",
                "请针对以下代码/场景给出性能优化建议。每项建议说明：优化思路、预期收益、潜在风险与权衡，并按优先级排序。\n\n{{代码或场景描述}}",
                vec!["优化"],
            ),
        ];
        for (i, (title, category, content, tags)) in samples.into_iter().enumerate() {
            self.prompts.push(Prompt {
                id: new_id(),
                title: title.to_string(),
                content: content.to_string(),
                category: category.to_string(),
                tags: tags.into_iter().map(|s| s.to_string()).collect(),
                pinned: i == 0,
                hotkey: String::new(),
                use_count: 0,
                last_used_at: 0,
                created_at: now,
                updated_at: now,
            });
        }
    }
}

/// 云同步载荷（不含 settings，避免设备间互相覆盖配置）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPayload {
    pub version: u32,
    pub exported_at: u64,
    pub categories: Vec<String>,
    pub prompts: Vec<Prompt>,
    pub clipboard: Vec<ClipboardItem>,
    pub tombstones: Vec<Tombstone>,
}

impl From<&AppData> for SyncPayload {
    fn from(d: &AppData) -> Self {
        Self {
            version: d.version,
            exported_at: now_ms(),
            categories: d.categories.clone(),
            prompts: d.prompts.clone(),
            // 图片条目不参与云同步；文本条目仅在用户显式开启 sync_clipboard 时上云。
            // 双保险：含同步凭据原文的条目强制排除——用户复制 token/密码的动作
            // 会被剪贴板记录捕获，原文一旦随同步写进 Gist/网盘即视为泄漏
            clipboard: if d.settings.sync_clipboard {
                let secrets = [&d.settings.gist.token, &d.settings.webdav.password];
                d.clipboard
                    .iter()
                    .filter(|i| !i.is_image())
                    .filter(|i| {
                        secrets
                            .iter()
                            .all(|s| s.trim().is_empty() || !i.content.contains(s.trim()))
                    })
                    .cloned()
                    .collect()
            } else {
                Vec::new()
            },
            tombstones: d.tombstones.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    pub added: u64,
    pub updated: u64,
    pub removed: u64,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_prompt(id: &str, title: &str, category: &str, updated_at: u64) -> Prompt {
        Prompt {
            id: id.to_string(),
            title: title.to_string(),
            content: format!("content of {title}"),
            category: category.to_string(),
            tags: vec![],
            pinned: false,
            hotkey: String::new(),
            use_count: 0,
            last_used_at: 0,
            created_at: 1_000,
            updated_at,
        }
    }

    fn text_clip(id: &str, content: &str, copied_at: u64) -> ClipboardItem {
        ClipboardItem {
            id: id.to_string(),
            content: content.to_string(),
            copied_at,
            kind: "text".into(),
            image: None,
        }
    }

    fn image_clip(id: &str, copied_at: u64) -> ClipboardItem {
        ClipboardItem {
            id: id.to_string(),
            content: String::new(),
            copied_at,
            kind: "image".into(),
            image: Some(ImageRef {
                file: format!("{id}.png"),
                width: 10,
                height: 10,
            }),
        }
    }

    // ---------- migrate：旧默认值迁移必须只执行一次 ----------

    #[test]
    fn migrate_moves_legacy_alt_p_hotkey_once() {
        let mut data = AppData {
            settings: Settings {
                hotkey: "alt+p".into(),
                ..Settings::default()
            },
            ..AppData::default()
        };
        migrate(&mut data);
        assert_eq!(data.settings.hotkey, "alt+q");
        assert!(data.settings.hotkey_migrated);

        // 迁移后用户主动把主键改回 alt+p，不得被再次强制改掉
        data.settings.hotkey = "Alt+P".into();
        migrate(&mut data);
        assert_eq!(data.settings.hotkey, "Alt+P");
    }

    #[test]
    fn migrate_theme_dark_to_light_once() {
        let mut data = AppData {
            settings: Settings {
                theme: "dark".into(),
                ..Settings::default()
            },
            ..AppData::default()
        };
        migrate(&mut data);
        assert_eq!(data.settings.theme, "light");
        assert!(data.settings.theme_migrated);

        data.settings.theme = "dark".into();
        migrate(&mut data);
        assert_eq!(data.settings.theme, "dark");
    }

    #[test]
    fn migrate_leaves_nonlegacy_values_untouched() {
        let mut data = AppData {
            settings: Settings {
                hotkey: "ctrl+shift+space".into(),
                theme: "auto".into(),
                ..Settings::default()
            },
            ..AppData::default()
        };
        migrate(&mut data);
        assert_eq!(data.settings.hotkey, "ctrl+shift+space");
        assert_eq!(data.settings.theme, "auto");
    }

    // ---------- tombstone ----------

    #[test]
    fn tombstone_rejects_empty_id() {
        let mut data = AppData::default();
        data.tombstone("");
        assert!(data.tombstones.is_empty());
    }

    #[test]
    fn tombstone_keeps_max_at_per_id() {
        let mut data = AppData::default();
        data.tombstones.push(Tombstone {
            id: "a".into(),
            at: 100,
        });
        let before = now_ms();
        // now_ms() 不小于手工设置的 100，重复删除同一条取较新时间
        data.tombstone("a");
        assert_eq!(data.tombstones.len(), 1);
        assert!(data.tombstones[0].at >= before, "应取 now 与旧值中较大的");
    }

    #[test]
    fn tombstone_cap_at_5000_drops_oldest() {
        let mut data = AppData::default();
        for i in 0..5000u64 {
            data.tombstones.push(Tombstone {
                id: format!("old-{i}"),
                at: i,
            });
        }
        data.tombstone("newest");
        assert_eq!(data.tombstones.len(), 5000);
        assert!(!data.tombstones.iter().any(|t| t.id == "old-0"));
        assert!(data.tombstones.last().unwrap().id == "newest");
    }

    // ---------- ensure_category / seed_if_empty ----------

    #[test]
    fn ensure_category_skips_empty_and_duplicates() {
        let mut data = AppData::default();
        data.ensure_category("开发");
        data.ensure_category("开发");
        data.ensure_category("");
        assert_eq!(data.categories, vec!["开发".to_string()]);
    }

    #[test]
    fn seed_if_empty_fills_samples_and_is_idempotent() {
        let mut data = AppData::default();
        data.seed_if_empty();
        assert!(data.seeded);
        assert_eq!(data.categories.len(), 3);
        assert_eq!(data.prompts.len(), 3);
        assert!(data.prompts[0].pinned, "首个示例应置顶");

        let titles_before: Vec<String> = data.prompts.iter().map(|p| p.title.clone()).collect();
        data.seed_if_empty();
        let titles_after: Vec<String> = data.prompts.iter().map(|p| p.title.clone()).collect();
        assert_eq!(titles_before, titles_after, "重复调用不得追加示例");
    }

    #[test]
    fn seed_if_empty_keeps_existing_categories() {
        let mut data = AppData {
            categories: vec!["我的分类".into()],
            ..AppData::default()
        };
        data.seed_if_empty();
        assert_eq!(data.categories, vec!["我的分类".to_string()]);
    }

    // ---------- SyncPayload：剪贴板出云范围（安全行为） ----------

    fn payload_settings() -> Settings {
        Settings {
            sync_clipboard: true,
            gist: GistConfig {
                token: "ghp_secret_token".into(),
                ..GistConfig::default()
            },
            webdav: WebDavConfig {
                password: "dav_pass".into(),
                ..WebDavConfig::default()
            },
            ..Settings::default()
        }
    }

    #[test]
    fn sync_payload_excludes_clipboard_when_disabled() {
        let mut data = AppData::default();
        data.settings.sync_clipboard = false;
        data.clipboard.push(text_clip("c1", "hello", 1));
        let payload = SyncPayload::from(&data);
        assert!(payload.clipboard.is_empty());
    }

    #[test]
    fn sync_payload_includes_text_excludes_images() {
        let mut data = AppData::default();
        data.settings = payload_settings();
        data.clipboard.push(text_clip("c1", "hello", 1));
        data.clipboard.push(image_clip("c2", 2));
        let payload = SyncPayload::from(&data);
        assert_eq!(payload.clipboard.len(), 1);
        assert_eq!(payload.clipboard[0].id, "c1");
    }

    #[test]
    fn sync_payload_never_leaks_sync_credentials() {
        let mut data = AppData::default();
        data.settings = payload_settings();
        data.clipboard.push(text_clip("c1", "my ghp_secret_token leaked", 1));
        data.clipboard.push(text_clip("c2", "password is dav_pass ok", 2));
        data.clipboard.push(text_clip("c3", "clean text", 3));
        let payload = SyncPayload::from(&data);
        let ids: Vec<&str> = payload.clipboard.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(ids, vec!["c3"], "含同步凭据原文的条目必须被强制排除");
    }

    #[test]
    fn sync_payload_carries_prompts_categories_tombstones() {
        let mut data = AppData::default();
        data.categories = vec!["开发".into()];
        data.prompts.push(sample_prompt("p1", "标题", "开发", 10));
        data.tombstones.push(Tombstone { id: "x".into(), at: 5 });
        let payload = SyncPayload::from(&data);
        assert_eq!(payload.prompts.len(), 1);
        assert_eq!(payload.prompts[0].id, "p1");
        assert_eq!(payload.categories, vec!["开发".to_string()]);
        assert_eq!(payload.tombstones.len(), 1);
    }

    // ---------- serde 向后兼容：旧版 data.json 缺新字段 ----------

    #[test]
    fn old_data_json_deserializes_with_defaults() {
        // 模拟早期版本的数据文件：没有 tags/pinned/hotkey/use_count 等字段
        let raw = r#"{
            "version": 1,
            "settings": { "hotkey": "alt+q" },
            "categories": ["开发"],
            "prompts": [
                { "id": "p1", "title": "旧条目", "content": "正文",
                  "category": "开发", "createdAt": 111, "updatedAt": 222 }
            ]
        }"#;
        let data: AppData = serde_json::from_str(raw).expect("旧格式必须可解析");
        let p = &data.prompts[0];
        assert_eq!(p.title, "旧条目");
        assert!(p.tags.is_empty());
        assert!(!p.pinned);
        assert_eq!(p.hotkey, "");
        assert_eq!(p.use_count, 0);
        assert_eq!(p.last_used_at, 0);
        assert_eq!(p.created_at, 111);
        assert_eq!(p.updated_at, 222);
        assert!(!data.seeded);
        assert_eq!(data.settings.sync_provider, "webdav");
        assert!(!data.settings.sync_clipboard);
    }

    #[test]
    fn clipboard_item_defaults_kind_text() {
        let raw = r#"{ "id": "c1", "content": "hi", "copiedAt": 5 }"#;
        let item: ClipboardItem = serde_json::from_str(raw).expect("缺 kind 必须可解析");
        assert_eq!(item.kind, "text");
        assert!(!item.is_image());
    }
}

