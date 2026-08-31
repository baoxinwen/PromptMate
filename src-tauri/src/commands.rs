use tauri::{AppHandle, Emitter, Manager, WebviewWindow, Wry};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::models::{new_id, now_ms, AppData, Prompt, Settings, SyncReport};
use crate::store::lock;
use crate::transfer;
use crate::{hotkey, paste, sync};

fn emit_data_changed(app: &AppHandle) {
    let _ = app.emit("data-changed", ());
}

// ---------- 数据读取 / 提示词 ----------

#[tauri::command]
pub fn get_data(app: AppHandle) -> AppData {
    lock(&app).data.clone()
}

#[tauri::command]
pub fn save_prompt(app: AppHandle, mut prompt: Prompt) -> Result<(), String> {
    hotkey::validate_prompt_hotkey(&app, &prompt.id, &prompt.hotkey)?;
    let mut store = lock(&app);
    prompt.updated_at = now_ms();
    if prompt.title.trim().is_empty() {
        prompt.title = "未命名提示词".into();
    }
    // 空分类兜底：否则该提示词不出现在任何分类下，
    // 且 export_markdown（按 categories 遍历）会把它静默丢掉
    if prompt.category.trim().is_empty() {
        prompt.category = "未分类".into();
    }
    let id = prompt.id.clone();
    if id.is_empty() {
        prompt.id = new_id();
        prompt.created_at = now_ms();
        let p = prompt.clone();
        store.mutate(|d| {
            d.ensure_category(&p.category);
            d.prompts.push(p);
        })?;
    } else {
        let p = prompt.clone();
        store.mutate(|d| match d.prompts.iter_mut().find(|x| x.id == id) {
            Some(existing) => {
                let created = existing.created_at;
                let use_count = existing.use_count;
                let last_used = existing.last_used_at;
                *existing = p.clone();
                existing.created_at = created;
                existing.use_count = use_count;
                existing.last_used_at = last_used;
                d.ensure_category(&p.category);
            }
            None => d.prompts.push(p.clone()),
        })?;
    }
    drop(store);
    hotkey::register_all(&app);
    emit_data_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn delete_prompt(app: AppHandle, id: String) -> Result<(), String> {
    {
        let mut store = lock(&app);
        store.mutate(|d| {
            d.prompts.retain(|p| p.id != id);
            d.tombstone(&id);
        })?;
    }
    hotkey::register_all(&app);
    emit_data_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn record_prompt_use(app: AppHandle, id: String) -> Result<(), String> {
    let mut store = lock(&app);
    store.mutate(|d| {
        if let Some(p) = d.prompts.iter_mut().find(|p| p.id == id) {
            p.use_count += 1;
            p.last_used_at = now_ms();
        }
    })?;
    emit_data_changed(&app);
    Ok(())
}

// ---------- 分类 ----------

#[tauri::command]
pub fn add_category(app: AppHandle, name: String) -> Result<(), String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("分类名不能为空".into());
    }
    let mut store = lock(&app);
    store.mutate(|d| d.ensure_category(&name))?;
    emit_data_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn rename_category(app: AppHandle, old_name: String, new_name: String) -> Result<(), String> {
    let new_name = new_name.trim().to_string();
    if new_name.is_empty() {
        return Err("分类名不能为空".into());
    }
    let mut store = lock(&app);
    if store.data.categories.iter().any(|c| *c == new_name) {
        return Err("该分类名已存在".into());
    }
    store.mutate(|d| {
        for c in d.categories.iter_mut() {
            if *c == old_name {
                *c = new_name.clone();
            }
        }
        for p in d.prompts.iter_mut() {
            if p.category == old_name {
                p.category = new_name.clone();
                p.updated_at = now_ms();
            }
        }
    })?;
    drop(store);
    emit_data_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn delete_category(app: AppHandle, name: String) -> Result<(), String> {
    let mut store = lock(&app);
    store.mutate(|d| {
        d.categories.retain(|c| *c != name);
        d.ensure_category("未分类");
        for p in d.prompts.iter_mut() {
            if p.category == name {
                p.category = "未分类".into();
                p.updated_at = now_ms();
            }
        }
    })?;
    emit_data_changed(&app);
    Ok(())
}

// ---------- 剪贴板 / 粘贴 ----------

#[tauri::command]
pub fn copy_text(app: AppHandle, text: String) -> Result<(), String> {
    {
        let mut store = lock(&app);
        store.suppress_clipboard = true;
    }
    // 写入失败必须解除抑制，否则剪贴板历史从此静默失效直到重启
    if let Err(e) = paste::set_clipboard_text(&text) {
        lock(&app).suppress_clipboard = false;
        return Err(e);
    }
    let h = app.clone();
    std::thread::spawn(move || {
        // 覆盖一个完整的剪贴板轮询周期(700ms)：监听线程至少采样一次并
        // 把本次写入吸收进基线，解除抑制后才不会把程序自己的复制记进历史
        std::thread::sleep(std::time::Duration::from_millis(800));
        lock(&h).suppress_clipboard = false;
    });
    Ok(())
}

#[tauri::command]
pub fn invoke_paste(
    app: AppHandle,
    window: WebviewWindow,
    text: String,
    prompt_id: Option<String>,
) -> Result<(), String> {
    if let Some(id) = prompt_id {
        let mut store = lock(&app);
        let _ = store.mutate(|d| {
            if let Some(p) = d.prompts.iter_mut().find(|p| p.id == id) {
                p.use_count += 1;
                p.last_used_at = now_ms();
            }
        });
    }
    paste::paste_to_previous_window(&window, &app, &text)
}

#[tauri::command]
pub fn paste_text_direct(app: AppHandle, window: WebviewWindow, text: String) -> Result<(), String> {
    paste::paste_to_previous_window(&window, &app, &text)
}

/// 读取当前剪贴板文本（供 {{clipboard}} 自动变量使用）
#[tauri::command]
pub fn get_clipboard_text() -> Option<String> {
    paste::get_clipboard_text()
}

#[tauri::command]
pub fn get_var_memory(
    app: tauri::AppHandle<Wry>,
    prompt_id: String,
) -> std::collections::BTreeMap<String, String> {
    crate::var_memory::get_var_memory(app, prompt_id)
}

#[tauri::command]
pub fn save_var_memory(
    app: tauri::AppHandle<Wry>,
    prompt_id: String,
    values: std::collections::BTreeMap<String, String>,
) -> Result<(), String> {
    crate::var_memory::save_var_memory(app, prompt_id, values)
}

// ---------- 剪贴板图片 ----------

#[tauri::command]
pub fn get_image_thumb(app: AppHandle, id: String) -> Result<String, String> {
    let file = lock(&app)
        .data
        .clipboard
        .iter()
        .find(|i| i.id == id)
        .and_then(|i| i.image.as_ref())
        .map(|im| im.file.clone())
        .ok_or("图片不存在")?;
    let bytes = crate::images::read_png(&app, &crate::images::thumb_name(&file))?;
    Ok(crate::images::png_base64(&bytes))
}

#[tauri::command]
pub fn paste_image(app: AppHandle, window: WebviewWindow, id: String) -> Result<(), String> {
    let file = lock(&app)
        .data
        .clipboard
        .iter()
        .find(|i| i.id == id)
        .and_then(|i| i.image.as_ref())
        .map(|im| im.file.clone())
        .ok_or("图片不存在")?;

    let bytes = crate::images::read_png(&app, &file)?;
    let img = image::load_from_memory(&bytes).map_err(|e| format!("解码图片失败: {e}"))?;
    let rgba = img.to_rgba8();

    {
        let mut store = lock(&app);
        store.suppress_clipboard = true;
    }
    // 写入失败必须解除抑制，否则剪贴板历史从此静默失效直到重启
    if let Err(e) = arboard::Clipboard::new().and_then(|mut c| {
        c.set_image(arboard::ImageData {
            width: rgba.width() as usize,
            height: rgba.height() as usize,
            bytes: std::borrow::Cow::Owned(rgba.into_raw()),
        })
    }) {
        lock(&app).suppress_clipboard = false;
        return Err(format!("写入图片剪贴板失败: {e}"));
    }

    let _ = window.hide();
    let append_enter = lock(&app).data.settings.paste_append_enter;
    let handle = app.clone();
    std::thread::spawn(move || {
        let target = lock(&handle).paste_target.take();
        paste::send_paste(target, append_enter);
        // 覆盖一个轮询周期(700ms)，避免图片写入/粘贴动作被记入剪贴板历史
        std::thread::sleep(std::time::Duration::from_millis(750));
        lock(&handle).suppress_clipboard = false;
    });
    Ok(())
}

#[tauri::command]
pub fn delete_history_item(app: AppHandle, id: String) -> Result<(), String> {
    let file = {
        let mut store = lock(&app);
        let file = store
            .data
            .clipboard
            .iter()
            .find(|i| i.id == id)
            .and_then(|i| i.image.as_ref())
            .map(|im| im.file.clone());
        store.mutate(|d| {
            d.clipboard.retain(|i| i.id != id);
            d.tombstone(&id);
        })?;
        file
    };
    if let Some(f) = file {
        if let Some(base) = f.strip_suffix(".png") {
            crate::images::delete_files(&app, base);
        }
    }
    emit_data_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn clear_history(app: AppHandle) -> Result<(), String> {
    let image_ids: Vec<String> = {
        let store = lock(&app);
        store
            .data
            .clipboard
            .iter()
            .filter(|i| i.is_image())
            .map(|i| i.id.clone())
            .collect()
    };
    {
        let mut store = lock(&app);
        let ids: Vec<String> = store.data.clipboard.iter().map(|i| i.id.clone()).collect();
        store.mutate(|d| {
            d.clipboard.clear();
            for id in ids {
                d.tombstone(&id);
            }
        })?;
    }
    for id in image_ids {
        crate::images::delete_files(&app, &id);
    }
    emit_data_changed(&app);
    Ok(())
}

// ---------- 窗口控制 ----------

#[tauri::command]
pub fn hide_quick(window: WebviewWindow) {
    let _ = window.hide();
}

#[tauri::command]
pub fn open_manager(app: AppHandle) {
    hotkey::open_manager_window(&app);
}

/// 启动时数据文件损坏的恢复提示（取后即清）。
/// 不用事件推送：窗口刚创建时前端尚未注册监听，事件必然丢失
#[tauri::command]
pub fn get_recovery_notice(app: AppHandle) -> Option<String> {
    lock(&app).recovered_notice.take()
}

// ---------- 设置 ----------

#[tauri::command]
pub fn close_capture(app: AppHandle) {
    crate::capture::hide(&app);
}

/// 快捷面板高度自适应：前端测量内容高度后调用，窗口随之伸缩
#[tauri::command]
pub fn set_panel_height(app: AppHandle, height: f64) {
    use tauri::LogicalSize;
    if let Some(w) = app.get_webview_window("main") {
        let h = height.clamp(300.0, 640.0);
        let _ = w.set_size(LogicalSize::new(760.0_f64, h));
    }
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Settings) -> Result<(), String> {
    let (old_hotkey, old_capture_hotkey) = {
        let store = lock(&app);
        (
            store.data.settings.hotkey.clone(),
            store.data.settings.capture_hotkey.clone(),
        )
    };
    {
        let mut store = lock(&app);
        store.mutate(|d| d.settings = settings.clone())?;
    }
    // 任一全局快捷键变化都需要重注册
    if settings.hotkey.trim().to_lowercase() != old_hotkey.trim().to_lowercase()
        || settings.capture_hotkey.trim().to_lowercase() != old_capture_hotkey.trim().to_lowercase()
    {
        hotkey::register_all(&app);
    }
    emit_data_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn get_autostart(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enable: bool) -> Result<(), String> {
    let launcher = app.autolaunch();
    if enable {
        launcher.enable().map_err(|e| e.to_string())
    } else {
        launcher.disable().map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn open_data_dir(app: AppHandle) -> Result<(), String> {
    let dir = {
        let store = lock(&app);
        store.data_dir()
    };
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| format!("打开目录失败: {e}"))
}

// ---------- 导入 / 导出 ----------

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub added: usize,
    pub skipped: usize,
    pub message: String,
}

#[tauri::command]
pub async fn export_data(app: AppHandle, kind: String, include_clipboard: bool) -> Result<String, String> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let data = lock(&handle).data.clone();
        let stamp = time_stamp();
        let (filter_name, filter_ext, default_name) = match kind.as_str() {
            "markdown" => (
                "Markdown",
                vec!["md"],
                format!("promptmate-{stamp}.md"),
            ),
            _ => ("JSON", vec!["json"], format!("promptmate-{stamp}.json")),
        };

        let file = handle
            .dialog()
            .file()
            .add_filter(filter_name, &filter_ext)
            .set_file_name(&default_name)
            .blocking_save_file();

        let Some(file_path) = file else {
            return Ok(String::new());
        };
        let path = file_path.into_path().map_err(|e| e.to_string())?;

        let content = match kind.as_str() {
            "markdown" => transfer::export_markdown(&data),
            _ => transfer::export_json(&data, include_clipboard),
        };
        std::fs::write(&path, content).map_err(|e| format!("写入文件失败: {e}"))?;
        Ok(path.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn import_data(app: AppHandle) -> Result<ImportSummary, String> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let files = handle
            .dialog()
            .file()
            .add_filter("提示词文件", &["json", "md", "markdown", "txt"])
            .blocking_pick_files();

        let Some(files) = files else {
            return Ok(ImportSummary {
                added: 0,
                skipped: 0,
                message: "已取消".into(),
            });
        };

        let paths: Vec<std::path::PathBuf> = files
            .into_iter()
            .filter_map(|f| f.into_path().ok())
            .collect();
        import_from_files(&handle, &paths)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 拖拽导入：直接传入文件路径列表
#[tauri::command]
pub async fn import_paths(app: AppHandle, paths: Vec<String>) -> Result<ImportSummary, String> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        import_from_files(&handle, &paths.iter().map(std::path::PathBuf::from).collect::<Vec<_>>())
    })
    .await
    .map_err(|e| e.to_string())?
}

fn import_from_files(handle: &AppHandle, files: &[std::path::PathBuf]) -> Result<ImportSummary, String> {
    let mut total_added = 0usize;
    let mut total_skipped = 0usize;
    let mut errors: Vec<String> = Vec::new();

    for path in files {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => {
                errors.push(format!("{}: {e}", path.display()));
                continue;
            }
        };
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        let mut store = lock(handle);
        let result: Result<(usize, usize), String> = match ext.as_str() {
            "json" => transfer::import_json(&mut store.data, &text),
            "md" | "markdown" => {
                let (a, s) = transfer::import_markdown(&mut store.data, &text, "未分类");
                Ok((a, s))
            }
            "txt" => {
                let title = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "导入文本".into());
                if transfer::import_text(&mut store.data, &title, &text) {
                    Ok((1, 0))
                } else {
                    Ok((0, 1))
                }
            }
            other => Err(format!("不支持的格式: {other}")),
        };
        match result {
            Ok((a, s)) => {
                total_added += a;
                total_skipped += s;
                // 导入不走 save_prompt 的快捷键校验，这里统一清理冲突
                hotkey::sanitize_prompt_hotkeys(&mut store.data);
                let _ = store.save();
            }
            Err(e) => errors.push(format!("{}: {e}", path.display())),
        }
    }

    if total_added > 0 {
        emit_data_changed(handle);
    }
    let mut message = format!("导入完成：新增 {total_added} 条，跳过 {total_skipped} 条");
    if !errors.is_empty() {
        message.push_str(&format!("，{} 个文件失败", errors.len()));
    }
    Ok(ImportSummary {
        added: total_added,
        skipped: total_skipped,
        message,
    })
}

fn time_stamp() -> String {
    let ms = now_ms();
    let secs = ms / 1000;
    let days = secs / 86400;
    // 简单的 UTC 日期格式化，足够用于文件名
    let (y, mo, d) = civil_from_days(days as i64);
    format!("{y}{mo:02}{d:02}")
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

// ---------- 同步 ----------

#[tauri::command]
pub async fn webdav_test(url: String, username: String, password: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || sync::test_connection(&url, &username, &password))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn gist_test(token: String, gist_id: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || sync::gist_test(&token, &gist_id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn sync_now(app: AppHandle, direction: String) -> Result<SyncReport, String> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || sync::run_sync(&handle, &direction))
        .await
        .map_err(|e| e.to_string())?
}
