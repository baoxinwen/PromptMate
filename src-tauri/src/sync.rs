use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use base64::Engine;
use serde_json::Value;
use tauri::{AppHandle, Emitter};

use crate::hotkey;
use crate::models::{Settings, SyncPayload, SyncReport};
use crate::store::lock;

const SYNC_FILE: &str = "promptmate-sync.json";
const TIMEOUT: Duration = Duration::from_secs(20);
const GIST_API: &str = "https://api.github.com/gists";
const USER_API: &str = "https://api.github.com/user";

/// 防止手动同步与自动同步并发重入
static SYNCING: AtomicBool = AtomicBool::new(false);

struct HttpResponse {
    status: u16,
    body: String,
    /// classic PAT 的 X-OAuth-Scopes 响应头（说明该 token 拥有的作用域）
    scopes: Option<String>,
}

fn collect_response(result: Result<ureq::Response, ureq::Error>) -> Result<HttpResponse, String> {
    match result {
        Ok(r) => {
            let status = r.status();
            let scopes = r.header("x-oauth-scopes").map(|s| s.to_string());
            let body = r.into_string().unwrap_or_default();
            Ok(HttpResponse { status, body, scopes })
        }
        // ureq 把非 2xx 当错误返回，这里统一还原成带状态码的响应交给调用方处理
        Err(ureq::Error::Status(status, r)) => {
            let scopes = r.header("x-oauth-scopes").map(|s| s.to_string());
            let body = r.into_string().unwrap_or_default();
            Ok(HttpResponse { status, body, scopes })
        }
        Err(e) => Err(format!("网络请求失败: {e}")),
    }
}

// ---------- WebDAV 后端 ----------

#[derive(Clone)]
struct DavClient {
    agent: ureq::Agent,
    base_url: String,
    username: String,
    password: String,
}

impl DavClient {
    fn new(url: &str, username: &str, password: &str) -> Self {
        Self {
            agent: ureq::AgentBuilder::new().timeout(TIMEOUT).build(),
            base_url: url.trim().trim_end_matches('/').to_string(),
            username: username.to_string(),
            password: password.to_string(),
        }
    }

