use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::models::{now_ms, Prompt};

/// 统一注册全部全局快捷键：面板主键 + 快速捕获键 + 各提示词的独立快捷键。
/// 任一注册失败只记录日志（其余键继续生效）。
pub fn register_all(app: &AppHandle) {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();

    let (main_hotkey, capture_hotkey) = {
        let store = crate::store::lock(app);
        (
            store.data.settings.hotkey.clone(),
            store.data.settings.capture_hotkey.clone(),
        )
    };

    if let Err(e) = gs.on_shortcut(normalize(&main_hotkey).as_str(), |app, _s, event| {
        if event.state() == ShortcutState::Pressed {
            toggle_quick_window(app);
        }
    }) {
        eprintln!("[promptmate] 注册主快捷键 {} 失败: {e}", normalize(&main_hotkey));
    }

    if !capture_hotkey.trim().is_empty() {
        let accel = normalize(&capture_hotkey);
        if let Err(e) = gs.on_shortcut(accel.as_str(), |app, _s, event| {
            if event.state() == ShortcutState::Pressed {
                crate::capture::start(app);
            }
        }) {
            eprintln!("[promptmate] 注册捕获快捷键 {accel} 失败: {e}");
        }
    }

    let hotkeyed: Vec<(String, String)> = {
        let store = crate::store::lock(app);
        store
            .data
            .prompts
            .iter()
            .filter(|p| !p.hotkey.trim().is_empty())
            .map(|p| (normalize(&p.hotkey), p.id.clone()))
            .collect()
    };
    for (accel, id) in hotkeyed {
        if let Err(e) = gs.on_shortcut(accel.as_str(), move |app, _s, event| {
            if event.state() == ShortcutState::Pressed {
                trigger_prompt(app, &id);
            }
        }) {
            eprintln!("[promptmate] 注册提示词快捷键 {accel} 失败（可能被占用或重复）: {e}");
        }
    }
}

/// 触发提示词：无变量直接后台粘贴；有变量则呼出面板并让前端弹出变量填写窗
pub fn trigger_prompt(app: &AppHandle, id: &str) {
    let prompt: Option<Prompt> = {
        let store = crate::store::lock(app);
        store.data.prompts.iter().find(|p| p.id == id).cloned()
    };
    let Some(p) = prompt else { return };

    if has_vars(&p.content) {
        show_quick_window(app);
        let _ = app.emit("open-prompt", p.id);
        return;
    }

    // 无变量：记录前台窗口后直接粘贴（面板保持隐藏）
    #[cfg(windows)]
    {
        let fg = crate::paste::foreground::current();
        let mut store = crate::store::lock(app);
        store.paste_target = fg;
    }
    {
        let mut store = crate::store::lock(app);
        let _ = store.mutate(|d| {
            if let Some(x) = d.prompts.iter_mut().find(|x| x.id == id) {
                x.use_count += 1;
                x.last_used_at = crate::models::now_ms();
            }
        });
    }
    if let Err(e) = crate::paste::paste_text(app, &p.content) {
        eprintln!("[promptmate] 快捷键粘贴失败: {e}");
    }
}

/// 与前端 vars.ts 一致的 {{变量}} 宽松检测（排除 {{clipboard}} 自动变量）
fn has_vars(content: &str) -> bool {
    let mut rest = content;
    while let Some(i) = rest.find("{{") {
        let after = &rest[i + 2..];
        let Some(j) = after.find("}}") else {
            return false;
        };
        let name = after[..j].split('|').next().unwrap_or("").trim().to_lowercase();
        if !name.is_empty() && name != "clipboard" {
            return true;
        }
        rest = &after[j + 2..];
    }
    false
}

/// 导入 / 云同步合并后清理提示词快捷键：
/// 与主键、捕获键或先注册的提示词冲突的键一律清空并 bump updated_at（让清理结果同步出去）。
/// 导入路径没有 save_prompt 的 validate_prompt_hotkey 卡口，必须在此兜底，
/// 否则分享的提示词包可静默抢占/挤掉全局快捷键。
pub fn sanitize_prompt_hotkeys(data: &mut crate::models::AppData) {
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let main = normalize(&data.settings.hotkey);
    if !main.is_empty() {
        seen.insert(main);
    }
    let capture = normalize(&data.settings.capture_hotkey);
    if !capture.is_empty() {
        seen.insert(capture);
    }
    for p in data.prompts.iter_mut() {
        if p.hotkey.trim().is_empty() {
            continue;
        }
        let n = normalize(&p.hotkey);
        if n.is_empty() || !seen.insert(n) {
            p.hotkey = String::new();
            p.updated_at = now_ms();
        }
    }
}

