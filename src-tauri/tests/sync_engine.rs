//! SyncEngine 集成测试 · TC-I01~I11 / TC-E01~E07（add-git-remote-sync spec：
//! 同步流程、Bootstrap、同步触发、并发与锁定、错误处理）。
//!
//! 测试远端 = 本地 bare repo（design D9：file/path 传输，零网络零认证；
//! 认证面由 sync_ipc.rs 的错误映射单测 + 手工冒烟覆盖）。

use std::collections::HashSet;
use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use microstep_lib::app_state::AppState;
use microstep_lib::data_dir;
use microstep_lib::store::EventStore;
use microstep_lib::sync::{
    load_sync_config, save_sync_config, SyncConfig, SyncEngine, SyncError, SyncHooks,
    DEFAULT_SYNC_TIMEOUT,
};

fn unique_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("microstep-synceng-{tag}-{nanos}"))
}

/// 最小合法事件行（event_id + 可重放字段）。
fn ev(id: &str) -> String {
    format!("{{\"event_id\":\"{id}\",\"type\":\"TASK_CREATED\",\"id\":\"t-{id}\",\"title\":\"{id}\"}}\n")
}

fn parsed_ids(content: &str) -> HashSet<String> {
    content
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter_map(|v| {
            v.get("event_id")
                .and_then(|x| x.as_str())
                .map(str::to_string)
        })
        .collect()
}

fn ids<const N: usize>(items: [&str; N]) -> HashSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

// ---------------------------------------------------------------------------
// git 夹具（git2 构造本地 bare repo 远端，无子进程）
// ---------------------------------------------------------------------------

fn init_bare(tag: &str) -> PathBuf {
    let path = unique_dir(tag);
    git2::Repository::init_bare(&path).expect("init bare repo");
    path
}

/// 以全量内容覆写远端分支 tip（parent = 当前 tip），返回新 commit id。
fn remote_commit(bare: &Path, branch: &str, content: &str) -> git2::Oid {
    let repo = git2::Repository::open_bare(bare).expect("open bare");
    let blob = repo.blob(content.as_bytes()).expect("blob");
    let mut builder = repo.treebuilder(None).expect("treebuilder");
    builder
        .insert("events.jsonl", blob, 0o100644)
        .expect("tree entry");
    let tree_id = builder.write().expect("tree");
    let tree = repo.find_tree(tree_id).expect("tree object");
    let sig = git2::Signature::now("microstep-test", "test@microstep.local").expect("sig");
    let refname = format!("refs/heads/{branch}");
    let parents: Vec<git2::Commit> = repo
        .find_reference(&refname)
        .ok()
        .and_then(|r| r.peel_to_commit().ok())
        .into_iter()
        .collect();
    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
    repo.commit(Some(&refname), &sig, &sig, "test seed", &tree, &parent_refs)
        .expect("commit")
}

fn remote_events(bare: &Path, branch: &str) -> Option<String> {
    let repo = git2::Repository::open_bare(bare).ok()?;
    let commit = repo
        .find_reference(format!("refs/heads/{branch}").as_str())
        .ok()?
        .peel_to_commit()
        .ok()?;
    let tree = commit.tree().ok()?;
    let entry = tree.get_name("events.jsonl")?;
    let blob = repo.find_blob(entry.id()).ok()?;
    Some(String::from_utf8(blob.content().to_vec()).expect("utf8"))
}

fn remote_tip(bare: &Path, branch: &str) -> Option<git2::Oid> {
    let repo = git2::Repository::open_bare(bare).ok()?;
    let commit = repo
        .find_reference(format!("refs/heads/{branch}").as_str())
        .ok()?
        .peel_to_commit()
        .ok()?;
    Some(commit.id())
}

/// 模拟并发设备推进远端：当前 tip 内容 + 追加一行，生成新 commit 并移动 ref。
fn advance_remote(bare: &Path, branch: &str, extra_line: &str) {
    let mut content = remote_events(bare, branch).unwrap_or_default();
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(extra_line);
    remote_commit(bare, branch, &content);
}

/// 本地数据目录（.git 骨架 + 可选初始事件内容）。
fn device(tag: &str, content: &str) -> (PathBuf, PathBuf) {
    let dir = unique_dir(tag);
    let events = data_dir::init_data_dir(&dir);
    if !content.is_empty() {
        fs::write(&events, content).expect("write events");
    }
    (dir, events)
}

