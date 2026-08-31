use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter,
};

use crate::hotkey::{open_manager_window, show_quick_window};
use crate::sync::run_sync;

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let hotkey = {
        use crate::store::lock;
        let store = lock(&app);
        store.data.settings.hotkey.clone()
    };
    let show_label = format!("显示快捷面板 ({})", display_hotkey(&hotkey));

    let show = MenuItem::with_id(app, "show", &show_label, true, None::<&str>)?;
    let manager = MenuItem::with_id(app, "manager", "打开管理窗口", true, None::<&str>)?;
    let sync = MenuItem::with_id(app, "sync", "立即同步云端", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &manager, &sync, &quit])?;

    let _tray = TrayIconBuilder::with_id("main-tray")
        .icon(app.default_window_icon().expect("缺少应用图标").clone())
        .tooltip("PromptMate 提示词助手")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_quick_window(app),
            "manager" => open_manager_window(app),
            "sync" => {
                let handle = app.clone();
                std::thread::spawn(move || match run_sync(&handle, "merge") {
                    Ok(report) => {
                        let _ = handle.emit("sync-done", report);
                    }
                    Err(e) => {
                        let _ = handle.emit(
                            "sync-done",
                            serde_json::json!({ "added": 0, "updated": 0, "removed": 0, "message": e, "ok": false }),
                        );
                    }
                });
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_quick_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

fn display_hotkey(accel: &str) -> String {
    let macos = cfg!(target_os = "macos");
    accel
        .split('+')
        .map(|p| {
            let p = p.trim();
            match p.to_lowercase().as_str() {
                "alt" => {
                    if macos {
                        "⌥".into()
                    } else {
                        "Alt".into()
                    }
                }
                "ctrl" | "control" | "commandorcontrol" => {
                    if macos {
                        "⌃".into()
                    } else {
                        "Ctrl".into()
                    }
                }
                "shift" => {
                    if macos {
                        "⇧".into()
                    } else {
                        "Shift".into()
                    }
                }
                "super" | "meta" | "cmd" | "command" => {
                    if macos {
                        "⌘".into()
                    } else {
                        "Win".into()
                    }
                }
                other => other.to_uppercase(),
            }
        })
        .collect::<Vec<_>>()
        .join(if macos { "" } else { "+" })
}
