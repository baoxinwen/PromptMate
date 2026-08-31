use std::collections::HashSet;

use serde_json::Value;

use crate::models::{new_id, now_ms, AppData, Prompt};

/// 导出为 JSON（完整备份）。图片条目体积大且是临时性内容，不导出；
/// 墓碑随备份走：恢复备份后仍能拦住云端同步把已删除条目复活。
/// 含同步凭据原文的剪贴板条目强制排除（复制 token/密码会被剪贴板记录捕获）
pub fn export_json(data: &AppData, include_clipboard: bool) -> String {
    let secrets = [&data.settings.gist.token, &data.settings.webdav.password];
    let clipboard = if include_clipboard {
        data.clipboard
            .iter()
            .filter(|i| !i.is_image())
            .filter(|i| {
                secrets
                    .iter()
                    .all(|s| s.trim().is_empty() || !i.content.contains(s.trim()))
            })
            .cloned()
            .collect::<Vec<_>>()
    } else {
        vec![]
    };
    let payload = serde_json::json!({
        "app": "PromptMate",
        "version": 1,
        "exportedAt": now_ms(),
        "categories": data.categories,
        "prompts": data.prompts,
        "tombstones": data.tombstones,
        "clipboard": clipboard,
    });
    serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
}

/// 内容里含连续反引号时用更长的围栏包裹，保证再导入能完整还原
fn fence_for(content: &str) -> String {
    let mut longest = 0usize;
    let mut run = 0usize;
    for c in content.chars() {
        if c == '`' {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    "`".repeat((longest + 1).max(3))
}

fn push_prompt(out: &mut String, p: &Prompt) {
    out.push_str(&format!("## {}\n\n", p.title));
    if !p.tags.is_empty() {
        out.push_str(&format!("标签: {}\n\n", p.tags.join(", ")));
    }
    let fence = fence_for(&p.content);
    out.push_str(&format!("{}\n{}\n{}\n\n", fence, p.content, fence));
}

/// 导出为 Markdown：H1 分类、H2 标题、围栏正文。
/// 分类列表之外的提示词（空分类 / 孤儿分类）归入「未分类」，不再静默丢失
pub fn export_markdown(data: &AppData) -> String {
    let mut out = String::from("# PromptMate 提示词库\n\n");
    let mut exported: Vec<&str> = Vec::new();
    for cat in &data.categories {
        let items: Vec<&Prompt> = data
            .prompts
            .iter()
            .filter(|p| &p.category == cat)
            .collect();
        if items.is_empty() {
            continue;
        }
        exported.push(cat.as_str());
        out.push_str(&format!("# {cat}\n\n"));
        for p in items {
            push_prompt(&mut out, p);
        }
    }
    let others: Vec<&Prompt> = data
        .prompts
        .iter()
        .filter(|p| p.category.is_empty() || !exported.contains(&p.category.as_str()))
        .collect();
    if !others.is_empty() {
        out.push_str("# 未分类\n\n");
        for p in others {
            push_prompt(&mut out, p);
        }
    }
    out
}

/// JSON 导入：兼容 完整备份 / 同步载荷 / {prompts:[...]} / 纯数组 四种形态。
/// 按 id 合并：已存在的跳过。返回 (新增数, 跳过数)
pub fn import_json(data: &mut AppData, text: &str) -> Result<(usize, usize), String> {
    let v: Value =
        serde_json::from_str(text).map_err(|e| format!("JSON 解析失败: {e}"))?;

    let prompts_val = match &v {
        Value::Array(_) => Some(&v),
        Value::Object(o) => o
            .get("prompts")
            .or_else(|| o.get("data"))
            .filter(|p| p.is_array()),
        _ => None,
    };
    let prompts_val = prompts_val.ok_or("JSON 中未找到 prompts 字段")?;

    if let Value::Object(o) = &v {
        if let Some(Value::Array(cats)) = o.get("categories") {
            for c in cats {
                if let Some(name) = c.as_str() {
                    data.ensure_category(name);
                }
            }
        }
        // 合并备份中的墓碑（按 id 去重取较新时间），恢复后云端已删条目不会复活
        if let Some(Value::Array(ts)) = o.get("tombstones") {
            for t in ts {
                let id = t.get("id").and_then(|x| x.as_str());
                let at = t.get("at").and_then(|x| x.as_u64());
                if let (Some(id), Some(at)) = (id, at) {
                    if id.is_empty() {
                        continue;
                    }
                    match data.tombstones.iter_mut().find(|x| x.id == id) {
                        Some(x) => x.at = x.at.max(at),
                        None => data.tombstones.push(crate::models::Tombstone {
                            id: id.to_string(),
                            at,
                        }),
                    }
                }
            }
            if data.tombstones.len() > 5000 {
                let over = data.tombstones.len() - 5000;
                data.tombstones.drain(..over);
            }
        }
    }

    let existing: HashSet<String> = data.prompts.iter().map(|p| p.id.clone()).collect();
    let existing_titles: HashSet<String> = data
        .prompts
        .iter()
        .map(|p| format!("{}\u{0}{}", p.category, p.title))
        .collect();

    let mut added = 0usize;
    let mut skipped = 0usize;
    for (i, item) in prompts_val
        .as_array()
        .expect("已在上方校验为数组")
        .iter()
        .enumerate()
    {
        let parsed = serde_json::from_value::<Prompt>(item.clone());
        let mut prompt = match parsed {
            Ok(p) => p,
            Err(_) => {
                // 宽松模式：仅要求 title/content
                let title = item
                    .get("title")
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string());
                let content = item
                    .get("content")
                    .or_else(|| item.get("text"))
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string());
                match (title, content) {
                    (Some(t), Some(c)) => Prompt {
                        id: new_id(),
                        title: t,
                        content: c,
                        category: String::new(),
                        tags: vec![],
                        pinned: false,
                        hotkey: String::new(),
                        use_count: 0,
                        last_used_at: 0,
                        created_at: now_ms(),
                        updated_at: now_ms(),
                    },
                    _ => {
                        skipped += 1;
                        continue;
                    }
                }
            }
        };
        if prompt.title.is_empty() {
            prompt.title = format!("导入提示词 {}", i + 1);
        }
        if existing.contains(&prompt.id) || existing_titles.contains(&key_of(&prompt)) {
            skipped += 1;
            continue;
        }
        if prompt.category.is_empty() {
            prompt.category = "未分类".into();
        }
        data.ensure_category(&prompt.category);
        data.prompts.push(prompt);
        added += 1;
    }
    Ok((added, skipped))
}