    fn basic_auth(&self) -> String {
        let raw = format!("{}:{}", self.username, self.password);
        format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(raw)
        )
    }

    fn request(
        &self,
        method: &str,
        url: &str,
        body: Option<String>,
    ) -> Result<HttpResponse, String> {
        let method = method.to_uppercase();
        let req = self
            .agent
            .request(&method, url)
            .set("Authorization", &self.basic_auth());
        let result = match body {
            Some(b) => req
                .set("Content-Type", "application/json; charset=utf-8")
                .send_string(&b),
            None => req.call(),
        };
        collect_response(result)
    }

    /// 解析出 scheme://host[:port] 与路径段
    fn split_url(&self) -> Result<(String, Vec<String>), String> {
        let (scheme, rest) = self
            .base_url
            .split_once("://")
            .ok_or("WebDAV 地址无效（缺少 http(s):// 前缀）")?;
        let (host, path) = match rest.split_once('/') {
            Some((h, p)) => (h.to_string(), p.to_string()),
            None => (rest.to_string(), String::new()),
        };
        let segments: Vec<String> = path
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        Ok((format!("{scheme}://{host}"), segments))
    }

    /// 确保 WebDAV 目录存在：逐级 MKCOL（已存在 405 / 冲突 409 视为成功）
    fn ensure_dir(&self) -> Result<(), String> {
        let (origin, segments) = self.split_url()?;
        let mut path = String::new();
        for seg in segments.iter() {
            path.push('/');
            path.push_str(seg);
            let resp = self.request("MKCOL", &format!("{origin}{path}"), None)?;
            if (300..400).contains(&resp.status) {
                return Err(format!(
                    "网盘地址发生重定向 (HTTP {})，请改用重定向后的最终地址（通常需 https://）",
                    resp.status
                ));
            }
            let ok = (200..300).contains(&resp.status) || resp.status == 405 || resp.status == 409;
            if !ok {
                return Err(format!(
                    "创建网盘目录失败 (HTTP {})，请在网盘中手动创建该目录",
                    resp.status
                ));
            }
        }
        Ok(())
    }

    fn file_url(&self) -> String {
        format!("{}/{}", self.base_url, SYNC_FILE)
    }

    fn fetch(&self) -> Result<Option<SyncPayload>, String> {
        self.ensure_dir()?;
        let resp = self.request("GET", &self.file_url(), None)?;
        if resp.status == 404 {
            return Ok(None);
        }
        if !(200..300).contains(&resp.status) {
            return Err(format!(
                "下载云端数据失败 (HTTP {})，请检查地址与账号",
                resp.status
            ));
        }
        if resp.body.trim().is_empty() {
            return Ok(None);
        }
        serde_json::from_str::<SyncPayload>(&resp.body)
            .map(Some)
            .map_err(|e| format!("云端数据解析失败: {e}"))
    }

    fn upload(&self, payload: &SyncPayload) -> Result<Option<String>, String> {
        self.ensure_dir()?;
        let json = serde_json::to_string(payload).map_err(|e| e.to_string())?;
        let resp = self.request("PUT", &self.file_url(), Some(json))?;
        if (200..300).contains(&resp.status) {
            Ok(None)
        } else {
            Err(format!("上传失败 (HTTP {})，请确认目录可写", resp.status))
        }
    }

    fn test(&self) -> Result<String, String> {
        self.ensure_dir()?;
        let probe = format!("{}/.promptmate-probe", self.base_url);
        self.request("PUT", &probe, Some("ok".into()))?;
        let body = self.request("GET", &probe, None)?.body;
        let _ = self.request("DELETE", &probe, None);
        if body.contains("ok") {
            Ok("连接成功，目录可读写".into())
        } else {
            Err("目录可访问，但读写校验失败".into())
        }
    }
}

// ---------- GitHub Gist 后端 ----------

#[derive(Clone)]
struct GistClient {
    agent: ureq::Agent,
    token: String,
    gist_id: Option<String>,
}

impl GistClient {
    fn new(token: &str, gist_id: &str) -> Self {
        Self {
            agent: ureq::AgentBuilder::new().timeout(TIMEOUT).build(),
            token: token.trim().to_string(),
            gist_id: {
                let id = gist_id.trim();
                (!id.is_empty()).then(|| id.to_string())
            },
        }
    }

    fn request(
        &self,
        method: &str,
        url: &str,
        body: Option<String>,
    ) -> Result<HttpResponse, String> {
        let method = method.to_uppercase();
        let req = self
            .agent
            .request(&method, url)
            .set("Authorization", &format!("Bearer {}", self.token))
            .set("User-Agent", "PromptMate")
            .set("Accept", "application/vnd.github+json");
        let result = match body {
            Some(b) => req
                .set("Content-Type", "application/json; charset=utf-8")
                .send_string(&b),
            None => req.call(),
        };
        collect_response(result)
    }

    /// 从 GET /gists/{id} 响应中取出数据文件内容（过大时回退 raw_url）
    fn extract_content(&self, body: &str) -> Result<SyncPayload, String> {
        let v: Value =
            serde_json::from_str(body).map_err(|e| format!("Gist 响应解析失败: {e}"))?;
        let file = v
            .get("files")
            .and_then(|f| f.get(SYNC_FILE))
            .ok_or("Gist 中未找到数据文件（可能不是 PromptMate 的同步 Gist）")?;
        let truncated = file
            .get("truncated")
            .and_then(|t| t.as_bool())
            .unwrap_or(false);
        let content = if truncated {
            let raw = file
                .get("raw_url")
                .and_then(|u| u.as_str())
                .ok_or("数据文件过大且缺少 raw_url")?;
            self.request("GET", raw, None)?.body
        } else {
            file.get("content")
                .and_then(|c| c.as_str())
                .ok_or("数据文件内容为空")?
                .to_string()
        };
        serde_json::from_str::<SyncPayload>(&content)
            .map_err(|e| format!("云端数据解析失败: {e}"))
    }

