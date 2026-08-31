use std::path::PathBuf;

use base64::Engine;
use tauri::{Manager, Wry};

/// 图片文件目录：<app_data_dir>/images
pub fn dir(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .map(|d| d.join("images"))
        .unwrap_or_else(|_| std::env::temp_dir().join("promptmate-images"))
}

fn ensure_dir(app: &tauri::AppHandle) -> PathBuf {
    let d = dir(app);
    let _ = std::fs::create_dir_all(&d);
    d
}

/// 保存 RGBA 原图并生成宽 160 的缩略图，返回文件名
pub fn save_png(
    app: &tauri::AppHandle,
    id: &str,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Result<String, String> {
    // 零尺寸会让缩略图计算除零得 inf、按 u32::MAX 分配缓冲直接 abort
    if width == 0 || height == 0 {
        return Err("图片尺寸无效".into());
    }
    let dir = ensure_dir(app);
    let img = image::RgbaImage::from_raw(width, height, rgba.to_vec())
        .ok_or("图片数据尺寸与宽高不匹配")?;
    let file = format!("{id}.png");
    img.save(dir.join(&file)).map_err(|e| format!("保存图片失败: {e}"))?;

    let tw = 160u32;
    let th = (((height as f64) * (tw as f64) / (width as f64)).round() as u32).max(1);
    let thumb = image::imageops::resize(&img, tw, th, image::imageops::FilterType::Triangle);
    thumb
        .save(dir.join(format!("{id}_t.png")))
        .map_err(|e| format!("保存缩略图失败: {e}"))?;
    Ok(file)
}

pub fn read_png(app: &tauri::AppHandle<Wry>, file: &str) -> Result<Vec<u8>, String> {
    // 文件名由程序生成（uuid.png），防御性校验防止路径穿越
    if file.contains("..") || file.contains('/') || file.contains('\\') {
        return Err("非法文件名".into());
    }
    std::fs::read(dir(app).join(file)).map_err(|e| format!("读取图片失败: {e}"))
}

pub fn thumb_name(file: &str) -> String {
    file.strip_suffix(".png")
        .map(|base| format!("{base}_t.png"))
        .unwrap_or_else(|| file.to_string())
}

pub fn png_base64(bytes: &[u8]) -> String {
    format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

pub fn delete_files(app: &tauri::AppHandle, id: &str) {
    let d = dir(app);
    let _ = std::fs::remove_file(d.join(format!("{id}.png")));
    let _ = std::fs::remove_file(d.join(format!("{id}_t.png")));
}