fn configure(data_dir: &Path, remote: &Path, branch: &str) {
    save_sync_config(
        data_dir,
        &SyncConfig {
            remote_url: remote.to_string_lossy().into_owned(),
            pat: String::new(),
            branch: branch.to_string(),
            last_sync_at: None,
            last_result: None,
        },
    )
    .expect("save sync config");
}

fn engine_and_store(data_dir: &Path) -> (SyncEngine, Arc<Mutex<EventStore>>) {
    let events = data_dir::events_path(data_dir);
    (
        SyncEngine::new(data_dir.to_path_buf(), events.clone()),
        Arc::new(Mutex::new(EventStore::open(events))),
    )
}

fn local_commit_count(data_dir: &Path) -> usize {
    let Ok(repo) = git2::Repository::open(data_dir.join(".git")) else {
        return 0;
    };
    let Ok(head) = repo.find_reference("HEAD") else {
        return 0;
    };
    let Ok(commit) = head.peel_to_commit() else {
        return 0; // unborn HEAD（骨架仓库，尚无 commit）
    };
    let mut walk = repo.revwalk().expect("revwalk");
    walk.push(commit.id()).expect("push tip");
    walk.count()
}

fn local_head_parent_count(data_dir: &Path) -> usize {
    let repo = git2::Repository::open(data_dir.join(".git")).expect("open local repo");
    let head = repo.find_reference("HEAD").expect("HEAD");
    let commit = head.peel_to_commit().expect("HEAD commit");
    commit.parent_count()
}

// ---------------------------------------------------------------------------
// 同步流程 · Bootstrap
// ---------------------------------------------------------------------------

/// TC-I01 · 空远端推种子：远端无 commit、本地有事件 → 首个快照 commit 推送成功，
/// 远端 events.jsonl = 本地全量；last_sync_at / last_result 更新。
#[test]
fn tc_i01_bootstrap_empty_remote_push_seed() {
    let bare = init_bare("i01");
    let local_content = format!("{}{}", ev("a1"), ev("a2"));
    let (dir, _events) = device("i01", &local_content);
    configure(&dir, &bare, "main");
    let (engine, store) = engine_and_store(&dir);

    let out = engine.sync_now(&store).expect("seed push");
    assert_eq!((out.pulled, out.pushed, out.merged), (0, 2, 0));
    assert_eq!(out.attempts, 1);

    assert_eq!(
        remote_events(&bare, "main").expect("remote tip"),
        local_content,
        "远端 blob 应为本地全量快照"
    );
    assert_eq!(local_commit_count(&dir), 1, "本地写入首个快照 commit");

    let cfg = load_sync_config(&dir);
    assert!(cfg.last_sync_at.as_deref().is_some_and(|s| !s.is_empty()));
    assert!(cfg.last_result.as_deref().is_some_and(|s| !s.is_empty()));
}

/// TC-I02a · 纯恢复（本地 events.jsonl 缺失）：远端事件按远端序全量落盘，
/// pulled=N；合并后状态可正常重放（TC-I11）。
#[test]
fn tc_i02a_pure_recovery_missing_file() {
    let bare = init_bare("i02a");
    let remote_content = format!("{}{}", ev("r1"), ev("r2"));
    remote_commit(&bare, "main", &remote_content);
    let (dir, events) = device("i02a", "");
    fs::remove_file(&events).ok(); // 确保缺失
    configure(&dir, &bare, "main");
    let (engine, store) = engine_and_store(&dir);

    let out = engine.sync_now(&store).expect("recovery");
    assert_eq!(out.pulled, 2);
    assert_eq!(out.pushed, 0);
    assert_eq!(fs::read_to_string(&events).unwrap(), remote_content);

    // TC-I11 · merge 后可正常重放：get_state 不炸、Task 投影正确。
    let app = AppState::new(EventStore::open(events));
    let state = app.get_state();
    assert!(state["profile"].is_object(), "state 应可重放: {state}");
    assert_eq!(state["tasks"].as_object().map(|t| t.len()), Some(2));
}

