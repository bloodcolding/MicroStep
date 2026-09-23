//! Git 远端同步引擎（data-sync spec）：SyncConfig（sync.json）+ Union Merge
//! 纯函数 + SyncEngine（fetch → 应用层合并 → 快照 commit → push）。
//!
//! 双栈（design D1 / ADR-004）：gix（fetch / blob 读写 / 对象写入，纯 Rust）
//! + git2-rs（push，libgit2 进程内绑定）；零子进程，app-shell 移动端就绪约束不破坏。
//! 并发模型（D6）：同步全程持 EventStore Mutex（含网络阶段），业务写阻塞等待。
//! 合并语义（D3）：本地序保留 + 远端独有追加 + 按 event_id 语义相等去重 +
//! 同 id 内容冲突整次拒绝（事件不可变红线）。commit 仅同步时写（D4 快照式）。

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use chrono::Local;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::store::EventStore;

/// 网络操作默认超时（spec 同步流程：30 秒）。
pub const DEFAULT_SYNC_TIMEOUT: Duration = Duration::from_secs(30);
/// push non-fast-forward 重跑 fetch+merge 的重试上限（spec：上限 2 次）。
const MAX_PUSH_RETRIES: usize = 2;
const EVENTS_FILE: &str = "events.jsonl";
const SYNC_CONFIG_FILE: &str = "sync.json";

// ---------------------------------------------------------------------------
// 同步配置（sync.json）
// ---------------------------------------------------------------------------

/// 远端同步配置（明文 PAT，design D5；未配置 remote_url = 同步未启用）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    #[serde(default)]
    pub remote_url: String,
    #[serde(default)]
    pub pat: String,
    #[serde(default = "default_branch")]
    pub branch: String,
    #[serde(default)]
    pub last_sync_at: Option<String>,
    #[serde(default)]
    pub last_result: Option<String>,
}

fn default_branch() -> String {
    "main".to_string()
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            remote_url: String::new(),
            pat: String::new(),
            branch: default_branch(),
            last_sync_at: None,
            last_result: None,
        }
    }
}

/// 读取 sync.json：文件不存在 / 损坏 → 默认配置（同步未启用态，不报错）。
pub fn load_sync_config(data_dir: &Path) -> SyncConfig {
    fs::read_to_string(data_dir.join(SYNC_CONFIG_FILE))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

/// 写入 sync.json（与 events.jsonl 同目录）。
pub fn save_sync_config(data_dir: &Path, cfg: &SyncConfig) -> io::Result<()> {
    fs::create_dir_all(data_dir)?;
    let raw = serde_json::to_string_pretty(cfg).map_err(io::Error::other)?;
    fs::write(data_dir.join(SYNC_CONFIG_FILE), raw)
}

/// 部分更新：None = 字段缺省（保持不变）；pat 显式空串 = 清除既有 PAT。
#[derive(Debug, Clone, Default)]
pub struct SyncConfigUpdate {
    pub remote_url: Option<String>,
    pub pat: Option<String>,
    pub branch: Option<String>,
}

/// 应用部分更新（未携带字段不动；branch 空串忽略以保同步分支恒非空）。
pub fn apply_sync_config_update(cfg: &mut SyncConfig, update: &SyncConfigUpdate) {
    if let Some(url) = &update.remote_url {
        cfg.remote_url = url.clone();
    }
    if let Some(pat) = &update.pat {
        cfg.pat = pat.clone();
    }
    if let Some(branch) = &update.branch {
        if !branch.trim().is_empty() {
            cfg.branch = branch.trim().to_string();
        }
    }
}

/// PAT 脱敏：长度 > 4 仅保留末 4 位；≤ 4 全掩码（不完整泄露）。
pub fn mask_pat(pat: &str) -> String {
    let chars: Vec<char> = pat.chars().collect();
    if chars.is_empty() {
        return String::new();
    }
    if chars.len() <= 4 {
        return "*".repeat(chars.len());
    }
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("****{tail}")
}

// ---------------------------------------------------------------------------
// Union Merge（纯函数，D3）
// ---------------------------------------------------------------------------

/// 合并统计：pulled = 追加的远端独有行；pushed = 本地独有行；merged = 双端同 id 去重数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeOutcome {
    pub content: String,
    pub pulled: usize,
    pub pushed: usize,
    pub merged: usize,
}

