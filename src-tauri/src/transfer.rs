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

    // 注意用 mut：循环内每收录一条都要回写，否则同一导入文件内
    // 的重复 id / 重复（分类,标题）条目会绕过去重双双入库
    let mut existing: HashSet<String> = data.prompts.iter().map(|p| p.id.clone()).collect();
    let mut existing_titles: HashSet<String> = data
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
                        // 宽松模式也要保留传入的 id：否则同 id 重复导入无法被去重拦截
                        id: item
                            .get("id")
                            .and_then(|x| x.as_str())
                            .filter(|s| !s.is_empty())
                            .map(String::from)
                            .unwrap_or_else(new_id),
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
        existing.insert(prompt.id.clone());
        existing_titles.insert(key_of(&prompt));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ClipboardItem, ImageRef, Tombstone};

    fn sample_prompt(id: &str, title: &str, category: &str, content: &str) -> Prompt {
        Prompt {
            id: id.to_string(),
            title: title.to_string(),
            content: content.to_string(),
            category: category.to_string(),
            tags: vec!["标签1".into(), "tag2".into()],
            pinned: false,
            hotkey: String::new(),
            use_count: 0,
            last_used_at: 0,
            created_at: 1_000,
            updated_at: 2_000,
        }
    }

    fn text_clip(id: &str, content: &str, copied_at: u64) -> ClipboardItem {
        ClipboardItem {
            id: id.to_string(),
            content: content.to_string(),
            copied_at,
            kind: "text".into(),
            image: None,
        }
    }

    fn image_clip(id: &str) -> ClipboardItem {
        ClipboardItem {
            id: id.to_string(),
            content: String::new(),
            copied_at: 1,
            kind: "image".into(),
            image: Some(ImageRef {
                file: format!("{id}.png"),
                width: 4,
                height: 4,
            }),
        }
    }

    // ---------- export_json ----------

    #[test]
    fn export_json_filters_clipboard_secrets_and_images() {
        let mut data = AppData::default();
        data.prompts.push(sample_prompt("p1", "标题", "开发", "正文"));
        data.settings.gist.token = "ghp_token".into();
        data.settings.webdav.password = "dav_pw".into();
        data.clipboard.push(text_clip("c1", "普通内容", 1));
        data.clipboard.push(text_clip("c2", "包含 ghp_token 的内容", 2));
        data.clipboard.push(text_clip("c3", "密码是 dav_pw", 3));
        data.clipboard.push(image_clip("c4"));
        data.tombstones.push(Tombstone { id: "dead".into(), at: 9 });

        let out = export_json(&data, true);
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["app"], "PromptMate");
        assert_eq!(v["prompts"].as_array().unwrap().len(), 1);
        assert_eq!(v["tombstones"].as_array().unwrap().len(), 1, "墓碑必须随备份导出");

        let clip_ids: Vec<&str> = v["clipboard"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["id"].as_str().unwrap())
            .collect();
        assert_eq!(clip_ids, vec!["c1"], "只应留下不含凭据原文的文本条目");
    }

    #[test]
    fn export_json_can_exclude_clipboard_entirely() {
        let mut data = AppData::default();
        data.clipboard.push(text_clip("c1", "hello", 1));
        let v: Value = serde_json::from_str(&export_json(&data, false)).unwrap();
        assert!(v["clipboard"].as_array().unwrap().is_empty());
    }

    // ---------- fence ----------

    #[test]
    fn fence_for_uses_min_three_backticks() {
        assert_eq!(fence_for("普通正文"), "```");
    }

    #[test]
    fn fence_for_extends_when_content_has_backtick_runs() {
        assert_eq!(fence_for("a\n```\nb"), "````");
        assert_eq!(fence_for("``````"), "```````", "6 连反引号 → 7");
    }

    #[test]
    fn strip_code_fence_variants() {
        assert_eq!(strip_code_fence("```\n正文\n```"), "正文");
        assert_eq!(strip_code_fence("无围栏"), "无围栏");
        // 内容自身的三连围栏不被外层误剥
        assert_eq!(strip_code_fence("````\n```\ninner\n```\n````"), "```\ninner\n```");
        // 闭围栏短于开围栏：不剥，保持原样
        assert_eq!(strip_code_fence("````\n正文\n```"), "````\n正文\n```");
    }

    // ---------- markdown 导出/导入往返 ----------

    #[test]
    fn markdown_roundtrip_preserves_prompts() {
        let mut data = AppData::default();
        data.categories = vec!["开发".into(), "写作".into()];
        data.prompts.push(sample_prompt(
            "p1",
            "代码审查",
            "开发",
            "第一行\n```\ncode();\n```\n最后一行",
        ));
        data.prompts.push(sample_prompt("p2", "周报", "写作", "简洁正文"));
        // 孤儿分类：不在 categories 列表里，导出必须归入「未分类」而不是丢失
        data.prompts.push(sample_prompt("p3", "孤儿", "散落分类", "正文3"));

        let md = export_markdown(&data);
        assert!(md.contains("# 未分类"), "孤儿分类必须出现在未分类中");

        let mut imported = AppData::default();
        let (added, skipped) = import_markdown(&mut imported, &md, "导入");
        assert_eq!((added, skipped), (3, 0));

        for orig in &data.prompts {
            let expect_cat = if orig.category == "散落分类" {
                "未分类"
            } else {
                &orig.category
            };
            let restored = imported
                .prompts
                .iter()
                .find(|p| p.title == orig.title)
                .unwrap_or_else(|| panic!("标题「{}」应在导入结果中", orig.title));
            assert_eq!(restored.content, orig.content, "正文往返必须一致（含内部围栏）");
            assert_eq!(restored.category, expect_cat);
            assert_eq!(restored.tags, orig.tags);
        }
    }

    #[test]
    fn import_markdown_parses_tags_with_chinese_comma() {
        let md = "# 分类\n\n## 标题A\n\n标签: 甲，乙 丙\n\n```\n正文\n```\n";
        let mut data = AppData::default();
        let (added, skipped) = import_markdown(&mut data, &md, "默认");
        assert_eq!((added, skipped), (1, 0));
        let p = &data.prompts[0];
        assert_eq!(p.title, "标题A");
        assert_eq!(p.tags, vec!["甲".to_string(), "乙".to_string(), "丙".to_string()]);
        assert_eq!(p.content, "正文");
        assert_eq!(p.category, "分类");
    }

    #[test]
    fn import_markdown_skips_empty_content_and_duplicates() {
        let md = "# 分类\n\n## 空条目\n\n## 空条目\n\n正文\n";
        let mut data = AppData::default();
        let (added, skipped) = import_markdown(&mut data, &md, "默认");
        assert_eq!(added, 1, "只有第二条（有正文）应被收录");
        assert_eq!(skipped, 1, "空正文一条应跳过");
        assert_eq!(data.prompts[0].title, "空条目");
    }

    #[test]
    fn import_markdown_uses_default_category_without_h1() {
        let md = "## 无分类条目\n\n```\n正文\n```\n";
        let mut data = AppData::default();
        let (added, _) = import_markdown(&mut data, &md, "兜底分类");
        assert_eq!(added, 1);
        assert_eq!(data.prompts[0].category, "兜底分类");
    }

    #[test]
    fn import_markdown_ignores_library_header() {
        let md = "# PromptMate 提示词库\n\n## 条目\n\n```\n正文\n```\n";
        let mut data = AppData::default();
        import_markdown(&mut data, &md, "默认");
        assert!(
            !data.categories.contains(&"PromptMate 提示词库".to_string()),
            "导出文件头不得被当成分类"
        );
        assert_eq!(data.prompts[0].category, "默认");
    }

    // ---------- import_json：四种形态与去重 ----------

    #[test]
    fn import_json_accepts_four_payload_shapes() {
        let prompt_json = r#"{"id":"x1","title":"T","content":"C","category":"开发"}"#;
        let shapes = [
            format!("[{prompt_json}]"),
            format!(r#"{{"prompts": [{prompt_json}]}}"#),
            format!(r#"{{"data": [{prompt_json}]}}"#),
            format!(r#"{{"app":"PromptMate","version":1,"prompts": [{prompt_json}], "categories": ["开发"]}}"#),
        ];
        for shape in shapes {
            let mut data = AppData::default();
            let (added, skipped) = import_json(&mut data, &shape).expect("应解析成功");
            assert_eq!((added, skipped), (1, 0), "形态应收录 1 条: {shape}");
            assert_eq!(data.prompts[0].title, "T");
        }
    }

    #[test]
    fn import_json_rejects_malformed_and_missing_prompts() {
        let mut data = AppData::default();
        let err = import_json(&mut data, "这不是JSON").unwrap_err();
        assert!(err.contains("JSON 解析失败"), "实际错误: {err}");

        let err = import_json(&mut data, r#"{"foo": 1}"#).unwrap_err();
        assert!(err.contains("未找到 prompts"), "实际错误: {err}");

        let err = import_json(&mut data, r#"{"prompts": 42}"#).unwrap_err();
        assert!(err.contains("未找到 prompts"), "prompts 非数组也应报错: {err}");
    }

    #[test]
    fn import_json_dedups_by_id_and_by_category_title() {
        let mut data = AppData::default();
        data.prompts.push(sample_prompt("x1", "已有", "开发", "旧正文"));

        let input = r#"[
            {"id":"x1","title":"另一个标题","content":"C"},
            {"id":"x2","title":"已有","category":"开发","content":"C"},
            {"id":"x3","title":"全新","content":"C"}
        ]"#;
        let (added, skipped) = import_json(&mut data, input).unwrap();
        assert_eq!((added, skipped), (1, 2), "同 id 与同（分类,标题）都应跳过");
        assert_eq!(data.prompts.len(), 2);
    }

    #[test]
    fn import_json_lenient_mode_needs_title_and_content() {
        let mut data = AppData::default();
        let (added, skipped) =
            import_json(&mut data, r#"[{"title":"只有标题","text":"text 字段正文"}]"#).unwrap();
        assert_eq!((added, skipped), (1, 0));
        assert_eq!(data.prompts[0].content, "text 字段正文");
        assert!(!data.prompts[0].id.is_empty(), "宽松模式必须生成新 id");

        let (added, skipped) = import_json(&mut data, r#"[{"title":"缺正文"}]"#).unwrap();
        assert_eq!((added, skipped), (0, 1), "缺正文的条目应跳过");
    }

    #[test]
    fn import_json_lenient_mode_preserves_id_for_dedup() {
        // 回归：宽松模式（缺 category 等字段）曾丢弃传入 id，导致同 id 条目重复导入
        let mut data = AppData::default();
        data.prompts.push(sample_prompt("dup1", "已有", "开发", "旧正文"));
        let (added, skipped) = import_json(
            &mut data,
            r#"[{"id":"dup1","title":"换了标题的重复条目","content":"C"}]"#,
        )
        .unwrap();
        assert_eq!((added, skipped), (0, 1), "同 id 条目即使缺分类也必须跳过");
        assert_eq!(data.prompts.len(), 1);
        assert_eq!(data.prompts[0].title, "已有", "原有条目不得被改动");
    }

    #[test]
    fn import_json_dedups_within_single_file_by_id() {
        // 回归（P2-1）：去重集合是循环前快照，同文件内同 id 的两条
        // 会双双入库产生重复 id，按 id 查找的保存/删除只会命中第一条
        let mut data = AppData::default();
        let (added, skipped) = import_json(
            &mut data,
            r#"[
                {"id":"dup","title":"第一条","content":"C1"},
                {"id":"dup","title":"第二条","content":"C2"}
            ]"#,
        )
        .unwrap();
        assert_eq!((added, skipped), (1, 1), "同文件内同 id 只收第一条");
        assert_eq!(data.prompts.len(), 1);
        assert_eq!(data.prompts[0].title, "第一条");
    }

    #[test]
    fn import_json_dedups_same_category_title_within_file() {
        let mut data = AppData::default();
        let (added, skipped) = import_json(
            &mut data,
            r#"[
                {"id":"a","title":"同名","category":"开发","content":"C1"},
                {"id":"b","title":"同名","category":"开发","content":"C2"},
                {"id":"c","title":"同名","category":"写作","content":"C3"}
            ]"#,
        )
        .unwrap();
        assert_eq!(
            (added, skipped),
            (2, 1),
            "同文件内同（分类,标题）去重，不同分类不受影响"
        );
        assert_eq!(data.prompts.len(), 2);
    }

    #[test]
    fn import_json_fills_empty_title_and_category() {
        let mut data = AppData::default();
        import_json(&mut data, r#"[{"id":"e1","title":"","content":"C"}]"#).unwrap();
        assert_eq!(data.prompts[0].title, "导入提示词 1");
        assert_eq!(data.prompts[0].category, "未分类");
        assert!(data.categories.contains(&"未分类".to_string()));
    }

    #[test]
    fn import_json_merges_tombstones_taking_max() {
        let mut data = AppData::default();
        data.tombstones.push(Tombstone { id: "t1".into(), at: 500 });

        let input = r#"{
            "prompts": [],
            "tombstones": [
                {"id": "t1", "at": 300},
                {"id": "t2", "at": 700},
                {"id": "", "at": 900},
                {"id": "t3"}
            ]
        }"#;
        import_json(&mut data, input).unwrap();
        assert_eq!(data.tombstones.len(), 2, "空 id / 缺 at 的墓碑应被忽略");
        let t1 = data.tombstones.iter().find(|t| t.id == "t1").unwrap();
        assert_eq!(t1.at, 500, "已有墓碑取较新时间");
        assert!(data.tombstones.iter().any(|t| t.id == "t2" && t.at == 700));
    }

    #[test]
    fn import_json_caps_tombstones_at_5000() {
        let mut data = AppData::default();
        let ts: Vec<Value> = (0..5010)
            .map(|i| serde_json::json!({"id": format!("t{i}"), "at": i}))
            .collect();
        let input = serde_json::json!({ "prompts": [], "tombstones": ts }).to_string();
        import_json(&mut data, &input).unwrap();
        assert_eq!(data.tombstones.len(), 5000);
    }

    #[test]
    fn import_json_on_real_repo_sample_file() {
        // 用仓库里随版本维护的真实导入样例做集成验证
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../dev-workflow-prompts.import.json");
        let raw = std::fs::read_to_string(path).expect("样例文件应存在");
        let expected: Value = serde_json::from_str(&raw).unwrap();
        let expected_count = expected["prompts"].as_array().unwrap().len();

        let mut data = AppData::default();
        let (added, skipped) = import_json(&mut data, &raw).expect("样例文件必须可导入");
        assert_eq!(added, expected_count, "样例中的每条提示词都应被收录");
        assert_eq!(skipped, 0);
        for p in &data.prompts {
            assert!(!p.title.is_empty());
            assert!(!p.content.is_empty());
            assert!(data.categories.iter().any(|c| c == &p.category));
        }
    }

    // ---------- import_text ----------

    #[test]
    fn import_text_rejects_blank_content() {
        let mut data = AppData::default();
        assert!(!import_text(&mut data, "标题", "   \n\t "));
        assert!(data.prompts.is_empty());
    }

    #[test]
    fn import_text_adds_prompt_under_uncategorized() {
        let mut data = AppData::default();
        assert!(import_text(&mut data, "便签", "便签正文"));
        assert_eq!(data.prompts.len(), 1);
        assert_eq!(data.prompts[0].title, "便签");
        assert_eq!(data.prompts[0].content, "便签正文");
        assert_eq!(data.prompts[0].category, "未分类");
    }
}

