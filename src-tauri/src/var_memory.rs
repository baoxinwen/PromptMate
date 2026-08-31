use std::collections::BTreeMap;
use std::path::PathBuf;

use tauri::{Manager, Wry};

/// 变量值记忆文件：<app_data_dir>/var-memory.json
/// 结构：{ [promptId]: { [变量名]: 上次填写的值 } }
/// 个性化数据，独立文件、不参与云同步。
fn file_path(app: &tauri::AppHandle<Wry>) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("var-memory.json")
}

type Memory = BTreeMap<String, BTreeMap<String, String>>;

const MAX_VALUE_LEN: usize = 4000;
const MAX_PROMPTS: usize = 500;

fn read_file(path: &PathBuf) -> Memory {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn get_var_memory(app: tauri::AppHandle<Wry>, prompt_id: String) -> BTreeMap<String, String> {
    let mem = read_file(&file_path(&app));
    mem.get(&prompt_id).cloned().unwrap_or_default()
}

pub fn save_var_memory(
    app: tauri::AppHandle<Wry>,
    prompt_id: String,
    values: BTreeMap<String, String>,
) -> Result<(), String> {
    if prompt_id.is_empty() {
        return Ok(());
    }
    let path = file_path(&app);
    let mut mem = read_file(&path);
    // 单值超限不记忆，避免把大段代码长期固化进文件
    let cleaned: BTreeMap<String, String> = values
        .into_iter()
        .filter(|(_, v)| v.len() <= MAX_VALUE_LEN)
        .collect();
    mem.insert(prompt_id, cleaned);
    if mem.len() > MAX_PROMPTS {
        let keep_from = mem.len().saturating_sub(MAX_PROMPTS);
        let drop_keys: Vec<String> = mem.keys().take(keep_from).cloned().collect();
        for k in drop_keys {
            mem.remove(&k);
        }
    }
    let tmp = path.with_extension("json.tmp");
    {
        let f = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
        serde_json::to_writer_pretty(&f, &mem).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&tmp, &path).map_err(|e| format!("保存变量记忆失败: {e}"))
}