/// TC-I02b · 纯恢复（本地 events.jsonl 为空文件）：同缺失路径。
#[test]
fn tc_i02b_pure_recovery_empty_file() {
    let bare = init_bare("i02b");
    let remote_content = format!("{}{}{}", ev("r1"), ev("r2"), ev("r3"));
    remote_commit(&bare, "main", &remote_content);
    let (dir, events) = device("i02b", "");
    fs::write(&events, "").expect("empty file");
    configure(&dir, &bare, "main");
    let (engine, store) = engine_and_store(&dir);

    let out = engine.sync_now(&store).expect("recovery");
    assert_eq!(out.pulled, 3);
    assert_eq!(fs::read_to_string(&events).unwrap(), remote_content);
}

// ---------------------------------------------------------------------------
// 同步流程 · 双向 / 快照 / 合流
// ---------------------------------------------------------------------------

/// TC-I03 · 双向同步：A、B 各有独有事件 → 先后 sync 后双端事件集合 = 并集；
/// 第三轮零动作（稳定收敛，无顺序乒乓）。
#[test]
fn tc_i03_bidirectional_union_and_stability() {
    let bare = init_bare("i03");
    let (dir_a, events_a) = device("i03a", &ev("a1"));
    configure(&dir_a, &bare, "main");
    let (engine_a, store_a) = engine_and_store(&dir_a);
    let out_a1 = engine_a.sync_now(&store_a).expect("A seed");
    assert_eq!((out_a1.pulled, out_a1.pushed), (0, 1));

    // 设备 B：独立数据目录，先有 b1，再同步（拉 a1 并入）。
    let (dir_b, events_b) = device("i03b", &ev("b1"));
    configure(&dir_b, &bare, "main");
    let (engine_b, store_b) = engine_and_store(&dir_b);
    let out_b = engine_b.sync_now(&store_b).expect("B sync");
    assert_eq!(out_b.pulled, 1, "B 拉到 a1");
    assert_eq!(out_b.pushed, 1, "B 推送本地独有 b1");

    // 设备 A 第二轮：拉到 b1，合流并推回。
    let out_a2 = engine_a.sync_now(&store_a).expect("A merge round");
    assert_eq!(out_a2.pulled, 1);
    assert_eq!(out_a2.pushed, 0);

    // 稳定收敛：集合已一致 → 零动作、无新 commit、远端 ref 不动（无乒乓）。
    let tip_before = remote_tip(&bare, "main").expect("tip");
    let count_before = local_commit_count(&dir_a);
    let out_a3 = engine_a.sync_now(&store_a).expect("A stable round");
    // 稳定收敛断言核心是零拉取/零推送（merged 为双端同 id 计数，与 TC-U16 去重语义一致，恒 >0）。
    assert_eq!((out_a3.pulled, out_a3.pushed), (0, 0));
    assert_eq!(remote_tip(&bare, "main"), Some(tip_before));
    assert_eq!(local_commit_count(&dir_a), count_before);

    // 双端一致 = 事件集合一致（D3 本地序保留，跨设备字节序不强制）。
    let content_a = fs::read_to_string(&events_a).unwrap();
    let content_b = fs::read_to_string(&events_b).unwrap();
    let remote = remote_events(&bare, "main").expect("remote");
    let union = ids(["a1", "b1"]);
    assert_eq!(parsed_ids(&content_a), union);
    assert_eq!(parsed_ids(&content_b), union);
    assert_eq!(parsed_ids(&remote), union);
}

/// TC-I04 · 快照 commit 纪律：内容无变化时同步不新增 commit、不动远端 ref。
#[test]
fn tc_i04_no_change_sync_writes_no_commit() {
    let bare = init_bare("i04");
    let (dir, _events) = device("i04", &ev("a1"));
    configure(&dir, &bare, "main");
    let (engine, store) = engine_and_store(&dir);
    engine.sync_now(&store).expect("seed");

    let tip = remote_tip(&bare, "main").expect("tip");
    let count = local_commit_count(&dir);
    let out = engine.sync_now(&store).expect("no-op sync");
    assert_eq!((out.pulled, out.pushed), (0, 0));
    assert_eq!(out.attempts, 1);
    assert_eq!(local_commit_count(&dir), count, "无变化不写 commit");
    assert_eq!(remote_tip(&bare, "main"), Some(tip), "无变化不动远端 ref");
}