/// 应用层 union merge：本地全部行按原序保留；远端独有行（id 不在本地集）按远端
/// 相对序追加；同 id 语义相等去重（保留本地字节形态）；同 id 内容不同整次拒绝。
/// 无法解析的行：本地原样保留；远端仅当字节级不存在于本地时追加。输出确定性。
pub fn union_merge(local: &str, remote: &str) -> Result<MergeOutcome, SyncError> {
    let mut local_ids: HashMap<String, Value> = HashMap::new();
    for line in local.lines() {
        if let Some(event) = parse_event(line) {
            if let Some(id) = event_id(&event) {
                local_ids.entry(id).or_insert(event);
            }
        }
    }
    let remote_ids: HashSet<String> = remote
        .lines()
        .filter_map(parse_event)
        .filter_map(|event| event_id(&event))
        .collect();

    let mut pushed = 0usize;
    for line in local.lines() {
        if let Some(event) = parse_event(line) {
            if let Some(id) = event_id(&event) {
                if !remote_ids.contains(&id) {
                    pushed += 1;
                }
            }
        }
    }

    let mut content = String::from(local);
    let mut seen_lines: HashSet<String> = local.lines().map(str::to_string).collect();
    let mut pulled = 0usize;
    let mut merged = 0usize;
    for line in remote.lines() {
        let id = parse_event(line).and_then(|event| event_id(&event));
        match id {
            Some(id) => {
                let remote_event = parse_event(line).expect("上方已解析成功");
                match local_ids.get(&id) {
                    None => {
                        append_line(&mut content, line);
                        pulled += 1;
                        // 远端重复行在本次合并内继续去重。
                        local_ids.insert(id, remote_event);
                    }
                    Some(local_event) => {
                        if local_event == &remote_event {
                            merged += 1;
                        } else {
                            return Err(SyncError::Conflict { event_id: id });
                        }
                    }
                }
            }
            None => {
                // 无 id / 无法解析的远端行：字节级比对。
                if seen_lines.insert(line.to_string()) {
                    append_line(&mut content, line);
                    pulled += 1;
                }
            }
        }
    }

    Ok(MergeOutcome { content, pulled, pushed, merged })
}

fn parse_event(line: &str) -> Option<Value> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    serde_json::from_str::<Value>(trimmed).ok()
}

fn event_id(event: &Value) -> Option<String> {
    event
        .get("event_id")
        .and_then(|id| id.as_str())
        .map(str::to_string)
}

fn append_line(content: &mut String, line: &str) {
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(line);
    content.push('\n');
}

// ---------------------------------------------------------------------------
// 错误分类（spec：网络（含超时）/ 认证 / 冲突 / 远端并发 / 未配置）
// ---------------------------------------------------------------------------

/// 同步错误（message() 即 IPC 信封 error 文案）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncError {
    NotConfigured,
    Network(String),
    Timeout(String),
    Auth(String),
    Conflict { event_id: String },
    ConcurrentUpdate(String),
    Git(String),
    Io(String),
}

impl SyncError {
    /// 信封 error 文案（前端展示用，分类可辨）。
    pub fn message(&self) -> String {
        match self {
            SyncError::NotConfigured => "同步未配置：请先在同步设置中填写远端地址".to_string(),
            SyncError::Network(detail) => format!("网络失败：无法访问远端（{detail}）"),
            SyncError::Timeout(detail) => format!("网络失败：同步超时（{detail}）"),
            SyncError::Auth(detail) => format!("认证失败：请检查 PAT 权限与有效期（{detail}）"),
            SyncError::Conflict { event_id } => format!(
                "同步拒绝：事件 {event_id} 在双端内容冲突（事件不可变红线），双端文件保持原样"
            ),
            SyncError::ConcurrentUpdate(detail) => format!(
                "远端并发更新：已重试 {MAX_PUSH_RETRIES} 次仍冲突，本地已合并结果保留，请稍后再同步（{detail}）"
            ),
            SyncError::Git(detail) => format!("Git 操作失败：{detail}"),
            SyncError::Io(detail) => format!("本地读写失败：{detail}"),
        }
    }