    fn fetch(&self) -> Result<Option<SyncPayload>, String> {
        let Some(id) = &self.gist_id else {
            // 尚无 Gist，视为云端暂无数据（首次同步将创建）
            return Ok(None);
        };
        let resp = self.request("GET", &format!("{GIST_API}/{id}"), None)?;
        if resp.status == 404 {
            return Err("Gist 不存在或 Token 无权访问（请检查 Gist ID）".into());
        }
        if !(200..300).contains(&resp.status) {
            return Err(gist_err(resp.status, "读取 Gist"));
        }
        self.extract_content(&resp.body).map(Some)
    }

    fn upload(&self, payload: &SyncPayload) -> Result<Option<String>, String> {
        let content = serde_json::to_string(payload).map_err(|e| e.to_string())?;
        // Gist API 对大文件会返回难懂的 422，这里提前给出可行动的错误
        if content.len() > 9_000_000 {
            return Err(
                "同步数据超过 9MB（通常是剪贴板历史过大），请在「剪贴板」页清理历史后重试"
                    .into(),
            );
        }
        let body = gist_body(&content);
        match &self.gist_id {
            Some(id) => {
                let resp = self.request("PATCH", &format!("{GIST_API}/{id}"), Some(body))?;
                if (200..300).contains(&resp.status) {
                    Ok(None)
                } else {
                    Err(gist_err(resp.status, "更新 Gist"))
                }
            }
            None => {
                let resp = self.request("POST", GIST_API, Some(body))?;
                if !(200..300).contains(&resp.status) {
                    return Err(gist_err(resp.status, "创建 Gist"));
                }
                let v: Value = serde_json::from_str(&resp.body)
                    .map_err(|e| format!("创建 Gist 响应解析失败: {e}"))?;
                let id = v
                    .get("id")
                    .and_then(|i| i.as_str())
                    .ok_or("创建 Gist 响应中缺少 id")?
                    .to_string();
                Ok(Some(id))
            }
        }
    }

    fn test(&self) -> Result<String, String> {
        let resp = self.request("GET", USER_API, None)?;
        if !(200..300).contains(&resp.status) {
            return Err(gist_err(resp.status, "验证 Token"));
        }
        // classic PAT 会携带 X-OAuth-Scopes：缺 gist 作用域时写操作必然失败，
        // 必须在这里拦下（GET /user 本身不需要任何作用域，会造成"测试成功"的假象）
        if let Some(scopes) = &resp.scopes {
            let has_gist = scopes
                .split(',')
                .any(|s| s.trim().eq_ignore_ascii_case("gist"));
            if !has_gist {
                return Err(
                    "Token 缺少 gist 作用域：请在 GitHub 生成 Token 时勾选 gist 权限后重试"
                        .into(),
                );
            }
        }
        let login = serde_json::from_str::<Value>(&resp.body)
            .ok()
            .and_then(|v| v.get("login").and_then(|l| l.as_str()).map(String::from))
            .unwrap_or_default();
        let account = if login.is_empty() {
            "Token 有效".to_string()
        } else {
            format!("Token 有效（账号 {login}）")
        };
        match &self.gist_id {
            Some(id) => {
                let r = self.request("GET", &format!("{GIST_API}/{id}"), None)?;
                if r.status == 404 {
                    return Err(format!("{account}，但无法访问该 Gist（不存在或无权限）"));
                }
                if !(200..300).contains(&r.status) {
                    return Err(gist_err(r.status, "验证 Gist"));
                }
                Ok(format!("{account}，Gist 可访问"))
            }
            None => Ok(format!("{account}，首次同步时将自动创建 Gist")),
        }
    }
}