/// TC-I05 · 快照式 commit：普通业务追加事件不产生 git commit（仅同步时写）。
#[test]
fn tc_i05_business_append_never_commits() {
    let bare = init_bare("i05");
    let (dir, events) = device("i05", &ev("a1"));
    configure(&dir, &bare, "main");
    let (engine, store) = engine_and_store(&dir);

    // 从未同步过：业务追加 → 0 commit。
    {
        let mut guard = store.lock().unwrap();
        guard.append_event(serde_json::json!({
            "event_id": "biz1", "type": "TASK_CREATED", "id": "biz1", "title": "biz"
        }));
    }
    assert_eq!(local_commit_count(&dir), 0, "业务追加绝不产生 commit");

    // 同步产生 1 个快照 commit 后，再业务追加 → 仍是 1 个。
    engine.sync_now(&store).expect("sync");
    assert_eq!(local_commit_count(&dir), 1);
    {
        let mut guard = store.lock().unwrap();
        guard.append_event(serde_json::json!({
            "event_id": "biz2", "type": "TASK_CREATED", "id": "biz2", "title": "biz"
        }));
    }
    assert_eq!(local_commit_count(&dir), 1, "业务追加绝不产生 commit");
    assert!(fs::read_to_string(&events).unwrap().contains("biz2"));
}

/// TC-I06 · 合流 commit 双 parent：A 已有本地 tip、远端 tip 分叉 → 合并 commit
/// 恰有 2 个 parent（本地 tip + 远端 tip）；B 首次收编远端为单 parent。
#[test]
fn tc_i06_merge_commit_double_parent() {
    let bare = init_bare("i06");
    let (dir_a, _events_a) = device("i06a", &ev("a1"));
    configure(&dir_a, &bare, "main");
    let (engine_a, store_a) = engine_and_store(&dir_a);
    engine_a.sync_now(&store_a).expect("A seed");
    assert_eq!(local_head_parent_count(&dir_a), 0, "种子为根 commit");

    let (dir_b, _events_b) = device("i06b", &ev("b1"));
    configure(&dir_b, &bare, "main");
    let (engine_b, store_b) = engine_and_store(&dir_b);
    engine_b.sync_now(&store_b).expect("B adopt");
    assert_eq!(
        local_head_parent_count(&dir_b),
        1,
        "B 无本地 tip，收编远端历史为单 parent"
    );

    // A 拉到 B 的分叉 tip → 合流 commit 双 parent。
    engine_a.sync_now(&store_a).expect("A merge");
    assert_eq!(
        local_head_parent_count(&dir_a),
        2,
        "合流 commit = 本地 tip + 远端 tip 双 parent"
    );
}

// ---------------------------------------------------------------------------
// 同步流程 · push 并发冲突重试（SyncHooks.before_push 为确定性注入点）
// ---------------------------------------------------------------------------

/// TC-I07a · push 并发冲突重试成功：fetch 后远端被推进一次 → 重跑 fetch+merge，
/// attempts=2，并发新增事件并入，双端完整。
#[test]
fn tc_i07a_push_conflict_retry_success() {
    let bare = init_bare("i07a");
    let (dir, events) = device("i07a", &ev("l1"));
    configure(&dir, &bare, "main");
    let (engine, store) = engine_and_store(&dir);
    engine.sync_now(&store).expect("seed");

    // 本地离线新增 l2（相对上次 commit 有变化 → 本轮会走到 push）。
    fs::write(&events, format!("{}{}", ev("l1"), ev("l2"))).expect("append l2");

    let fired = Arc::new(AtomicUsize::new(0));
    let bare_hook = bare.clone();
    let hooks = SyncHooks::before_push(Box::new(move || {
        // 首轮 push 前模拟另一设备推进远端（fetch 之后）→ non-fast-forward。
        if fired.fetch_add(1, Ordering::SeqCst) == 0 {
            advance_remote(&bare_hook, "main", &ev("u1"));
        }
    }));
    let out = engine
        .sync_now_with_hooks(&store, &hooks)
        .expect("retry sync");
    assert_eq!(out.attempts, 2, "重跑一轮 fetch+merge 后成功");
    assert_eq!(out.pulled, 1, "重试轮并入并发新增 u1");
    assert_eq!(out.pushed, 1, "本地独有 l2 推至远端");

    let content = fs::read_to_string(&events).unwrap();
    assert!(content.contains("\"event_id\":\"l2\""));
    assert!(content.contains("\"event_id\":\"u1\""));
    let remote = remote_events(&bare, "main").expect("remote");
    assert!(remote.contains("\"event_id\":\"l2\""));
    assert!(remote.contains("\"event_id\":\"u1\""));
}