    /// HTTP 状态分类映射（401/403 → 认证失败提示检查 PAT；其余归网络失败）。
    pub fn from_http_status(status: u16) -> Self {
        match status {
            401 | 403 => SyncError::Auth(format!("HTTP {status}")),
            _ => SyncError::Network(format!("HTTP {status}")),
        }
    }
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message())
    }
}

/// 传输层错误文案归类（gix / git2 错误 → 五类信封）。
fn classify_transport(detail: String) -> SyncError {
    let lower = detail.to_lowercase();
    if lower.contains("401")
        || lower.contains("403")
        || lower.contains("authentication")
        || lower.contains("authorization")
        || lower.contains("credentials")
    {
        SyncError::Auth(detail)
    } else if lower.contains("timeout") || lower.contains("timed out") {
        SyncError::Timeout(detail)
    } else {
        SyncError::Network(detail)
    }
}

/// push 阶段错误归类：non-fast-forward / fetch first / rejected → 远端并发更新。
fn classify_push_error(err: git2::Error) -> SyncError {
    let message = err.message().to_string();
    let lower = message.to_lowercase();
    if lower.contains("non-fast-forward") || lower.contains("fetch first") || lower.contains("rejected") {
        SyncError::ConcurrentUpdate(message)
    } else {
        classify_transport(message)
    }
}

// ---------------------------------------------------------------------------
// SyncEngine
// ---------------------------------------------------------------------------

/// 一次同步的统计（attempts 为内部字段：fetch+merge 循环次数，不进 IPC 信封）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncOutcome {
    pub pulled: usize,
    pub pushed: usize,
    pub merged: usize,
    pub attempts: usize,
}

/// 并发测试注入点：push 前夜回调（SyncEngine 全程持锁状态中触发）。
pub struct SyncHooks {
    pub before_push: Option<Box<dyn Fn() + Send>>,
}

impl SyncHooks {
    pub fn noop() -> Self {
        Self { before_push: None }
    }

    pub fn before_push(hook: Box<dyn Fn() + Send>) -> Self {
        Self { before_push: Some(hook) }
    }
}

/// 同步引擎：绑定一个数据目录（.git + sync.json + events.jsonl）。
pub struct SyncEngine {
    data_dir: PathBuf,
    events_path: PathBuf,
    timeout: Duration,
}

impl SyncEngine {
    pub fn new(data_dir: PathBuf, events_path: PathBuf) -> Self {
        Self { data_dir, events_path, timeout: DEFAULT_SYNC_TIMEOUT }
    }

    /// 注入超时（测试用；默认 DEFAULT_SYNC_TIMEOUT）。
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// 完整同步：fetch → merge → 快照 commit → push（未配置 → NotConfigured）。
    pub fn sync_now(&self, store: &Mutex<EventStore>) -> Result<SyncOutcome, SyncError> {
        self.sync_now_with_hooks(store, &SyncHooks::noop())
    }

    /// 同上，带测试钩子。push 遇 non-fast-forward 重跑完整循环，重试上限 2 次。
    pub fn sync_now_with_hooks(
        &self,
        store: &Mutex<EventStore>,
        hooks: &SyncHooks,
    ) -> Result<SyncOutcome, SyncError> {
        let cfg = load_sync_config(&self.data_dir);
        if cfg.remote_url.trim().is_empty() {
            return Err(SyncError::NotConfigured);
        }
        let mut attempts = 0usize;
        loop {
            attempts += 1;
            match self.sync_once(&cfg, store, hooks) {
                Ok(mut outcome) => {
                    outcome.attempts = attempts;
                    self.record_success(&outcome);
                    return Ok(outcome);
                }
                Err(err) if matches!(err, SyncError::ConcurrentUpdate(_))
                    && attempts <= MAX_PUSH_RETRIES =>
                {
                    continue; // 远端并发推进 → 重跑 fetch+merge
                }
                Err(err) => {
                    self.record_failure(&err);
                    return Err(err);
                }
            }
        }
    }

