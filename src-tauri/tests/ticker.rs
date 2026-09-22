//! Ticker 后台任务测试（TC-I29/I30）：启动补结算 + 可变时钟跨日自动注入
//! （background-ticker spec「启动补结算」「运行中跨零点」场景）。
//! 时钟注入用 Arc<Mutex<String>> 可变日期；周期用毫秒级 tokio sleep。

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::json;

use microstep::app_state::AppState;
use microstep::store::{Clock, ClockNow, EventStore};
use microstep::ticker;

fn fixed_clock(date: &str) -> Clock {
    let date = date.to_string();
    Arc::new(move || ClockNow {
        date: date.clone(),
        datetime: format!("{date}T12:00:00+08:00"),
    })
}

fn unique_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("microstep-ticker-{tag}-{nanos}"))
}

/// TC-I29 · 启动补结算：不预调 ensure_bootstrapped，ticker 首轮立即补齐当日 Tick
/// （对齐 Python main() 启动路径：先 ensure_daily_ticks 再起 Ticker 线程）。
#[tokio::test]
async fn tc_i29_ticker_backfills_on_start() {
    let state = Arc::new(AppState::new(EventStore::with_clock(
        unique_dir("i29").join("events.jsonl"),
        fixed_clock("2026-03-01"),
    )));
    tokio::spawn(ticker::run(Arc::clone(&state), Duration::from_millis(50)));
    tokio::time::sleep(Duration::from_millis(300)).await;
    let payload = state.get_state();
    assert_eq!(payload["meta"]["last_tick_day"], json!("2026-03-01"));
}

/// TC-I30 · 运行中跨零点：可变时钟翻日后自动注入新一天 Tick，
/// 记录前一日 SAN 历史并重置为 100。
#[tokio::test]
async fn tc_i30_ticker_crosses_day_boundary() {
    let date = Arc::new(Mutex::new("2026-03-01".to_string()));
    let clock: Clock = {
        let date = Arc::clone(&date);
        Arc::new(move || ClockNow {
            date: date.lock().unwrap().clone(),
            datetime: "2026-03-01T12:00:00+08:00".to_string(),
        })
    };
    let state = Arc::new(AppState::new(EventStore::with_clock(
        unique_dir("i30").join("events.jsonl"),
        clock,
    )));
    tokio::spawn(ticker::run(Arc::clone(&state), Duration::from_millis(30)));

    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(state.get_state()["meta"]["last_tick_day"], json!("2026-03-01"));

    *date.lock().unwrap() = "2026-03-02".to_string();
    tokio::time::sleep(Duration::from_millis(300)).await;
    let payload = state.get_state();
    assert_eq!(payload["meta"]["last_tick_day"], json!("2026-03-02"));
    assert!(payload["meta"]["daily_san_history"].as_object().unwrap().contains_key("2026-03-01"));
    assert_eq!(payload["dimensions"]["san"].as_f64().unwrap(), 100.0);
}
