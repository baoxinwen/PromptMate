use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use tauri::Wry;

/// 变量值记忆文件：<app_data_dir>/var-memory.json
/// 结构：{ [promptId]: { [变量名]: 上次填写的值 } }
/// 个性化数据，独立文件、不参与云同步。
fn file_path(app: &tauri::AppHandle<Wry>) -> PathBuf {
    crate::store::resolve_data_dir(app)
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("var-memory.json")
}

type Memory = BTreeMap<String, BTreeMap<String, String>>;

const MAX_VALUE_LEN: usize = 4000;
const MAX_PROMPTS: usize = 500;

fn read_memory(path: &Path) -> Memory {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn write_memory(path: &Path, mem: &Memory) -> Result<(), String> {
    let tmp = path.with_extension("json.tmp");
    {
        let f = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
        serde_json::to_writer_pretty(&f, mem).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&tmp, path).map_err(|e| format!("保存变量记忆失败: {e}"))
}

/// 单值超限不记忆，避免把大段代码长期固化进文件
fn clean_values(values: BTreeMap<String, String>) -> BTreeMap<String, String> {
    values
        .into_iter()
        .filter(|(_, v)| v.len() <= MAX_VALUE_LEN)
        .collect()
}

/// 超上限时丢弃键序靠前的旧条目
fn cap_prompts(mem: Memory) -> Memory {
    if mem.len() <= MAX_PROMPTS {
        return mem;
    }
    let keep_from = mem.len().saturating_sub(MAX_PROMPTS);
    let drop_keys: Vec<String> = mem.keys().take(keep_from).cloned().collect();
    let mut mem = mem;
    for k in drop_keys {
        mem.remove(&k);
    }
    mem
}

pub fn get_var_memory(app: tauri::AppHandle<Wry>, prompt_id: String) -> BTreeMap<String, String> {
    read_memory(&file_path(&app))
        .get(&prompt_id)
        .cloned()
        .unwrap_or_default()
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
    let mut mem = read_memory(&path);
    mem.insert(prompt_id, clean_values(values));
    let mem = cap_prompts(mem);
    write_memory(&path, &mem)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_reads_as_empty() {
        let dir = tempfile::tempdir().unwrap();
        let mem = read_memory(&dir.path().join("var-memory.json"));
        assert!(mem.is_empty());
    }

    #[test]
    fn memory_roundtrip_via_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("var-memory.json");

        let mut mem: Memory = BTreeMap::new();
        let mut vals = BTreeMap::new();
        vals.insert("name".to_string(), "小明".to_string());
        mem.insert("p1".to_string(), vals);
        write_memory(&path, &mem).unwrap();

        let loaded = read_memory(&path);
        assert_eq!(loaded["p1"]["name"], "小明");
        let missing = loaded.get("p2").cloned().unwrap_or_default();
        assert!(missing.is_empty(), "不存在的提示词返回空记忆");
    }

    #[test]
    fn clean_values_drops_oversized_but_keeps_normal() {
        let mut values = BTreeMap::new();
        values.insert("ok".to_string(), "短值".to_string());
        values.insert("huge".to_string(), "x".repeat(MAX_VALUE_LEN + 1));
        values.insert("edge".to_string(), "y".repeat(MAX_VALUE_LEN));
        let cleaned = clean_values(values);
        assert!(!cleaned.contains_key("huge"), "超过上限的值必须被丢弃");
        assert!(cleaned.contains_key("ok"));
        assert!(cleaned.contains_key("edge"), "恰好等于上限的值保留");
    }

    #[test]
    fn cap_prompts_keeps_last_500_by_key_order() {
        let mut mem: Memory = BTreeMap::new();
        for i in 0..510u32 {
            mem.insert(format!("k{i:03}"), BTreeMap::new());
        }
        let capped = cap_prompts(mem);
        assert_eq!(capped.len(), MAX_PROMPTS);
        assert!(!capped.contains_key("k000"), "键序靠前的旧条目被丢弃");
        assert!(!capped.contains_key("k009"));
        assert!(capped.contains_key("k010"));
        assert!(capped.contains_key("k509"));
    }

    #[test]
    fn cap_prompts_noop_under_limit() {
        let mut mem: Memory = BTreeMap::new();
        mem.insert("only".to_string(), BTreeMap::new());
        let capped = cap_prompts(mem);
        assert_eq!(capped.len(), 1);
    }
}