/// TC-I07b · push 并发冲突重试耗尽：远端每轮都被推进 → 重试上限 2 次后报
/// 远端并发更新；本地已合并结果保留、数据无损。
#[test]
fn tc_i07b_push_conflict_retry_exhausted() {
    let bare = init_bare("i07b");
    let (dir, events) = device("i07b", &ev("l1"));
    configure(&dir, &bare, "main");
    let (engine, store) = engine_and_store(&dir);
    engine.sync_now(&store).expect("seed");
    fs::write(&events, format!("{}{}", ev("l1"), ev("l2"))).expect("append l2");

    let round = Arc::new(AtomicUsize::new(0));
    let bare_hook = bare.clone();
    let hooks = SyncHooks::before_push(Box::new(move || {
        let n = round.fetch_add(1, Ordering::SeqCst) + 1;
        advance_remote(&bare_hook, "main", &ev(&format!("u{n}")));
    }));
    let err = engine
        .sync_now_with_hooks(&store, &hooks)
        .expect_err("持续冲突必须报错");
    assert!(
        matches!(err, SyncError::ConcurrentUpdate(_)),
        "实际错误: {err:?}"
    );

    // 本地数据无损：本地事件 + 最后一轮已 fetch 的远端事件均保留。
    let content = fs::read_to_string(&events).unwrap();
    for id in ["l1", "l2", "u1", "u2"] {
        assert!(
            content.contains(&format!("\"event_id\":\"{id}\"")),
            "本地应保留 {id}: {content}"
        );
    }
    // 我方推送始终未落地：远端不含本地独有 l2。
    let remote = remote_events(&bare, "main").expect("remote");
    assert!(!remote.contains("\"event_id\":\"l2\""));
}

/// TC-I08 · 应用管理边界：配置远端并完成同步后 .git/config 无 remote 段，
/// 远端信息仅存在于 sync.json（design D8）。
#[test]
fn tc_i08_no_remote_in_git_config() {
    let bare = init_bare("i08");
    let (dir, _events) = device("i08", &ev("a1"));
    configure(&dir, &bare, "main");
    let (engine, store) = engine_and_store(&dir);
    engine.sync_now(&store).expect("sync");

    let git_config = fs::read_to_string(dir.join(".git").join("config")).unwrap();
    assert!(
        !git_config.contains("[remote"),
        ".git/config 不得写入 remote 配置: {git_config}"
    );
    let raw: Value =
        serde_json::from_str(&fs::read_to_string(dir.join("sync.json")).unwrap()).expect("sync.json");
    assert_eq!(
        raw["remote_url"].as_str(),
        Some(bare.to_string_lossy().as_ref())
    );
}

// ---------------------------------------------------------------------------
// 同步触发 · 启动 best-effort pull
// ---------------------------------------------------------------------------

/// TC-I09a · 启动 pull 未配置时跳过：Ok 零统计（不报错、不产生副作用）。
#[test]
fn tc_i09a_startup_pull_skips_when_unconfigured() {
    let (dir, _events) = device("i09a", &ev("x1"));
    let (engine, store) = engine_and_store(&dir); // 未 configure
    let out = engine.startup_pull(&store).expect("未配置应跳过而非报错");
    assert_eq!((out.pulled, out.pushed, out.merged, out.attempts), (0, 0, 0, 0));
    assert_eq!(local_commit_count(&dir), 0);
}

/// TC-I09b · 启动 pull 远端不可达：不 panic，失败记入 last_result。
#[test]
fn tc_i09b_startup_pull_unreachable_records_last_result() {
    let (dir, _events) = device("i09b", &ev("x1"));
    configure(&dir, &unique_dir("i09b-nope"), "main"); // 永不创建的路径
    let (engine, store) = engine_and_store(&dir);

    let result = engine.startup_pull(&store);
    assert!(result.is_err(), "不可达远端应返回 Err（由调用方吞掉）");
    let cfg = load_sync_config(&dir);
    assert!(
        cfg.last_result.as_deref().is_some_and(|s| s.contains("失败")),
        "失败应记入 last_result: {:?}",
        cfg.last_result
    );
}

