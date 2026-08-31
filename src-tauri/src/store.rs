use std::path::{Path, PathBuf};
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

/// 从 data.json 载入数据并跑迁移；文件不存在以默认数据启动，
/// 损坏时隔离坏文件（data.json.corrupt-*）并以空数据启动，返回给前端的提示
fn load_or_recover(path: &Path) -> (AppData, Option<String>) {
    let mut recovered_notice = None;
    let mut data = if path.exists() {
        let loaded = std::fs::read_to_string(path)
            .map_err(|e| format!("读取数据失败: {e}"))
            .and_then(|raw| {
                serde_json::from_str::<AppData>(&raw).map_err(|e| format!("数据文件损坏: {e}"))
            });
        match loaded {
            Ok(d) => d,
            Err(e) => {
                // 数据损坏不再让应用拒绝启动：隔离坏文件，空数据继续运行
                let dir = path.parent().unwrap_or_else(|| Path::new("."));
                let quarantined = dir.join(format!("data.json.corrupt-{}", crate::models::now_ms()));
                let _ = std::fs::rename(path, &quarantined);
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
    (data, recovered_notice)
}

/// 对 Mutex 中毒场景的容错取锁
pub fn lock(app: &tauri::AppHandle) -> std::sync::MutexGuard<'_, Store> {
    use tauri::Manager;
    let store: &SharedStore = app.state::<SharedStore>().inner();
    store.lock().unwrap_or_else(|p| p.into_inner())
}

/// 应用数据目录。自动化测试可通过环境变量 PROMPTMATE_DATA_DIR 覆盖，
/// 避免真机 E2E 读写用户真实数据；正常启动不受影响
pub(crate) fn resolve_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    if let Ok(dir) = std::env::var("PROMPTMATE_DATA_DIR") {
        if !dir.trim().is_empty() {
            return Ok(PathBuf::from(dir));
        }
    }
    app.path()
        .app_data_dir()
        .map_err(|e| format!("无法确定数据目录: {e}"))
}

impl Store {
    pub fn load(app: &AppHandle) -> Result<Self, String> {
        let dir = resolve_data_dir(app)?;
        std::fs::create_dir_all(&dir).map_err(|e| format!("创建数据目录失败: {e}"))?;
        let path = dir.join("data.json");

        let (data, recovered_notice) = load_or_recover(&path);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Prompt;
    use std::sync::Arc;

    fn stub_prompt(id: &str, title: &str) -> Prompt {
        Prompt {
            id: id.to_string(),
            title: title.to_string(),
            content: format!("正文-{title}"),
            category: "开发".into(),
            tags: vec![],
            pinned: false,
            hotkey: String::new(),
            use_count: 0,
            last_used_at: 0,
            created_at: 1,
            updated_at: 1,
        }
    }

    fn store_in(dir: &Path) -> Store {
        let path = dir.join("data.json");
        let (data, notice) = load_or_recover(&path);
        Store {
            data,
            path,
            suppress_clipboard: false,
            paste_target: None,
            dirty_unsynced: false,
            mutations: 0,
            recovered_notice: notice,
        }
    }

    // ---------- load / 损坏隔离 ----------

    #[test]
    fn missing_file_starts_with_defaults_after_migrate() {
        let dir = tempfile::tempdir().unwrap();
        let store = store_in(dir.path());
        assert!(store.recovered_notice.is_none(), "无文件不算恢复场景");
        assert_eq!(store.data.settings.hotkey, "alt+q");
        assert!(store.data.settings.hotkey_migrated, "载入时必须执行迁移");
        assert!(store.is_first_run());
    }

    #[test]
    fn valid_file_is_loaded_and_not_first_run() {
        let dir = tempfile::tempdir().unwrap();
        let mut original = AppData::default();
        original.prompts.push(stub_prompt("p1", "已有条目"));
        std::fs::write(
            dir.path().join("data.json"),
            serde_json::to_string(&original).unwrap(),
        )
        .unwrap();

        let store = store_in(dir.path());
        assert!(store.recovered_notice.is_none());
        assert_eq!(store.data.prompts.len(), 1);
        assert_eq!(store.data.prompts[0].title, "已有条目");
        assert!(!store.is_first_run());
    }

    #[test]
    fn corrupt_file_is_quarantined_with_notice() {
        let dir = tempfile::tempdir().unwrap();
        let bad = "{{{ 不是合法 JSON".to_string();
        std::fs::write(dir.path().join("data.json"), &bad).unwrap();

        let store = store_in(dir.path());
        let notice = store
            .recovered_notice
            .as_ref()
            .expect("损坏场景必须返回恢复提示");
        assert!(notice.contains("data.json.corrupt-"), "提示应包含隔离文件名: {notice}");
        assert!(!dir.path().join("data.json").exists(), "坏文件必须被改名隔离");
        assert!(store.data.prompts.is_empty(), "损坏后以空数据启动");

        let quarantined: Vec<String> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.starts_with("data.json.corrupt-"))
            .collect();
        assert_eq!(quarantined.len(), 1, "只应隔离出一个文件");
        let saved = std::fs::read_to_string(dir.path().join(&quarantined[0])).unwrap();
        assert_eq!(saved, bad, "隔离文件必须原样保留坏内容供抢救");
    }

    // ---------- save / .bak ----------

    #[test]
    fn save_persists_deserializable_data_json() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = store_in(dir.path());
        store.data.prompts.push(stub_prompt("p1", "条目"));
        store.save().expect("save 应成功");

        let raw = std::fs::read_to_string(dir.path().join("data.json")).unwrap();
        let parsed: AppData = serde_json::from_str(&raw).expect("落盘文件必须可反序列化");
        assert_eq!(parsed.prompts[0].title, "条目");
    }

    #[test]
    fn save_keeps_previous_version_as_bak() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = store_in(dir.path());
        store.data.prompts.push(stub_prompt("p1", "第一版"));
        store.save().unwrap();

        store
            .mutate(|d| d.prompts.push(stub_prompt("p2", "第二版")))
            .unwrap();

        let bak_path = dir.path().join("data.json.bak");
        assert!(bak_path.exists(), "二次保存后必须存在 .bak");
        let bak: AppData =
            serde_json::from_str(&std::fs::read_to_string(&bak_path).unwrap()).unwrap();
        assert_eq!(bak.prompts.len(), 1, ".bak 应保存上一版（仅 1 条）");
        assert_eq!(bak.prompts[0].title, "第一版");
        assert_eq!(store.data.prompts.len(), 2);
    }

    #[test]
    fn mutate_updates_counters_flags_and_disk() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = store_in(dir.path());
        assert_eq!(store.mutations, 0);
        assert!(!store.dirty_unsynced);

        store
            .mutate(|d| d.prompts.push(stub_prompt("p1", "变更一")))
            .unwrap();
        assert_eq!(store.mutations, 1);
        assert!(store.dirty_unsynced);

        store
            .mutate(|d| d.prompts.push(stub_prompt("p2", "变更二")))
            .unwrap();
        assert_eq!(store.mutations, 2);
        let raw = std::fs::read_to_string(dir.path().join("data.json")).unwrap();
        let parsed: AppData = serde_json::from_str(&raw).unwrap();
        assert_eq!(parsed.prompts.len(), 2, "每次 mutate 都应落盘");
    }

    // ---------- 并发 mutate ----------

    #[test]
    fn concurrent_mutates_count_exactly_and_keep_file_valid() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(std::sync::Mutex::new(store_in(dir.path())));

        let handles: Vec<_> = (0..4)
            .map(|t| {
                let store = Arc::clone(&store);
                std::thread::spawn(move || {
                    for i in 0..25 {
                        let mut s = store.lock().unwrap();
                        s.mutate(|d| d.prompts.push(stub_prompt(&format!("p{t}-{i}"), &format!("t{t}-{i}"))))
                            .unwrap();
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }

        let s = store.lock().unwrap();
        assert_eq!(s.mutations, 100, "4 线程 × 25 次变更，计数必须精确");
        assert_eq!(s.data.prompts.len(), 100);
        let raw = std::fs::read_to_string(dir.path().join("data.json")).unwrap();
        let parsed: AppData = serde_json::from_str(&raw).expect("并发保存后的文件必须始终合法");
        assert_eq!(parsed.prompts.len(), 100);
        let mut ids: Vec<&str> = parsed.prompts.iter().map(|p| p.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 100, "所有变更都必须真实落盘、无丢失");
    }
}
