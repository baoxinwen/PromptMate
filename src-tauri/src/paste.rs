use std::time::Duration;

/// 模拟粘贴/复制/回车按键。Windows 用 SendInput(Ctrl+V)，macOS 用 CGEvent(Cmd+V)。
#[cfg(windows)]
mod keys {
    fn press_combo(vk_modifier: u16, vk_key: u16) {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
        };

        fn key(vk: u16, keyup: bool) -> INPUT {
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: vk,
                        wScan: 0,
                        dwFlags: if keyup { KEYEVENTF_KEYUP } else { 0 },
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            }
        }

        let inputs = [
            key(vk_modifier, false),
            key(vk_key, false),
            key(vk_key, true),
            key(vk_modifier, true),
        ];
        unsafe {
            SendInput(
                inputs.len() as u32,
                inputs.as_ptr(),
                std::mem::size_of::<INPUT>() as i32,
            );
        }
    }

    pub fn press_paste() {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_V;
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_CONTROL;
        press_combo(VK_CONTROL, VK_V);
    }

    /// 单键模拟（无修饰键），用于粘贴后追加回车
    pub fn press_enter() {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_RETURN;
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
            keybd_event, KEYEVENTF_KEYUP,
        };
        unsafe {
            keybd_event(VK_RETURN as u8, 0, 0, 0);
            keybd_event(VK_RETURN as u8, 0, KEYEVENTF_KEYUP, 0);
        }
    }

    pub fn press_copy() {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_C;
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_CONTROL;
        press_combo(VK_CONTROL, VK_C);
    }
}

#[cfg(target_os = "macos")]
mod keys {
    use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    const KEY_V: u16 = 9;
    const KEY_C: u16 = 8;
    const KEY_RETURN: u16 = 36;
    const CMD: CGEventFlags = CGEventFlags::CGEventFlagCommand;

    fn post_key(keycode: u16, flags: CGEventFlags, keydown: bool) {
        let Ok(source) = CGEventSource::new(CGEventSourceStateID::CombinedSessionState) else {
            return;
        };
        if let Ok(event) = CGEvent::new_keyboard_event(source, keycode, keydown) {
            event.set_flags(flags);
            event.post(CGEventTapLocation::HID);
        }
    }

    fn press_combo(keycode: u16) {
        post_key(keycode, CMD, true);
        post_key(keycode, CMD, false);
    }

    pub fn press_paste() {
        press_combo(KEY_V);
    }

    pub fn press_enter() {
        post_key(KEY_RETURN, CGEventFlags::empty(), true);
        post_key(KEY_RETURN, CGEventFlags::empty(), false);
    }

    pub fn press_copy() {
        press_combo(KEY_C);
    }
}

pub fn press_ctrl_v() {
    keys::press_paste();
}

pub fn press_enter() {
    keys::press_enter();
}

pub fn press_ctrl_c() {
    keys::press_copy();
}

/// 呼出面板前的前台窗口，粘贴时唤回它，保证粘贴落点正确。
/// macOS 上隐藏自身窗口后系统会自动把焦点还给前一个应用，
/// 因此不需要记录/恢复前台窗口，全部返回 None 即可。
#[cfg(windows)]
pub mod foreground {
    use windows_sys::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{keybd_event, KEYEVENTF_KEYUP, VK_MENU};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow,
    };

    pub fn current() -> Option<isize> {
        let hwnd = unsafe { GetForegroundWindow() };
        (!hwnd.is_null()).then_some(hwnd as isize)
    }

    /// 目标窗口当前是否处于前台（发送 Ctrl+V 前校验，防止误粘到别的窗口）
    pub fn is_foreground(hwnd: isize) -> bool {
        let fore = unsafe { GetForegroundWindow() };
        !fore.is_null() && fore as isize == hwnd
    }

    /// 恢复前台窗口。Windows 对后台进程有前台锁定限制，
    /// 通过短暂的 AttachThreadInput 共享输入状态绕过。
    pub fn restore(hwnd: isize) -> bool {
        unsafe {
            let fore = GetForegroundWindow();
            let this_thread = GetCurrentThreadId();
            let fore_thread = if fore.is_null() {
                0
            } else {
                GetWindowThreadProcessId(fore, std::ptr::null_mut())
            };

            // ALT 键击让系统认为有用户输入，解除部分前台锁定
            keybd_event(VK_MENU as u8, 0, 0, 0);
            keybd_event(VK_MENU as u8, 0, KEYEVENTF_KEYUP, 0);

            let attached = fore_thread != 0 && fore_thread != this_thread;
            if attached {
                AttachThreadInput(this_thread, fore_thread, 1);
            }
            let ok = SetForegroundWindow(hwnd as _) != 0;
            if attached {
                AttachThreadInput(this_thread, fore_thread, 0);
            }
            ok
        }
    }
}