/// TC-I09c · 启动 pull 成功：只 pull 落盘不 push（远端 ref 与内容不变）。
#[test]
fn tc_i09c_startup_pull_no_push() {
    let bare = init_bare("i09c");
    let remote_content = format!("{}{}", ev("r1"), ev("r2"));
    remote_commit(&bare, "main", &remote_content);
    let (dir, events) = device("i09c", &ev("l1"));
    configure(&dir, &bare, "main");
    let (engine, store) = engine_and_store(&dir);

    let tip_before = remote_tip(&bare, "main");
    let out = engine.startup_pull(&store).expect("startup pull");
    assert_eq!(out.pulled, 2);
    assert_eq!(out.pushed, 0, "启动 pull 不 push（pushed 恒 0）");

    let content = fs::read_to_string(&events).unwrap();
    assert_eq!(parsed_ids(&content), ids(["l1", "r1", "r2"]));
    assert_eq!(remote_tip(&bare, "main"), tip_before, "远端 ref 不动");
    assert_eq!(
        remote_events(&bare, "main"),
        Some(remote_content),
        "远端内容不被启动 pull 改写"
    );
}

// ---------------------------------------------------------------------------
// 并发与锁定（TC-I10）
// ---------------------------------------------------------------------------

/// TC-I10 · 同步全程持 EventStore 锁：同步进行中（push 前夜）try_lock 必失败，
/// 业务写入阻塞等待，同步完成后正常执行、数据无损。
#[test]
fn tc_i10_sync_holds_store_lock_write_blocks_then_completes() {
    let bare = init_bare("i10"); // 空远端：本地有事件 → 本轮必走到 push
    let (dir, events) = device("i10", &ev("a1"));
    configure(&dir, &bare, "main");
    let engine = Arc::new(SyncEngine::new(dir.clone(), events.clone()));
    let store = Arc::new(Mutex::new(EventStore::open(events.clone())));

    let (tx_hold, rx_hold) = mpsc::channel::<()>();
    let (tx_release, rx_release) = mpsc::channel::<()>();
    let hooks = SyncHooks::before_push(Box::new(move || {
        tx_hold.send(()).expect("signal hold");
        rx_release
            .recv_timeout(Duration::from_secs(10))
            .expect("wait release");
    }));

    let sync_engine = Arc::clone(&engine);
    let sync_store = Arc::clone(&store);
    let sync_handle = thread::spawn(move || sync_engine.sync_now_with_hooks(&sync_store, &hooks));

    rx_hold
        .recv_timeout(Duration::from_secs(10))
        .expect("sync 应进入 push 阶段");
    assert!(store.try_lock().is_err(), "同步网络阶段必须持有 EventStore 锁");

    // 写入线程：应阻塞等待锁，而非报错。
    let writer_store = Arc::clone(&store);
    let (tx_done, rx_done) = mpsc::channel::<()>();
    thread::spawn(move || {
        let mut guard = writer_store.lock().expect("写入应等待而非报错");
        guard.append_event(serde_json::json!({
            "event_id": "w1", "type": "TASK_CREATED", "id": "w1", "title": "w"
        }));
        tx_done.send(()).expect("signal done");
    });
    assert!(
        rx_done.recv_timeout(Duration::from_millis(200)).is_err(),
        "同步持锁期间写入必须阻塞"
    );

    tx_release.send(()).expect("release sync");
    rx_done.recv_timeout(Duration::from_secs(10)).expect("写入应完成");
    let out = sync_handle.join().expect("sync thread").expect("sync ok");
    assert_eq!(out.pushed, 1);
    assert!(fs::read_to_string(&events).unwrap().contains("\"event_id\":\"w1\""));
}

// ---------------------------------------------------------------------------
// 错误处理与边界（TC-E01~E07）
// ---------------------------------------------------------------------------

/// TC-E01 · 远端不可达 → 网络失败分类；本地功能无损。
#[test]
fn tc_e01_unreachable_remote_network_error() {
    let (dir, events) = device("e01", &ev("a1"));
    configure(&dir, &unique_dir("e01-nope"), "main");
    let (engine, store) = engine_and_store(&dir);

    let err = engine.sync_now(&store).expect_err("不可达必须报错");
    assert!(matches!(err, SyncError::Network(_)), "实际错误: {err:?}");
    assert_eq!(
        fs::read_to_string(&events).unwrap(),
        ev("a1"),
        "失败同步不得改动本地事件流"
    );
}

