use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;

use tauri::{AppHandle, Emitter};

use crate::models::{new_id, now_ms, ClipboardItem, ImageRef, MAX_CLIPBOARD_ITEMS};
use crate::store::lock;

const POLL_INTERVAL: Duration = Duration::from_millis(700);
const MAX_IMAGES: usize = 50;

/// 多点采样哈希：只看首尾 32 字节时，中部内容不同的两张图（如仅改中部的截图）
/// 会被误判为同一张而漏记历史
fn image_hash(width: usize, height: usize, data: &[u8]) -> u64 {
    let mut h = DefaultHasher::new();
    width.hash(&mut h);
    height.hash(&mut h);
    data.len().hash(&mut h);
    let n = data.len();
    if n <= 192 {
        data.hash(&mut h);
    } else {
        // 取首/1/4/中/3/4/尾五段各 64 字节
        for start in [0, n / 4, n / 2, 3 * n / 4, n - 64] {
            data[start..start + 64].hash(&mut h);
        }
    }
    h.finish()
}

/// 后台轮询剪贴板，把系统内新复制的内容记入历史（文本与图片）
pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || {
        let Ok(mut board) = arboard::Clipboard::new() else {
            eprintln!("[clipboard] 无法打开剪贴板，历史记录不可用");
            return;
        };
        // 启动时先采样基线，避免把用户已有的剪贴板内容记进来
        let mut last_text: Option<String> = board.get_text().ok();
        let mut last_image: Option<u64> = board
            .get_image()
            .ok()
            .map(|i| image_hash(i.width, i.height, &i.bytes));

        loop {
            std::thread::sleep(POLL_INTERVAL);

            // 文本优先；剪贴板是图片时 get_text 会失败
            if let Ok(text) = board.get_text() {
                if text.is_empty() || last_text.as_deref() == Some(text.as_str()) {
                    continue;
                }
                last_text = Some(text.clone());
                if record_text(&app, text) {
                    let _ = app.emit("data-changed", ());
                }
                continue;
            }

            if let Ok(img) = board.get_image() {
                let hash = image_hash(img.width, img.height, &img.bytes);
                if last_image == Some(hash) {
                    continue;
                }
                last_image = Some(hash);
                if record_image(&app, img.width as u32, img.height as u32, &img.bytes) {
                    let _ = app.emit("data-changed", ());
                }
            }
        }
    });
}

fn record_text(app: &AppHandle, text: String) -> bool {
    let mut store = lock(app);
    if store.suppress_clipboard || !store.data.settings.capture_clipboard {
        return false;
    }
    let now = now_ms();
    store.data.clipboard.retain(|i| i.content != text);
    store.data.clipboard.insert(
        0,
        ClipboardItem {
            id: new_id(),
            content: text,
            copied_at: now,
            kind: "text".into(),
            image: None,
        },
    );
    store.data.clipboard.truncate(MAX_CLIPBOARD_ITEMS);
    store.save().is_ok()
}

fn record_image(app: &AppHandle, width: u32, height: u32, rgba: &[u8]) -> bool {
    let mut store = lock(app);
    if store.suppress_clipboard || !store.data.settings.capture_clipboard {
        return false;
    }
    let id = new_id();
    let Ok(file) = crate::images::save_png(app, &id, width, height, rgba) else {
        eprintln!("[clipboard] 保存图片失败");
        return false;
    };

    store.data.clipboard.insert(
        0,
        ClipboardItem {
            id: id.clone(),
            content: String::new(),
            copied_at: now_ms(),
            kind: "image".into(),
            image: Some(ImageRef { file, width, height }),
        },
    );
    store.data.clipboard.truncate(MAX_CLIPBOARD_ITEMS);

    // 图片条目单独限数，超出删最旧（含磁盘文件）
    let image_count = store.data.clipboard.iter().filter(|i| i.is_image()).count();
    if image_count > MAX_IMAGES {
        let oldest = store
            .data
            .clipboard
            .iter()
            .filter(|i| i.is_image())
            .min_by_key(|i| i.copied_at)
            .map(|i| i.id.clone());
        if let Some(old_id) = oldest {
            crate::images::delete_files(app, &old_id);
            store.data.clipboard.retain(|i| i.id != old_id);
        }
    }
    store.save().is_ok()
}