#[cfg(target_os = "macos")]
#[allow(dead_code)] // macOS 上隐藏面板即归还焦点，这些接口仅保留以对齐 Windows 侧签名
pub mod foreground {
    pub fn current() -> Option<isize> {
        None
    }

    pub fn is_foreground(_hwnd: isize) -> bool {
        true
    }

    pub fn restore(_hwnd: isize) -> bool {
        true
    }
}

pub fn set_clipboard_text(text: &str) -> Result<(), String> {
    arboard::Clipboard::new()
        .and_then(|mut c| c.set_text(text.to_string()))
        .map_err(|e| format!("写入剪贴板失败: {e}"))
}

pub fn get_clipboard_text() -> Option<String> {
    arboard::Clipboard::new().and_then(|mut c| c.get_text()).ok()
}

/// 恢复目标窗口焦点并模拟粘贴（可选追加回车）。
/// Windows 下在发送前校验目标确实回到前台，失败重试一次，仍失败则放弃——
/// 固定延时无法保证焦点切换成功，盲发会把内容粘进恰好在前台的其他应用。
/// macOS 下隐藏面板即自动还焦点给前一个应用，直接发送 Cmd+V。
pub fn send_paste(target: Option<isize>, append_enter: bool) {
    #[cfg(windows)]
    match target {
        Some(hwnd) => {
            std::thread::sleep(Duration::from_millis(60));
            foreground::restore(hwnd);
            std::thread::sleep(Duration::from_millis(200));
            if !foreground::is_foreground(hwnd) {
                foreground::restore(hwnd);
                std::thread::sleep(Duration::from_millis(200));
            }
            if !foreground::is_foreground(hwnd) {
                eprintln!("[promptmate] 粘贴目标未回到前台，已取消按键以防误粘");
                return;
            }
            press_ctrl_v();
            if append_enter {
                std::thread::sleep(Duration::from_millis(60));
                press_enter();
            }
        }
        None => {
            std::thread::sleep(Duration::from_millis(200));
            press_ctrl_v();
            if append_enter {
                std::thread::sleep(Duration::from_millis(60));
                press_enter();
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        let _ = target;
        std::thread::sleep(Duration::from_millis(200));
        press_ctrl_v();
        if append_enter {
            std::thread::sleep(Duration::from_millis(60));
            press_enter();
        }
    }
}

/// 后台粘贴核心：写入剪贴板（含恢复准备）→ 唤回目标窗口 → 模拟粘贴 → 按设置恢复原剪贴板。
/// 调用方负责在需要时先行隐藏窗口 / 记录 paste_target。
pub fn paste_text(app: &tauri::AppHandle, text: &str) -> Result<(), String> {
    let (restore_clipboard, append_enter) = {
        let mut store = crate::store::lock(app);
        store.suppress_clipboard = true;
        (
            store.data.settings.restore_clipboard,
            store.data.settings.paste_append_enter,
        )
    };

    // 保存原剪贴板文本（原内容是图片等非文本时无法恢复，置 None）
    let original = if restore_clipboard { get_clipboard_text() } else { None };

    // 写入失败必须解除抑制，否则剪贴板历史从此静默失效直到重启
    if let Err(e) = set_clipboard_text(text) {
        crate::store::lock(app).suppress_clipboard = false;
        return Err(e);
    }

    let handle = app.clone();
    std::thread::spawn(move || {
        // 唤回呼出面板前的前台窗口（防止中间焦点被其他窗口抢走）
        let target = crate::store::lock(&handle).paste_target.take();
        send_paste(target, append_enter);
        // 留出目标应用处理按键的时间，再恢复原剪贴板（过早恢复会粘错内容）；
        // 之后统一保持 suppress 跨过一个剪贴板轮询周期(700ms)，让监听线程把
        // 本次粘贴/恢复动作吸收进基线，避免把程序写入的内容误记进历史
        std::thread::sleep(Duration::from_millis(300));
        if let Some(orig) = original.filter(|o| !o.is_empty()) {
            let _ = arboard::Clipboard::new().and_then(|mut c| c.set_text(orig));
        }
        std::thread::sleep(Duration::from_millis(750));
        let mut store = crate::store::lock(&handle);
        store.suppress_clipboard = false;
    });

    Ok(())
}

/// 面板粘贴：隐藏面板让焦点回到上一个窗口，然后后台粘贴
pub fn paste_to_previous_window(
    window: &tauri::WebviewWindow,
    app: &tauri::AppHandle,
    text: &str,
) -> Result<(), String> {
    let _ = window.hide();
    paste_text(app, text)
}