    /// 启动 best-effort pull：fetch → merge → 本地落盘；不 push、不写 commit。
    /// 未配置 → Ok 零统计（跳过）；失败返回 Err 并记 last_result（调用方吞掉）。
    pub fn startup_pull(&self, store: &Mutex<EventStore>) -> Result<SyncOutcome, SyncError> {
        let cfg = load_sync_config(&self.data_dir);
        if cfg.remote_url.trim().is_empty() {
            return Ok(SyncOutcome::default());
        }
        let result = (|| -> Result<SyncOutcome, SyncError> {
            let _guard = store.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let local = fs::read_to_string(&self.events_path).unwrap_or_default();
            let (_, remote_content) = self.fetch_remote(&cfg, &cfg.branch)?;
            let outcome = union_merge(&local, remote_content.as_deref().unwrap_or(""))?;
            if outcome.content != local {
                fs::write(&self.events_path, &outcome.content)
                    .map_err(|err| SyncError::Io(err.to_string()))?;
            }
            Ok(SyncOutcome {
                pulled: outcome.pulled,
                pushed: 0, // 启动 pull 不 push（pushed 恒 0）
                merged: outcome.merged,
                attempts: 1,
            })
        })();
        match &result {
            Ok(outcome) => self.record_success(outcome),
            Err(err) => self.record_failure(err),
        }
        result
    }

    // -- 单轮 fetch+merge+commit+push ------------------------------------

    fn sync_once(
        &self,
        cfg: &SyncConfig,
        store: &Mutex<EventStore>,
        hooks: &SyncHooks,
    ) -> Result<SyncOutcome, SyncError> {
        // D6：全程持锁（含网络阶段），业务写阻塞等待而非报错。
        let _guard = store.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let branch = cfg.branch.as_str();
        let local = fs::read_to_string(&self.events_path).unwrap_or_default();

        let (remote_tip, remote_content) = self.fetch_remote(cfg, branch)?;
        let outcome = union_merge(&local, remote_content.as_deref().unwrap_or(""))?;
        if outcome.content != local {
            fs::write(&self.events_path, &outcome.content)
                .map_err(|err| SyncError::Io(err.to_string()))?;
        }

        let local_tip = self.branch_tip(branch);
        let tip_content = local_tip.and_then(|oid| self.events_content_at(&oid));
        let content_current = tip_content.as_deref() == Some(outcome.content.as_str());

        // 双空且无历史（bootstrap 无事可做）。
        if local_tip.is_none() && remote_tip.is_none() && outcome.content.trim().is_empty() {
            return Ok(stats_of(&outcome, 0));
        }
        // 稳定收敛：集合一致且内容与上次 commit 相同 → 零动作（防顺序乒乓）。
        if outcome.pulled == 0 && outcome.pushed == 0 && content_current {
            return Ok(stats_of(&outcome, 0));
        }

        // D4 快照 commit：内容有变 → 新 commit；内容未变但远端分叉 → 同树合流 commit。
        if !content_current || (remote_tip.is_some() && remote_tip != self.branch_tip(branch)) {
            let local_tip_now = self.branch_tip(branch);
            let commit = self.write_snapshot(
                &outcome.content,
                local_tip_now.as_ref(),
                remote_tip.as_ref(),
            )?;
            self.set_branch_tip(branch, &commit)?;
        }

        if let Some(hook) = &hooks.before_push {
            hook(); // 并发重试 / 持锁测试注入点（锁仍持有）
        }

        // 并发检查：fetch 之后远端又被推进 → non-fast-forward，重跑循环。
        let tip_now = self.remote_tip_now(cfg, branch)?;
        if tip_now != remote_tip {
            return Err(SyncError::ConcurrentUpdate(
                "fetch 之后远端 tip 又被其他设备推进".to_string(),
            ));
        }

        if self.branch_tip(branch).is_none() {
            return Err(SyncError::Git("本地分支 tip 缺失".to_string()));
        }
        self.push(cfg, branch)?;

        Ok(stats_of(&outcome, 0))
    }

