use std::path::PathBuf;
use std::sync::Mutex;

use tauri::{AppHandle, Manager};

use crate::models::AppData;

/// 共享数据仓库：内存数据 + 本地 JSON 持久化
pub struct Store {
    pub data: AppData,
    path: PathBuf,
    /// 置为 true 时剪贴板监听线程跳过采样（避免把程序自己写入的粘贴内容记入历史）
    pub suppress_clipboard: bool,
    /// 呼出快捷面板时的前台窗口句柄，用于粘贴时恢复焦点
    pub paste_target: Option<isize>,
    /// 自上次同步后有数据变更（内存态，不持久化），供自动同步判断
    pub dirty_unsynced: bool,
    /// 本地变更计数：每次 mutate 自增。同步收尾时比对快照时刻的计数，
    /// 判断上传期间是否有新变更，防止把未上传的内容误标为已同步
    pub mutations: u64,
    /// 启动时发现 data.json 损坏并已隔离的提示信息（供前端 toast 展示）
    pub recovered_notice: Option<String>,
}

pub type SharedStore = Mutex<Store>;

/// 对 Mutex 中毒场景的容错取锁
pub fn lock(app: &tauri::AppHandle) -> std::sync::MutexGuard<'_, Store> {
    use tauri::Manager;
    let store: &SharedStore = app.state::<SharedStore>().inner();
    store.lock().unwrap_or_else(|p| p.into_inner())
}

impl Store {
    pub fn load(app: &AppHandle) -> Result<Self, String> {
        let dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("无法确定数据目录: {e}"))?;
        std::fs::create_dir_all(&dir).map_err(|e| format!("创建数据目录失败: {e}"))?;
        let path = dir.join("data.json");

        let mut recovered_notice = None;
        let mut data = if path.exists() {
            let loaded = std::fs::read_to_string(&path)
                .map_err(|e| format!("读取数据失败: {e}"))
                .and_then(|raw| {
                    serde_json::from_str::<AppData>(&raw).map_err(|e| format!("数据文件损坏: {e}"))
                });
            match loaded {
                Ok(d) => d,
                Err(e) => {
                    // 数据损坏不再让应用拒绝启动：隔离坏文件，空数据继续运行
                    let quarantined = dir.join(format!("data.json.corrupt-{}", crate::models::now_ms()));
                    let _ = std::fs::rename(&path, &quarantined);
                    eprintln!(
                        "[promptmate] data.json 解析失败已隔离到 {}，以空数据启动: {e}",
                        quarantined.display()
                    );
                    recovered_notice = Some(format!(
                        "数据文件损坏，已隔离为 {}，本次以空数据启动。若存在 data.json.bak 可手动改名为 data.json 恢复",
                        quarantined
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default()
                    ));
                    AppData::default()
                }
            }
        } else {
            AppData::default()
        };
        crate::models::migrate(&mut data);

        Ok(Self {
            data,
            path,
            suppress_clipboard: false,
            paste_target: None,
            dirty_unsynced: false,
            mutations: 0,
            recovered_notice,
        })
    }

    pub fn is_first_run(&self) -> bool {
        !self.path.exists() && !self.data.seeded
    }

    pub fn save(&self) -> Result<(), String> {
        let tmp = self.path.with_extension("json.tmp");
        let json = serde_json::to_string_pretty(&self.data).map_err(|e| e.to_string())?;
        {
            use std::io::Write;
            let mut f = std::fs::File::create(&tmp).map_err(|e| format!("写入数据失败: {e}"))?;
            f.write_all(json.as_bytes())
                .map_err(|e| format!("写入数据失败: {e}"))?;
            // 落盘后再改名，避免断电时 rename 出一个空壳文件
            f.sync_all().map_err(|e| format!("写入数据失败: {e}"))?;
        }
        // 保留上一份完好数据；本次写入若在之后损坏，可手动改回恢复
        if self.path.exists() {
            let _ = std::fs::copy(&self.path, self.path.with_extension("json.bak"));
        }
        std::fs::rename(&tmp, &self.path).map_err(|e| format!("保存数据失败: {e}"))?;
        Ok(())
    }

    /// 修改数据并落盘（用户数据变更，标记待同步）
    pub fn mutate<F>(&mut self, f: F) -> Result<(), String>
    where
        F: FnOnce(&mut AppData),
    {
        f(&mut self.data);
        self.mutations = self.mutations.wrapping_add(1);
        self.dirty_unsynced = true;
        self.save()
    }

    pub fn data_dir(&self) -> PathBuf {
        self.path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    }
}