fn gist_err(status: u16, ctx: &str) -> String {
    match status {
        401 => "GitHub Token 无效（可能已在 GitHub 侧被重新生成或删除）——请重新生成 Token 并在同步设置中更新".into(),
        403 => "GitHub API 拒绝访问（检查 Token 的 gist 权限或速率限制）".into(),
        404 => "Gist 不存在或 Token 无权访问".into(),
        422 => "GitHub 拒绝了请求（数据格式或参数错误）".into(),
        s => format!("GitHub 请求失败 (HTTP {s})，{ctx}"),
    }
}

fn gist_body(content: &str) -> String {
    let mut files = serde_json::Map::new();
    files.insert(
        SYNC_FILE.to_string(),
        serde_json::json!({ "content": content }),
    );
    serde_json::json!({
        "description": "PromptMate sync data",
        "public": false,
        "files": Value::Object(files),
    })
    .to_string()
}

// ---------- 后端抽象 ----------

enum SyncBackend {
    Dav(DavClient),
    Gist(GistClient),
}

impl SyncBackend {
    fn fetch(&self) -> Result<Option<SyncPayload>, String> {
        match self {
            SyncBackend::Dav(c) => c.fetch(),
            SyncBackend::Gist(c) => c.fetch(),
        }
    }

    /// 返回 Ok(Some(id)) 表示新建了 Gist，需要把 id 写回配置
    fn upload(&self, payload: &SyncPayload) -> Result<Option<String>, String> {
        match self {
            SyncBackend::Dav(c) => c.upload(payload),
            SyncBackend::Gist(c) => c.upload(payload),
        }
    }
}

fn select_backend(settings: &Settings) -> Result<SyncBackend, String> {
    match settings.sync_provider.as_str() {
        "gist" => {
            if settings.gist.token.trim().is_empty() {
                return Err("尚未配置 GitHub Token，请在「云同步」页填写".into());
            }
            Ok(SyncBackend::Gist(GistClient::new(
                &settings.gist.token,
                &settings.gist.gist_id,
            )))
        }
        _ => {
            if settings.webdav.url.trim().is_empty() {
                return Err("尚未配置 WebDAV 地址，请在「云同步」页填写".into());
            }
            Ok(SyncBackend::Dav(DavClient::new(
                &settings.webdav.url,
                &settings.webdav.username,
                &settings.webdav.password,
            )))
        }
    }
}

/// 条目级合并：内容按 updated_at 取新者；使用统计（use_count/last_used_at）
/// 单调收敛取 max——它们的变化不 bump updated_at，不能参与 LWW，否则
/// 一端的使用计数永远传不到另一端，且会被另一端的内容编辑整体覆盖。
fn merge(local: &mut crate::models::AppData, remote: &SyncPayload) -> (u64, u64, u64) {
    let mut added: u64 = 0;
    let mut updated: u64 = 0;

    for c in &remote.categories {
        local.ensure_category(c);
    }

    for rp in &remote.prompts {
        match local.prompts.iter_mut().find(|p| p.id == rp.id) {
            None => {
                local.prompts.push(rp.clone());
                added += 1;
            }
            Some(lp) => {
                if rp.updated_at > lp.updated_at {
                    let use_count = lp.use_count.max(rp.use_count);
                    let last_used_at = lp.last_used_at.max(rp.last_used_at);
                    *lp = rp.clone();
                    lp.use_count = use_count;
                    lp.last_used_at = last_used_at;
                    updated += 1;
                } else if rp.use_count > lp.use_count || rp.last_used_at > lp.last_used_at {
                    lp.use_count = lp.use_count.max(rp.use_count);
                    lp.last_used_at = lp.last_used_at.max(rp.last_used_at);
                }
            }
        }
    }

    for ri in &remote.clipboard {
        match local.clipboard.iter_mut().find(|i| i.id == ri.id) {
            None => {
                local.clipboard.push(ri.clone());
                added += 1;
            }
            Some(li) => {
                if ri.copied_at > li.copied_at {
                    *li = ri.clone();
                }
            }
        }
    }

    for rt in &remote.tombstones {
        match local.tombstones.iter_mut().find(|t| t.id == rt.id) {
            None => local.tombstones.push(rt.clone()),
            Some(lt) => lt.at = lt.at.max(rt.at),
        }
    }

    let before = local.prompts.len() + local.clipboard.len();
    let tombstones = local.tombstones.clone();
    local
        .prompts
        .retain(|p| !tombstones.iter().any(|t| t.id == p.id && t.at > p.updated_at));
    local
        .clipboard
        .retain(|i| !tombstones.iter().any(|t| t.id == i.id && t.at > i.copied_at));
    let removed = (before - (local.prompts.len() + local.clipboard.len())) as u64;

    // 双端剪贴板做并集可能突破上限，收敛到统一上限（保留最新）
    if local.clipboard.len() > crate::models::MAX_CLIPBOARD_ITEMS {
        local
            .clipboard
            .sort_by_key(|i| std::cmp::Reverse(i.copied_at));
        local
            .clipboard
            .truncate(crate::models::MAX_CLIPBOARD_ITEMS);
    }

    (added, updated, removed)
}

