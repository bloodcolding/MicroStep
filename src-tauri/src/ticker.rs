//! 后台每日结算（design D5 / background-ticker spec）：tokio 周期任务，
//! 语义等价 Python `Ticker` 线程——先结算后等待、首轮立即执行（启动补结算）、
//! 单轮异常打印并继续，绝不中断循环。

use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Duration;

use crate::app_state::AppState;

/// Python `Ticker._run`：循环 { ensure_daily_ticks(); wait(interval) }。
/// 周期默认 600s（Python 默认值），测试注入毫秒级周期 + 可变时钟。
pub async fn run(state: Arc<AppState>, interval: Duration) {
    loop {
        // Python：except Exception 打印后继续下一轮；Rust 以 catch_unwind 承接。
        if let Err(panic) = std::panic::catch_unwind(AssertUnwindSafe(|| {
            state.ensure_daily_ticks();
        })) {
            let message = panic
                .downcast_ref::<&str>()
                .map(|text| text.to_string())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "未知 panic".to_string());
            println!("Daily tick error: {message}");
        }
        tokio::time::sleep(interval).await;
    }
}
