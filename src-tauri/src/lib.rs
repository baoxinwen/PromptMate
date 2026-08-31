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

/// 创建三个窗口：快捷面板 / 管理窗口 / 快速捕获小窗。
/// E2E 模式下主窗口直接可见（便于 WebDriver 定位与截图），并显式开启
/// WebView2 远程调试端口：wry 传入的 additional_browser_args 会覆盖
/// WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS 环境变量，调试端口必须由这里给出
fn create_windows(app: &tauri::App, e2e: bool) -> tauri::Result<()> {
    // 快捷面板（无框、置顶、不进任务栏，Spotlight 式）
    let mut main = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
        .title("PromptMate")
        .inner_size(760.0, 520.0)
        .resizable(false)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(e2e)
        .center();
    if e2e {
        // 固定调试端口供 msedgedriver 以 debuggerAddress 附加；
        // --disable-features 是 wry 的默认参数，覆盖时必须原样带上
        main = main.additional_browser_args(
            "--remote-debugging-port=9222 --disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection",
        );
    }
    main.build()?;

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
    // E2E 模式（PROMPTMATE_E2E=1，供 tauri-driver 全栈测试使用）：
    // 跳过单实例、全局快捷键、剪贴板监听、自动同步与托盘，主窗口直接可见；
    // 数据目录可用 PROMPTMATE_DATA_DIR 隔离到临时目录。正常启动完全不受影响
    let e2e = std::env::var("PROMPTMATE_E2E").map(|v| v == "1").unwrap_or(false);

    let builder = tauri::Builder::default();
    let builder = if e2e {
        builder
    } else {
        builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            eprintln!("[promptmate] single-instance callback fired");
            hotkey::show_quick_window(app);
        }))
    };

    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(move |app| {
            let handle = app.handle().clone();
            let mut store = store::Store::load(&handle)?;
            let first_run = store.is_first_run();
            let recovered = store.recovered_notice.clone();
            store.data.seed_if_empty();
            store.save()?;
            app.manage(SharedStore::new(store));

            if !e2e {
                clipboard::spawn(handle.clone());
                sync::spawn_auto_sync(handle.clone());

                // 注册全部全局快捷键（面板主键 + 快速捕获 + 提示词独立键）
                hotkey::register_all(&handle);

                tray::create(&handle)?;
            }

            // 窗口必须在 manage() 之后创建：webview 加载后前端会立即调用
            // get_data 等命令，若窗口先于数据初始化创建，命令会因 state
            // 未就绪而 panic（release 模式内嵌资源加载极快，必现）。
            create_windows(app, e2e)?;

            if !e2e && (first_run || recovered.is_some()) {
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
            commands::check_accessibility,
            commands::open_accessibility_settings,
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