/// 执行同步。direction: "merge"（双向合并，默认）| "push"（本机覆盖云端）| "pull"（云端覆盖本机）
pub fn run_sync(app: &AppHandle, direction: &str) -> Result<SyncReport, String> {
    if SYNCING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("已有同步在进行中，请稍候".into());
    }
    let result = run_sync_inner(app, direction);
    SYNCING.store(false, Ordering::SeqCst);
    result
}

fn run_sync_inner(app: &AppHandle, direction: &str) -> Result<SyncReport, String> {
    let (backend, payload, snap_mark, sync_clipboard) = {
        let store = lock(app);
        (
            select_backend(&store.data.settings)?,
            SyncPayload::from(&store.data),
            store.mutations,
            store.data.settings.sync_clipboard,
        )
    };

    match direction {
        "push" => {
            let new_gist = backend.upload(&payload)?;
            persist_new_gist_id(app, new_gist)?;
            // 上传期间若有新变更，保持 dirty，让下个自动同步周期补传
            let mut store = lock(app);
            if store.mutations == snap_mark {
                store.dirty_unsynced = false;
            }
            Ok(SyncReport {
                added: 0,
                updated: 0,
                removed: 0,
                message: format!("已上传本机数据（{} 条提示词）", payload.prompts.len()),
            })
        }
        "pull" => {
            let remote = backend.fetch()?.ok_or("云端暂无数据")?;
            {
                let mut store = lock(app);
                store.data.categories = remote.categories.clone();
                store.data.prompts = remote.prompts.clone();
                // 云端载荷不含图片条目：整体替换会清掉本机图片历史。
                // 替换范围只限同步作用域：开启剪贴板同步时取云端文本条目；
                // 关闭时文本剪贴板是纯本地数据，不在"云端覆盖"范围内，原样保留
                let mut clip = if sync_clipboard {
                    remote.clipboard.iter().cloned().collect::<Vec<_>>()
                } else {
                    store
                        .data
                        .clipboard
                        .iter()
                        .filter(|i| !i.is_image())
                        .cloned()
                        .collect()
                };
                clip.extend(store.data.clipboard.iter().filter(|i| i.is_image()).cloned());
                clip.sort_by_key(|i| std::cmp::Reverse(i.copied_at));
                clip.truncate(crate::models::MAX_CLIPBOARD_ITEMS);
                store.data.clipboard = clip;
                store.data.tombstones = remote.tombstones.clone();
                store.dirty_unsynced = false;
                store.save()?;
            }
            let _ = app.emit("data-changed", ());
            Ok(SyncReport {
                added: remote.prompts.len() as u64,
                updated: 0,
                removed: 0,
                message: format!("已下载云端数据（{} 条提示词）", remote.prompts.len()),
            })
        }
        _ => {
            let mut remote = backend.fetch()?;
            // 用户关闭剪贴板同步时，不把云端剪贴板合并进本地
            if let Some(r) = &mut remote {
                if !sync_clipboard {
                    r.clipboard.clear();
                }
            }
            let (added, updated, removed, detail) = match &remote {
                Some(r) => {
                    let mut store = lock(app);
                    let (a, u, rm) = merge(&mut store.data, r);
                    hotkey::sanitize_prompt_hotkeys(&mut store.data);
                    (a, u, rm, format!("新增 {a} 条，更新 {u} 条，删除 {rm} 条"))
                }
                None => (0, 0, 0, "云端暂无数据，已上传本机数据".to_string()),
            };
            // 上传放在锁外，避免网络请求阻塞剪贴板监听等持锁方
            let (merged, merged_mark) = {
                let store = lock(app);
                (SyncPayload::from(&store.data), store.mutations)
            };
            let new_gist = backend.upload(&merged)?;
            persist_new_gist_id(app, new_gist)?;
            {
                let mut store = lock(app);
                store.data.seeded = true;
                // 快照后又有本地变更：不能清 dirty，否则这次变更在下次修改前都不会再上传
                if store.mutations == merged_mark {
                    store.dirty_unsynced = false;
                }
                store.save()?;
            }
            if remote.is_some() {
                let _ = app.emit("data-changed", ());
            }
            Ok(SyncReport {
                added,
                updated,
                removed,
                message: format!("同步完成：{detail}，并已上传云端"),
            })
        }
    }
}