    // -- git 传输：fetch（gix）/ push（git2-rs） --------------------------

    /// fetch 远端分支到 refs/remotes/origin/<branch>，返回 (远端 tip, events blob)。
    fn fetch_remote(
        &self,
        cfg: &SyncConfig,
        branch: &str,
    ) -> Result<(Option<gix::ObjectId>, Option<String>), SyncError> {
        let url = authenticated_url(&cfg.remote_url, &cfg.pat);
        let data_dir = self.data_dir.clone();
        let branch = branch.to_string();
        self.run_with_timeout(move || {
            let repo = gix::open(&data_dir).map_err(|err| SyncError::Git(err.to_string()))?;
            let mut remote = repo
                .remote_at(url.as_str())
                .map_err(|err| classify_transport(err.to_string()))?;
            let tracking_ref = format!("refs/remotes/origin/{branch}");
            let refspec = format!("+refs/heads/{branch}:{tracking_ref}");
            remote
                .replace_refspecs(Some(refspec.as_str()), gix::remote::Direction::Fetch)
                .map_err(|err| SyncError::Git(err.to_string()))?;
            let connection = remote
                .connect(gix::remote::Direction::Fetch)
                .map_err(|err| classify_transport(err.to_string()))?;
            let preparation = connection
                .prepare_fetch(gix::progress::Discard, Default::default())
                .map_err(|err| classify_transport(err.to_string()))?;
            // 空远端 / 远端无任何分支：不执行 receive（其 update_refs 阶段会对
            // 0 refs 的 NoMapping 报错），等价「无远端内容」→ bootstrap 推种子路径。
            if preparation.ref_map().remote_refs.is_empty() {
                return Ok((None, None));
            }
            let interrupt = AtomicBool::new(false);
            preparation
                .receive(gix::progress::Discard, &interrupt)
                .map_err(|err| classify_transport(err.to_string()))?;

            let repo = gix::open(&data_dir).map_err(|err| SyncError::Git(err.to_string()))?;
            let tip = repo
                .find_reference(tracking_ref.as_str())
                .ok()
                .and_then(|mut reference| reference.peel_to_id().ok())
                .map(|id| id.detach());
            let content = match tip {
                Some(oid) => read_events_blob(&repo, &oid)?,
                None => None,
            };
            Ok((tip, content))
        })
    }

    /// 当前远端分支 tip（push 前并发复核）：gix 握手 ls-refs（prepare_fetch
    /// 不 receive，零对象写入）；git2 的 Remote::list 在 0 refs 时有空指针 UB 检查崩溃。
    fn remote_tip_now(
        &self,
        cfg: &SyncConfig,
        branch: &str,
    ) -> Result<Option<gix::ObjectId>, SyncError> {
        let url = authenticated_url(&cfg.remote_url, &cfg.pat);
        let data_dir = self.data_dir.clone();
        let branch = branch.to_string();
        let refname = format!("refs/heads/{branch}");
        self.run_with_timeout(move || {
            let repo = gix::open(&data_dir).map_err(|err| SyncError::Git(err.to_string()))?;
            let mut remote = repo
                .remote_at(url.as_str())
                .map_err(|err| classify_transport(err.to_string()))?;
            // 仅握手取 advertisement：refspec 复用追踪映射，不 receive 即不落任何写。
            remote
                .replace_refspecs(
                    Some(format!("+refs/heads/{branch}:refs/remotes/origin/{branch}").as_str()),
                    gix::remote::Direction::Fetch,
                )
                .map_err(|err| SyncError::Git(err.to_string()))?;
            let connection = remote
                .connect(gix::remote::Direction::Fetch)
                .map_err(|err| classify_transport(err.to_string()))?;
            let preparation = connection
                .prepare_fetch(gix::progress::Discard, Default::default())
                .map_err(|err| classify_transport(err.to_string()))?;
            let mut tip = None;
            for reference in &preparation.ref_map().remote_refs {
                if let gix::protocol::handshake::Ref::Direct { full_ref_name, object } = reference {
                    if full_ref_name.as_slice() == refname.as_bytes() {
                        tip = Some(*object);
                        break;
                    }
                }
            }
            Ok(tip)
        })
    }