/// TC-E02 · 网络超时：本地 TCP 黑洞（accept 后永不响应）+ 300ms 注入超时 →
/// 超时分类；默认超时常量 = 30s。
#[test]
fn tc_e02_timeout_blackhole() {
    assert_eq!(DEFAULT_SYNC_TIMEOUT, Duration::from_secs(30));

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(sock) = stream {
                thread::spawn(move || {
                    let _sock = sock; // 接受连接但永不响应
                    thread::park();
                });
            }
        }
    });

    let (dir, _events) = device("e02", &ev("a1"));
    save_sync_config(
        &dir,
        &SyncConfig {
            remote_url: format!("http://127.0.0.1:{port}/repo.git"),
            pat: String::new(),
            branch: "main".into(),
            last_sync_at: None,
            last_result: None,
        },
    )
    .unwrap();
    let (engine, store) = engine_and_store(&dir);
    let engine = engine.with_timeout(Duration::from_millis(300));

    let start = std::time::Instant::now();
    let err = engine.sync_now(&store).expect_err("黑洞远端必须超时");
    assert!(matches!(err, SyncError::Timeout(_)), "实际错误: {err:?}");
    assert!(start.elapsed() < Duration::from_secs(5), "超时应及时返回");
}

/// TC-E04 · 同 id 内容冲突：整次同步拒绝，双端字节级原样（本地文件 + 远端 ref）。
#[test]
fn tc_e04_conflict_leaves_both_sides_untouched() {
    let bare = init_bare("e04");
    let remote_line = "{\"event_id\":\"c1\",\"type\":\"TASK_CREATED\",\"id\":\"t1\",\"title\":\"remote-title\"}\n";
    remote_commit(&bare, "main", remote_line);
    let local_line = "{\"event_id\":\"c1\",\"type\":\"TASK_CREATED\",\"id\":\"t1\",\"title\":\"local-title\"}\n";
    let (dir, events) = device("e04", local_line);
    configure(&dir, &bare, "main");
    let (engine, store) = engine_and_store(&dir);

    let before = fs::read_to_string(&events).unwrap();
    let tip_before = remote_tip(&bare, "main");
    let err = engine.sync_now(&store).expect_err("同 id 冲突必须拒绝");
    assert_eq!(err, SyncError::Conflict { event_id: "c1".into() });
    assert_eq!(fs::read_to_string(&events).unwrap(), before, "本地字节不变");
    assert_eq!(remote_tip(&bare, "main"), tip_before, "远端 ref 不动");
}

/// TC-E06 · 双空 bootstrap（假设 A2）：本地无事件 + 远端空仓库 → 成功、零统计。
#[test]
fn tc_e06_double_empty_bootstrap() {
    let bare = init_bare("e06");
    let (dir, _events) = device("e06", "");
    configure(&dir, &bare, "main");
    let (engine, store) = engine_and_store(&dir);

    let out = engine.sync_now(&store).expect("双空 bootstrap 应成功");
    assert_eq!((out.pulled, out.pushed, out.merged), (0, 0, 0));
}

/// TC-E07 · 自定义分支：branch=dev → fetch/push 全程走 dev，main 不受影响。
#[test]
fn tc_e07_custom_branch_dev() {
    let bare = init_bare("e07");
    remote_commit(&bare, "dev", &ev("r1"));
    let (dir, events) = device("e07", &ev("l1"));
    configure(&dir, &bare, "dev");
    let (engine, store) = engine_and_store(&dir);

    let out = engine.sync_now(&store).expect("dev 分支同步");
    assert_eq!(out.pulled, 1);
    assert_eq!(out.pushed, 1);

    let remote = remote_events(&bare, "dev").expect("dev tip");
    assert_eq!(parsed_ids(&remote), ids(["l1", "r1"]));
    assert_eq!(
        parsed_ids(&fs::read_to_string(&events).unwrap()),
        ids(["l1", "r1"])
    );
    let repo = git2::Repository::open_bare(&bare).unwrap();
    assert!(
        repo.find_reference("refs/heads/main").is_err(),
        "main 不应被创建"
    );
}