/// 首次上传时自动创建的 Gist id 写回配置
fn persist_new_gist_id(app: &AppHandle, new_gist: Option<String>) -> Result<(), String> {
    let Some(id) = new_gist else {
        return Ok(());
    };
    {
        let mut store = lock(app);
        store.data.settings.gist.gist_id = id;
        store.save()?;
    }
    let _ = app.emit("data-changed", ());
    Ok(())
}

/// WebDAV 连接测试（供 webdav_test 命令使用）
pub fn test_connection(url: &str, username: &str, password: &str) -> Result<String, String> {
    DavClient::new(url, username, password).test()
}

/// GitHub Gist 连接测试（供 gist_test 命令使用）
pub fn gist_test(token: &str, gist_id: &str) -> Result<String, String> {
    if token.trim().is_empty() {
        return Err("请先填写 GitHub Token".into());
    }
    GistClient::new(token, gist_id).test()
}

/// 自动同步：轮询 dirty 标志，按 provider 配置后台合并同步。
/// 成功静默（前端有 data-changed 即可）；失败通过 sync-done 事件上报，
/// 并按连续失败次数退避，避免离线时每 15s 一轮、每轮 20s 超时的空转
pub fn spawn_auto_sync(app: AppHandle) {
    std::thread::spawn(move || {
        let mut first_round = true;
        let mut failures: u32 = 0;
        loop {
            let backoff = Duration::from_secs(15) * failures.min(16);
            let wait = if first_round {
                Duration::from_secs(10)
            } else {
                Duration::from_secs(15) + backoff
            };
            std::thread::sleep(wait);

            let (auto_on, dirty) = {
                let store = lock(&app);
                let s = &store.data.settings;
                let auto_on = match s.sync_provider.as_str() {
                    "gist" => s.gist.enabled && s.gist.auto_sync,
                    _ => s.webdav.enabled && s.webdav.auto_sync,
                };
                (auto_on, store.dirty_unsynced)
            };

            if auto_on && (first_round || dirty) {
                match run_sync(&app, "merge") {
                    Ok(_) => failures = 0,
                    Err(e) => {
                        // 手动同步占用不算自动同步失败，不计入退避
                        if e.contains("已有同步在进行中") {
                            eprintln!("[promptmate] 自动同步跳过（手动同步进行中）");
                        } else {
                            failures = failures.saturating_add(1);
                            eprintln!("[promptmate] 自动同步失败: {e}");
                            let _ = app.emit(
                                "sync-done",
                                serde_json::json!({
                                    "added": 0, "updated": 0, "removed": 0,
                                    "message": e, "ok": false, "auto": true,
                                }),
                            );
                        }
                    }
                }
            }
            first_round = false;
        }
    });
}