    /// push 本地分支至远端（git2-rs；认证走内存回调，不落盘）。
    fn push(&self, cfg: &SyncConfig, branch: &str) -> Result<(), SyncError> {
        let url = authenticated_url(&cfg.remote_url, &cfg.pat);
        let pat = cfg.pat.clone();
        let data_dir = self.data_dir.clone();
        let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");
        self.run_with_timeout(move || {
            let repo = git2::Repository::open(data_dir.join(".git"))
                .map_err(|err| SyncError::Git(err.to_string()))?;
            let mut remote = repo
                .remote_anonymous(&url)
                .map_err(|err| classify_transport(err.to_string()))?;
            let mut callbacks = git2::RemoteCallbacks::new();
            if !pat.is_empty() {
                callbacks.credentials(move |_url, user, _| {
                    git2::Cred::userpass_plaintext(user.unwrap_or("microstep"), &pat)
                });
            }
            let rejection = Arc::new(Mutex::new(None::<String>));
            let rejection_slot = Arc::clone(&rejection);
            callbacks.push_update_reference(move |_refname, status| {
                if let Some(status) = status {
                    *rejection_slot.lock().unwrap() = Some(status.to_string());
                    Err(git2::Error::from_str(status))
                } else {
                    Ok(())
                }
            });
            let mut options = git2::PushOptions::new();
            options.remote_callbacks(callbacks);
            if let Err(err) = remote.push(&[refspec.as_str()], Some(&mut options)) {
                return Err(classify_push_error(err));
            }
            if let Some(status) = rejection.lock().unwrap().take() {
                return Err(SyncError::ConcurrentUpdate(status));
            }
            Ok(())
        })
    }

    // -- 对象读写（gix） ---------------------------------------------------

    /// 本地分支 tip（refs/heads/<branch>，无 commit 返回 None）。
    fn branch_tip(&self, branch: &str) -> Option<gix::ObjectId> {
        let repo = gix::open(&self.data_dir).ok()?;
        let mut reference = repo
            .find_reference(format!("refs/heads/{branch}").as_str())
            .ok()?;
        reference.peel_to_id().ok().map(|id| id.detach())
    }

    /// 指定 commit 树内的 events.jsonl blob 内容。
    fn events_content_at(&self, oid: &gix::ObjectId) -> Option<String> {
        let repo = gix::open(&self.data_dir).ok()?;
        read_events_blob(&repo, oid).ok().flatten()
    }

    /// 写快照 commit（blob=events.jsonl 全量；双端分叉时双 parent，D4）。
    fn write_snapshot(
        &self,
        content: &str,
        local_tip: Option<&gix::ObjectId>,
        remote_tip: Option<&gix::ObjectId>,
    ) -> Result<gix::ObjectId, SyncError> {
        let repo = gix::open(&self.data_dir).map_err(|err| SyncError::Git(err.to_string()))?;
        let blob_id = repo
            .write_blob(content.as_bytes())
            .map_err(|err| SyncError::Git(err.to_string()))?
            .detach();
        let tree = gix::objs::Tree {
            entries: vec![gix::objs::tree::Entry {
                mode: gix::objs::tree::EntryKind::Blob.into(),
                filename: EVENTS_FILE.into(),
                oid: blob_id,
            }],
        };
        let tree_id = repo
            .write_object(tree)
            .map_err(|err| SyncError::Git(err.to_string()))?
            .detach();
        let mut parents: Vec<gix::ObjectId> = Vec::new();
        if let Some(tip) = local_tip {
            parents.push(*tip);
        }
        if let Some(tip) = remote_tip {
            if !parents.contains(tip) {
                parents.push(*tip);
            }
        }
        let now = Local::now();
        let signature = gix::actor::Signature {
            name: "MicroStep Sync".into(),
            email: "sync@microstep.local".into(),
            time: gix::date::Time {
                seconds: now.timestamp().max(0),
                offset: now.offset().local_minus_utc(),
            },
        };
        let commit = gix::objs::Commit {
            message: "microstep sync snapshot".into(),
            tree: tree_id,
            parents: parents.into(),
            author: signature.clone(),
            committer: signature,
            encoding: None,
            extra_headers: Default::default(),
        };
        let commit_id = repo
            .write_object(commit)
            .map_err(|err| SyncError::Git(err.to_string()))?
            .detach();
        Ok(commit_id)
    }

