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

/// 手工 `git init`：HEAD + config + objects/refs 目录树。
/// `.git/HEAD` 已存在视为已是仓库，直接跳过（不覆盖既有仓库）。
fn ensure_git_skeleton(data_dir: &Path) -> io::Result<()> {
    let git = data_dir.join(".git");
    // 目录树幂等创建（含存量仓库）：config 开启 logallrefupdates 后，
    // fetch/push 更新 ref 需写 reflog；Linux 上 gix/git2 不会自动逐级
    // 创建 logs/ 父目录（Windows 恰好容忍），必须骨架期备齐（自愈旧骨架）。
    fs::create_dir_all(git.join("objects/info"))?;
    fs::create_dir_all(git.join("objects/pack"))?;
    fs::create_dir_all(git.join("refs/heads"))?;
    fs::create_dir_all(git.join("refs/tags"))?;
    fs::create_dir_all(git.join("refs/remotes"))?;
    fs::create_dir_all(git.join("logs/refs/heads"))?;
    fs::create_dir_all(git.join("logs/refs/remotes"))?;
    if git.join("HEAD").exists() {
        return Ok(());
    }
    fs::write(git.join("HEAD"), b"ref: refs/heads/main\n")?;
    fs::write(
        git.join("config"),
        "[core]\n\trepositoryformatversion = 0\n\tfilemode = false\n\tbare = false\n\tlogallrefupdates = true\n",
    )?;
    Ok(())
}
