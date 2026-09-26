//! AppData 数据目录初始化（data-storage spec）：目录就绪 + 手工 `.git` 骨架。
//! 不调用子进程 / sidecar（app-shell spec 移动端就绪约束），等价 `git init`
//! 只需写出 git 识别仓库所需的最小结构；失败仅告警、不阻断启动。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// 数据目录内事件流文件路径（唯一持久化文件）。
pub fn events_path(data_dir: &Path) -> PathBuf {
    data_dir.join("events.jsonl")
}

/// 初始化数据目录：创建目录 + git 骨架（幂等；已有 `.git` 不动，已有
/// events.jsonl 不覆盖——文件本体由 EventStore 按「非空即存量」规则接管）。
pub fn init_data_dir(data_dir: &Path) -> PathBuf {
    if let Err(err) = fs::create_dir_all(data_dir) {
        eprintln!("数据目录创建失败（不阻断启动）: {err}");
    }
    if let Err(err) = ensure_git_skeleton(data_dir) {
        eprintln!("数据目录 git 骨架初始化失败（不阻断启动）: {err}");
    }
    events_path(data_dir)
}

/// 同步身份段（reflog committer 用；提交对象签名在 sync.rs 显式构造，不受影响）。
/// 仓库为应用自有骨架，不依赖用户全局 git 身份——无身份环境（CI 全新
/// runner、未安装/未配置 git 的终端用户）下 gix ref 事务写 reflog 会整体
/// 失败（ERR-002/ADR-006）。
const GIT_USER_SECTION: &str = "[user]\n\tname = MicroStep\n\temail = sync@microstep.local\n";
const GIT_SKELETON_CONFIG: &str = "[core]\n\trepositoryformatversion = 0\n\tfilemode = false\n\tbare = false\n\tlogallrefupdates = true\n";

/// 手工 `git init`：HEAD + config + objects/refs 目录树。
/// `.git/HEAD` 已存在视为已是仓库：仅自愈同步身份，不覆盖既有结构。
fn ensure_git_skeleton(data_dir: &Path) -> io::Result<()> {
    let git = data_dir.join(".git");
    // 目录树幂等创建（含存量仓库，自愈旧骨架）：gix/libgit2 写 reflog 时
    // 也会自建父目录，此处显式预置是防御性兜底（消除实现差异）。
    fs::create_dir_all(git.join("objects/info"))?;
    fs::create_dir_all(git.join("objects/pack"))?;
    fs::create_dir_all(git.join("refs/heads"))?;
    fs::create_dir_all(git.join("refs/tags"))?;
    fs::create_dir_all(git.join("refs/remotes"))?;
    fs::create_dir_all(git.join("logs/refs/heads"))?;
    fs::create_dir_all(git.join("logs/refs/remotes"))?;
    if git.join("HEAD").exists() {
        ensure_sync_identity(&git)?;
        return Ok(());
    }
    fs::write(git.join("HEAD"), b"ref: refs/heads/main\n")?;
    fs::write(git.join("config"), format!("{GIT_SKELETON_CONFIG}{GIT_USER_SECTION}"))?;
    Ok(())
}

/// 存量骨架自愈：仅当 config 中完全没有 `[user]` 段时追加同步身份，
/// 既有段落与值原样保留（不覆盖）。gix ref 事务（fetch 更新远端追踪 ref /
/// 移动本地分支）写 reflog 需要 committer；全局 git 身份不存在的环境
/// （CI 全新 runner、未安装/未配置 git 的终端用户）会整体失败。
fn ensure_sync_identity(git: &Path) -> io::Result<()> {
    let config = git.join("config");
    let existing = match fs::read_to_string(&config) {
        Ok(content) => content,
        Err(_) => return Ok(()), // 无 config 的半成品骨架：不伪造，静默跳过
    };
    if existing
        .lines()
        .any(|line| line.trim().eq_ignore_ascii_case("[user]"))
    {
        return Ok(());
    }
    let mut patched = existing;
    if !patched.ends_with('\n') {
        patched.push('\n');
    }
    patched.push_str(GIT_USER_SECTION);
    fs::write(config, patched)
}
