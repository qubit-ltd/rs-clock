# Qubit Clock

[![Rust CI](https://github.com/qubit-ltd/rs-clock/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-clock/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-clock/coverage-badge.json)](https://qubit-ltd.github.io/rs-clock/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-clock.svg?color=blue)](https://crates.io/crates/qubit-clock)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

为 Rust 提供可注入的 wall clock、monotonic clock，以及可确定性测试的同步和异步 sleeper。

## 设计

Qubit Clock 将时间拆成四种能力：

- `WallClock`：以 `SystemTime` 读取现实世界时间。
- `MonotonicClock`：读取属于特定 clock domain 的 `MonotonicInstant`。
- `BlockingSleeper`：按 monotonic deadline 阻塞当前线程。
- `AsyncSleeper`：返回按 monotonic deadline 完成的 future。

Wall time 可能跳变，monotonic time 永不倒退。每个 sleeper 都显式基于对应的 concrete monotonic clock，不维护第二套时间状态。

## 实现

| 能力 | 真实时间实现 | 确定性测试实现 |
|---|---|---|
| Wall time | `StdWallClock` | `FixedWallClock`、`ManualWallClock` |
| Monotonic time | `StdMonotonicClock`、`TokioMonotonicClock` | `ManualMonotonicClock` |
| 同步 sleep | `StdBlockingSleeper` | `ManualBlockingSleeper` |
| 异步 sleep | `TokioAsyncSleeper` | `ManualAsyncSleeper` |

Tokio 类型需要启用可选的 `tokio` feature。Manual async sleeper 不依赖 Tokio runtime。

## 安装

```toml
[dependencies]
qubit-clock = "0.9"
```

需要 Tokio 集成时：

```toml
[dependencies]
qubit-clock = { version = "0.9", features = ["tokio"] }
```

## Wall Time

```rust
use qubit_clock::{StdWallClock, WallClock};

let clock = StdWallClock::new();
let now = clock.now();
println!("当前 wall time：{now:?}");
```

## Monotonic Time

```rust
use qubit_clock::{MonotonicClock, StdMonotonicClock};

let clock = StdMonotonicClock::new();
let start = clock.now();
let elapsed = clock
    .now()
    .duration_since(start)
    .expect("两个 instant 来自同一个 clock");
println!("耗时：{elapsed:?}");
```

## 确定性同步 Sleep

通过 `Arc` 显式表达多个组件共享同一个 clock：

```rust
use qubit_clock::{
    BlockingSleeper, ManualBlockingSleeper, ManualMonotonicClock,
};
use std::sync::Arc;
use std::time::Duration;

let clock = Arc::new(ManualMonotonicClock::new());
let sleeper = Arc::new(ManualBlockingSleeper::from_clock(
    Arc::clone(&clock),
));
let worker_sleeper = Arc::clone(&sleeper);

let worker = std::thread::spawn(move || {
    worker_sleeper
        .sleep_for(Duration::from_secs(10))
        .expect("manual sleep 应正常完成");
});

assert!(sleeper.wait_for_waiters(1, Duration::from_secs(1)));
clock
    .advance(Duration::from_secs(10))
    .expect("manual time 应推进成功");
worker.join().expect("工作线程不应 panic");
```

## Manual Wall Time

`ManualWallClock` 将一个 manual monotonic timeline 映射到 wall-time anchor：

```rust
use qubit_clock::{
    ManualMonotonicClock, ManualWallClock, WallClock,
};
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};

let monotonic_clock = Arc::new(ManualMonotonicClock::new());
let wall_clock = ManualWallClock::from_clock(
    UNIX_EPOCH,
    Arc::clone(&monotonic_clock),
);

monotonic_clock
    .advance(Duration::from_secs(600))
    .expect("manual time 应推进成功");
assert_eq!(UNIX_EPOCH + Duration::from_secs(600), wall_clock.now());

wall_clock.reanchor(UNIX_EPOCH);
assert_eq!(UNIX_EPOCH, wall_clock.now());
```

重新设置 wall anchor 不会改变 monotonic deadline。
如果 wall anchor 加上手动推进的时长超出平台可表示的 `SystemTime` 范围，
`ManualWallClock::now()` 会 panic。

## 统一 Manual Time Driver

同一个 `ManualMonotonicClock` 可以同时驱动 blocking 和 async sleeper。
`pending_waiters()` 汇总两类 waiter，`next_deadline()` 查看最早的未来
deadline，`advance_to_next_deadline()` 原子地推进到该 deadline。异步测试可用
`ManualMonotonicClock::wait_for_waiters_async(&clock, expected_count)` 等待注册，
无需轮询，也不绑定特定 runtime。

## Manual Advance 通知

需要让自身 notification 与 manual deadline 竞争的同步测试替身，可以订阅 clock advance：

```rust
use qubit_clock::ManualMonotonicClock;
use std::sync::Arc;

let clock = Arc::new(ManualMonotonicClock::new());
let subscription = ManualMonotonicClock::subscribe_advances(
    &clock,
    || {
        // 唤醒测试替身自己的 Condvar、watch channel 或 task waker。
    },
);
```

callback 在 clock mutex 外同步执行。它应可重复调用，并且只负责唤醒另一个等待原语。并发 advance 可能无序、并发地执行 callback。如果 callback panic，本次 advance 已捕获的所有 callback 仍会执行，随后在推进线程恢复第一个 panic。丢弃 `subscription` 会阻止后续 advance 注册 callback，但已被某次进行中的 advance 捕获的 callback 仍可能执行一次。

## 设计文档

- [重构设计](doc/clock_refactoring_design.zh_CN.md)
- [实施计划](doc/clock_refactoring_implementation_plan.zh_CN.md)
- [下游接入计划](doc/downstream_integration_implementation_plan.zh_CN.md)

## 许可证

使用 Apache License 2.0 许可。
