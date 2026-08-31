mod capture;
mod clipboard;
mod commands;
mod hotkey;
mod images;
mod models;
mod paste;
mod store;
mod sync;
mod transfer;
mod tray;
mod var_memory;

use tauri::{Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use store::SharedStore;

/// 创建三个窗口：快捷面板 / 管理窗口 / 快速捕获小窗
fn create_windows(app: &tauri::App) -> tauri::Result<()> {
    // 快捷面板（无框、置顶、不进任务栏，Spotlight 式）
    WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
        .title("PromptMate")
        .inner_size(760.0, 520.0)
        .resizable(false)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .center()
        .build()?;

    // 管理窗口（点关闭 = 隐藏到托盘）
    WebviewWindowBuilder::new(app, "manager", WebviewUrl::default())
        .title("PromptMate 提示词助手")
        .inner_size(1040.0, 700.0)
        .min_inner_size(880.0, 580.0)
        .visible(false)
        .center()
        .build()?;

    // 快速捕获小窗
    WebviewWindowBuilder::new(app, "capture", WebviewUrl::default())
        .title("快速捕获")
        .inner_size(360.0, 320.0)
        .resizable(false)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .center()
        .build()?;

    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            eprintln!("[promptmate] single-instance callback fired");
            hotkey::show_quick_window(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let handle = app.handle().clone();
            let mut store = store::Store::load(&handle)?;
            let first_run = store.is_first_run();
            let recovered = store.recovered_notice.clone();
            store.data.seed_if_empty();
            store.save()?;
            app.manage(SharedStore::new(store));

            clipboard::spawn(handle.clone());
            sync::spawn_auto_sync(handle.clone());

            // 注册全部全局快捷键（面板主键 + 快速捕获 + 提示词独立键）
            hotkey::register_all(&handle);

            tray::create(&handle)?;

            // 窗口必须在 manage() 之后创建：webview 加载后前端会立即调用
            // get_data 等命令，若窗口先于数据初始化创建，命令会因 state
            // 未就绪而 panic（release 模式内嵌资源加载极快，必现）。
            create_windows(app)?;

            if first_run || recovered.is_some() {
                hotkey::open_manager_window(&handle);
            }
            // 恢复提示由前端挂载后经 get_recovery_notice 主动拉取，
            // 不用事件：窗口创建初期事件先于监听注册，必然丢失
            Ok(())
        })
        .on_window_event(|window, event| match event {
            // 快捷面板失焦即隐藏（类似 Spotlight）
            WindowEvent::Focused(false) if window.label() == "main" => {
                let _ = window.hide();
            }
            // 捕获小窗失焦即取消
            WindowEvent::Focused(false) if window.label() == "capture" => {
                let _ = window.hide();
            }
            // 管理窗口点关闭 = 隐藏到托盘，不退出程序
            WindowEvent::CloseRequested { api, .. } if window.label() == "manager" => {
                api.prevent_close();
                let _ = window.hide();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_data,
            commands::get_recovery_notice,
            commands::save_prompt,
            commands::delete_prompt,
            commands::record_prompt_use,
            commands::add_category,
            commands::rename_category,
            commands::delete_category,
            commands::copy_text,
            commands::invoke_paste,
            commands::paste_text_direct,
            commands::get_image_thumb,
            commands::paste_image,
            commands::delete_history_item,
            commands::clear_history,
            commands::hide_quick,
            commands::open_manager,
            commands::close_capture,
            commands::set_panel_height,
            commands::get_clipboard_text,
            commands::get_var_memory,
            commands::save_var_memory,
            commands::save_settings,
            commands::get_autostart,
            commands::set_autostart,
            commands::open_data_dir,
            commands::export_data,
            commands::import_data,
            commands::import_paths,
            commands::webdav_test,
            commands::gist_test,
            commands::sync_now,
        ])
        .run(tauri::generate_context!())
        .expect("error while running PromptMate");
}