/// 在鼠标当前所在显示器上居中显示快捷面板并聚焦
pub fn show_quick_window(app: &AppHandle) {
    let Some(win) = app.get_webview_window("main") else {
        eprintln!("[promptmate] show_quick_window: main 窗口不存在");
        return;
    };
    let already_visible = win.is_visible().unwrap_or(false);

    // 记录呼出前的前台窗口，供粘贴时恢复焦点
    if !already_visible {
        #[cfg(windows)]
        {
            let fg = crate::paste::foreground::current();
            let mut store = crate::store::lock(app);
            // 前台不是面板自身时才记录
            if fg.map_or(true, |h| Some(h) != win.hwnd().ok().map(|w| w.0 as isize)) {
                store.paste_target = fg;
            }
        }
    }

    let _ = win.unminimize();

    // 让面板出现在光标所在屏幕
    if let Ok(pos) = app.cursor_position() {
        if let Ok(Some(monitor)) = win.monitor_from_point(pos.x, pos.y) {
            let size = win.outer_size().unwrap_or(tauri::PhysicalSize::new(760, 520));
            let mp = monitor.position();
            let ms = monitor.size();
            let x = (mp.x + (ms.width as i32 - size.width as i32) / 2).max(mp.x + 8);
            let y = (mp.y + (ms.height as i32 - size.height as i32) / 3).max(mp.y + 8);
            let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
        }
    } else {
        let _ = win.center();
    }

    let _ = win.show();
    let _ = win.set_focus();
    // 直接向前端注入 DOM 事件重置会话（比 emit 更可靠，无 IPC 时序问题）：
    // 每次呼出都是全新的搜索（清空关键词/模式/选中项）
    let _ = win.eval("window.dispatchEvent(new CustomEvent('pm-panel-shown'));");
}

pub fn toggle_quick_window(app: &AppHandle) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    let visible = win.is_visible().unwrap_or(false);
    let focused = win.is_focused().unwrap_or(false);
    if visible && focused {
        let _ = win.hide();
    } else {
        show_quick_window(app);
    }
}

pub fn open_manager_window(app: &AppHandle) {
    let Some(win) = app.get_webview_window("manager") else {
        return;
    };
    let _ = win.unminimize();
    let _ = win.show();
    let _ = win.set_focus();
}

/// 校验提示词快捷键：与面板主键 / 捕获键 / 其他提示词不得重复
pub fn validate_prompt_hotkey(app: &AppHandle, prompt_id: &str, hotkey: &str) -> Result<(), String> {
    let hk = hotkey.trim();
    if hk.is_empty() {
        return Ok(());
    }
    let normalized = normalize(hk);
    let (main_hotkey, capture_hotkey) = {
        let store = crate::store::lock(app);
        (
            store.data.settings.hotkey.clone(),
            store.data.settings.capture_hotkey.clone(),
        )
    };
    if normalized == normalize(&main_hotkey) {
        return Err("该快捷键已被「呼出面板」占用".into());
    }
    if !capture_hotkey.trim().is_empty() && normalized == normalize(&capture_hotkey) {
        return Err("该快捷键已被「快速捕获」占用".into());
    }
    let conflict = {
        let store = crate::store::lock(app);
        store
            .data
            .prompts
            .iter()
            .find(|p| {
                p.id != prompt_id
                    && !p.hotkey.trim().is_empty()
                    && normalize(&p.hotkey) == normalized
            })
            .map(|p| p.title.clone())
    };
    if let Some(title) = conflict {
        return Err(format!("该快捷键已被提示词「{}」占用", title));
    }
    Ok(())
}