    /// 移动本地分支 ref（PreviousValue::Any：创建或更新皆可）。
    fn set_branch_tip(&self, branch: &str, oid: &gix::ObjectId) -> Result<(), SyncError> {
        let repo = gix::open(&self.data_dir).map_err(|err| SyncError::Git(err.to_string()))?;
        repo.reference(
            format!("refs/heads/{branch}"),
            *oid,
            gix::refs::transaction::PreviousValue::Any,
            "microstep sync",
        )
        .map_err(|err| SyncError::Git(err.to_string()))?;
        Ok(())
    }

    // -- 基础设施 ----------------------------------------------------------

    /// 在工作线程执行网络任务，主线程按超时等待（超时即返回，不阻塞调用方）。
    fn run_with_timeout<T, F>(&self, job: F) -> Result<T, SyncError>
    where
        F: FnOnce() -> Result<T, SyncError> + Send + 'static,
        T: Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(job()); // 接收端已超时放弃时静默丢弃
        });
        match rx.recv_timeout(self.timeout) {
            Ok(result) => result,
            Err(_) => Err(SyncError::Timeout(format!(
                "网络操作超过 {}ms 未完成",
                self.timeout.as_millis()
            ))),
        }
    }

    fn record_success(&self, outcome: &SyncOutcome) {
        let mut cfg = load_sync_config(&self.data_dir);
        cfg.last_sync_at = Some(Local::now().to_rfc3339());
        cfg.last_result = Some(format!(
            "成功: pulled={} pushed={} merged={}",
            outcome.pulled, outcome.pushed, outcome.merged
        ));
        let _ = save_sync_config(&self.data_dir, &cfg);
    }

    fn record_failure(&self, err: &SyncError) {
        let mut cfg = load_sync_config(&self.data_dir);
        cfg.last_result = Some(format!("失败: {}", err.message()));
        let _ = save_sync_config(&self.data_dir, &cfg);
    }
}

fn stats_of(outcome: &MergeOutcome, attempts: usize) -> SyncOutcome {
    SyncOutcome {
        pulled: outcome.pulled,
        pushed: outcome.pushed,
        merged: outcome.merged,
        attempts,
    }
}

/// 读 commit 树内 events.jsonl blob（树无该文件 = None，如空仓库种子前）。
fn read_events_blob(
    repo: &gix::Repository,
    commit_oid: &gix::ObjectId,
) -> Result<Option<String>, SyncError> {
    let object = repo
        .find_object(*commit_oid)
        .map_err(|err| SyncError::Git(err.to_string()))?;
    let commit = object
        .try_into_commit()
        .map_err(|err| SyncError::Git(err.to_string()))?;
    let tree = commit
        .tree()
        .map_err(|err| SyncError::Git(err.to_string()))?;
    let Some(entry) = tree
        .lookup_entry_by_path(EVENTS_FILE)
        .map_err(|err| SyncError::Git(err.to_string()))?
    else {
        return Ok(None);
    };
    let blob = repo
        .find_object(entry.id())
        .map_err(|err| SyncError::Git(err.to_string()))?
        .try_into_blob()
        .map_err(|err| SyncError::Git(err.to_string()))?;
    Ok(Some(String::from_utf8_lossy(&blob.data).into_owned()))
}

/// HTTPS 远端注入 PAT（URL userinfo 形态；file/path 远端不受影响）。
fn authenticated_url(url: &str, pat: &str) -> String {
    if pat.is_empty() || !url.starts_with("http") {
        return url.to_string();
    }
    match url.split_once("://") {
        Some((scheme, rest)) => format!("{scheme}://microstep:{pat}@{rest}"),
        None => url.to_string(),
    }
}