fn key_of(p: &Prompt) -> String {
    format!("{}\u{0}{}", p.category, p.title)
}

/// Markdown 导入：# 分类 → ## 标题 + 标签行 + 代码块/正文。返回 (新增数, 跳过数)
pub fn import_markdown(data: &mut AppData, text: &str, default_category: &str) -> (usize, usize) {
    let mut added = 0usize;
    let mut skipped = 0usize;
    let existing: HashSet<String> = data
        .prompts
        .iter()
        .map(|p| key_of(p))
        .collect();

    let mut category = String::new();
    let mut current: Option<Prompt> = None;
    let mut in_code = false;
    let mut lines: Vec<String> = Vec::new();

    let flush = |data: &mut AppData,
                     current: &mut Option<Prompt>,
                     lines: &mut Vec<String>,
                     existing: &HashSet<String>,
                     added: &mut usize,
                     skipped: &mut usize,
                     in_code: &mut bool| {
        if let Some(mut p) = current.take() {
            let raw = lines.join("\n");
            let content = strip_code_fence(&raw);
            p.content = content;
            if !p.content.trim().is_empty() {
                if existing.contains(&key_of(&p)) {
                    *skipped += 1;
                } else {
                    p.id = new_id();
                    let now = now_ms();
                    p.created_at = now;
                    p.updated_at = now;
                    if p.category.is_empty() {
                        p.category = default_category.to_string();
                    }
                    data.ensure_category(&p.category);
                    data.prompts.push(p);
                    *added += 1;
                }
            } else {
                *skipped += 1;
            }
            lines.clear();
            *in_code = false;
        }
    };

    for raw_line in text.lines() {
        let line = raw_line.trim_end();
        let trimmed = line.trim();

        if trimmed.starts_with("```") {
            in_code = !in_code;
            lines.push(line.to_string());
            continue;
        }
        if in_code {
            lines.push(line.to_string());
            continue;
        }

        if let Some(h2) = trimmed.strip_prefix("## ") {
            flush(
                data,
                &mut current,
                &mut lines,
                &existing,
                &mut added,
                &mut skipped,
                &mut in_code,
            );
            current = Some(Prompt {
                id: new_id(),
                title: h2.trim().to_string(),
                content: String::new(),
                category: category.clone(),
                tags: vec![],
                pinned: false,
    hotkey: String::new(),
                use_count: 0,
                last_used_at: 0,
                created_at: now_ms(),
                updated_at: now_ms(),
            });
        } else if let Some(h1) = trimmed.strip_prefix("# ") {
            flush(
                data,
                &mut current,
                &mut lines,
                &existing,
                &mut added,
                &mut skipped,
                &mut in_code,
            );
            let name = h1.trim().to_string();
            if name != "PromptMate 提示词库" {
                category = name;
                data.ensure_category(&category);
            }
        } else if current.is_some() {
            if trimmed.starts_with("标签:") || trimmed.starts_with("标签：") {
                let tags_part = trimmed
                    .trim_start_matches("标签:")
                    .trim_start_matches("标签：")
                    .trim();
                if let Some(cp) = current.as_mut() {
                    cp.tags = tags_part
                        .split([',', '，', ' '])
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            } else {
                lines.push(line.to_string());
            }
        }
    }
    flush(
        data,
        &mut current,
        &mut lines,
        &existing,
        &mut added,
        &mut skipped,
        &mut in_code,
    );
    (added, skipped)
}

/// 纯文本导入：一个文件一条，文件名为标题
pub fn import_text(data: &mut AppData, title: &str, content: &str) -> bool {
    if content.trim().is_empty() {
        return false;
    }
    let now = now_ms();
    let prompt = Prompt {
        id: new_id(),
        title: title.to_string(),
        content: content.to_string(),
        category: "未分类".into(),
        tags: vec![],
        pinned: false,
    hotkey: String::new(),
        use_count: 0,
        last_used_at: 0,
        created_at: now,
        updated_at: now,
    };
    data.ensure_category("未分类");
    data.prompts.push(prompt);
    true
}

/// 剥离导出时包裹的围栏。开/闭围栏均为 3 个及以上连续反引号，
/// 闭围栏长度须不短于开围栏（与 markdown 规则一致），内容自身的围栏保持原样
fn strip_code_fence(raw: &str) -> String {
    let trimmed = raw.trim_matches('\n');
    let lines: Vec<&str> = trimmed.lines().collect();
    if lines.len() >= 2 {
        let open = lines[0].trim();
        let open_len = open.chars().take_while(|c| *c == '`').count();
        if open_len >= 3 && open.chars().skip(open_len).all(|c| c == '`') {
            let close = lines[lines.len() - 1].trim();
            let close_len = close.chars().take_while(|c| *c == '`').count();
            if close_len >= open_len && close.chars().skip(close_len).all(|c| c == '`') {
                return lines[1..lines.len() - 1].join("\n");
            }
        }
    }
    trimmed.to_string()
}