/// 统一 accelerator 大小写格式，如 "Alt+P" -> "alt+p"
fn normalize(s: &str) -> String {
    let parts: Vec<String> = s
        .split('+')
        .map(|p| p.trim().to_lowercase())
        .filter(|p| !p.is_empty())
        .collect();
    parts.join("+")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::AppData;

    fn prompt_with_hotkey(id: &str, hotkey: &str, updated_at: u64) -> crate::models::Prompt {
        crate::models::Prompt {
            id: id.to_string(),
            title: format!("提示词-{id}"),
            content: "正文".into(),
            category: "开发".into(),
            tags: vec![],
            pinned: false,
            hotkey: hotkey.to_string(),
            use_count: 0,
            last_used_at: 0,
            created_at: 1,
            updated_at,
        }
    }

    // ---------- normalize ----------

    #[test]
    fn normalize_unifies_case_and_whitespace() {
        assert_eq!(normalize("Alt+P"), "alt+p");
        assert_eq!(normalize("  Ctrl + Shift + A "), "ctrl+shift+a");
        assert_eq!(normalize("ALT+Q"), "alt+q");
    }

    #[test]
    fn normalize_drops_empty_segments() {
        assert_eq!(normalize("alt++p"), "alt+p");
        assert_eq!(normalize("++"), "");
        assert_eq!(normalize(""), "");
    }

    // ---------- sanitize_prompt_hotkeys ----------

    #[test]
    fn sanitize_clears_prompt_hotkey_conflicting_with_main_key() {
        let mut data = AppData::default(); // 默认主键 alt+q
        data.prompts.push(prompt_with_hotkey("p1", "Alt+Q", 5));
        data.prompts.push(prompt_with_hotkey("p2", "ctrl+k", 5));

        sanitize_prompt_hotkeys(&mut data);
        let p1 = data.prompts.iter().find(|p| p.id == "p1").unwrap();
        let p2 = data.prompts.iter().find(|p| p.id == "p2").unwrap();
        assert!(p1.hotkey.is_empty(), "与主键冲突（归一化后比较）必须清空");
        assert!(p1.updated_at > 5, "清理结果必须 bump updated_at 才能同步出去");
        assert_eq!(p2.hotkey, "ctrl+k", "无冲突的快捷键必须保留");
    }

    #[test]
    fn sanitize_clears_prompt_hotkey_conflicting_with_capture_key() {
        let mut data = AppData::default(); // 默认捕获键 alt+s
        data.prompts.push(prompt_with_hotkey("p1", "  ALT + S ", 5));
        sanitize_prompt_hotkeys(&mut data);
        assert!(data.prompts[0].hotkey.is_empty());
    }

    #[test]
    fn sanitize_duplicate_prompt_hotkeys_first_wins() {
        let mut data = AppData::default();
        data.prompts.push(prompt_with_hotkey("p1", "ctrl+j", 5));
        data.prompts.push(prompt_with_hotkey("p2", "Ctrl+J", 5));
        data.prompts.push(prompt_with_hotkey("p3", " ctrl+j ", 5));

        sanitize_prompt_hotkeys(&mut data);
        assert_eq!(data.prompts[0].hotkey, "ctrl+j", "先注册者保留");
        assert!(data.prompts[1].hotkey.is_empty(), "后到的重复键清空");
        assert!(data.prompts[2].hotkey.is_empty(), "仅空白差异的等价形式同样算重复");
    }

    #[test]
    fn sanitize_leaves_empty_hotkeys_untouched() {
        let mut data = AppData::default();
        data.prompts.push(prompt_with_hotkey("p1", "   ", 7));
        sanitize_prompt_hotkeys(&mut data);
        assert_eq!(data.prompts[0].updated_at, 7, "空快捷键不应触发 updated_at 变化");
    }

    // ---------- has_vars：与前端 vars.ts 的行为一致性 ----------

    #[test]
    fn rust_has_vars_matches_frontend_semantics() {
        assert!(!has_vars("纯文本"));
        assert!(!has_vars(""), "空内容无变量");
        assert!(!has_vars("{{}}"), "空变量名不算");
        assert!(!has_vars("{{clipboard}}"), "自动变量不算手动变量");
        assert!(!has_vars("{{ ClipBoard }}"), "自动变量忽略大小写与空白");
        assert!(!has_vars("{{未闭合"), "缺少闭合不算");
        assert!(has_vars("{{名字}}"));
        assert!(has_vars("前缀 {{ name | 提示语 }} 后缀"));
        assert!(has_vars("{{a|hint}}"));
    }
}
