//! MicroStep 2.0 Rust 领域核心：纯函数 Reducer + 事件流存储。
//!
//! 移植自 Python `backend/domain.py` / `backend/store.py`，行为等价性由
//! `tests/golden.rs` 的 golden replay 对照守卫（真实流 + 14 事件构造序列零差异）。
//! 生产真实时钟已落位（`EventStore::open` → `real_clock`，chrono 本地时区）；
//! Tauri 壳接线（commands/app_state/data_dir/ticker）随批次二落位。

pub mod domain;
pub mod store;
