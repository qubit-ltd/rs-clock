# Qubit Clock

[![Rust CI](https://github.com/qubit-ltd/rs-clock/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-clock/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-clock/coverage-badge.json)](https://qubit-ltd.github.io/rs-clock/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-clock.svg?color=blue)](https://crates.io/crates/qubit-clock)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

为 Rust 提供可注入的 wall clock、monotonic clock 和可确定性测试的 timer。

详细文档：

- [中文用户手册](doc/user_guide.zh_CN.md)
- [English User Guide](doc/user_guide.en.md)
- [API 文档](https://docs.rs/qubit-clock)

## 按能力选择

| 需求 | Trait | 真实时间实现 | 确定性测试实现 |
|---|---|---|---|
| 民用时间戳 | `WallClock` | `StdWallClock` | `FixedWallClock`、`ManualWallClock` |
| 耗时与 deadline | `MonotonicClock` | `StdMonotonicClock`、`TokioMonotonicClock` | `ManualMonotonicClock` |
| 异步 deadline | `Timer` | `StdTimer`、`TokioTimer` | `ManualTimer` |
| 阻塞等待 | `BlockingSleeper` 适配器 | 组合真实 timer | 组合 manual timer |

Wall time 可能跳变，适合表示对外有意义的时间戳。一个 clock domain 内的 monotonic
time 永不倒退，适合测量耗时、实现 retry 和 timeout。每个 clock 都可通过
`clock.new_timer()` 直接创建同域 timer。

## 安装

```toml
[dependencies]
qubit-clock = "0.9"
```

需要 Tokio clock 和 timer 类型时启用对应 feature：

```toml
[dependencies]
qubit-clock = { version = "0.9", features = ["tokio"] }
```

Manual timer 和 manual 协调与 runtime 无关。

## 使用真实时间

```rust
use qubit_clock::{BlockingSleeper, MonotonicClock, StdMonotonicClock, StdWallClock, WallClock};
use std::time::Duration;

let wall_clock = StdWallClock::new();
let clock = StdMonotonicClock::new();
let sleeper = BlockingSleeper::new(clock.new_timer());

let started_at = wall_clock.now();
sleeper
    .sleep_for(Duration::from_millis(10))
    .expect("阻塞 sleep 应正常完成");
println!("started at {started_at:?}");
```

## 确定性 Manual Time

保留一个共享 manual clock 作为测试控制面，并从它派生所有消费能力：

```rust
use qubit_clock::{ManualMonotonicClock, MonotonicClock, Timer};
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
let clock = ManualMonotonicClock::new_shared();
let timer = clock.new_timer();
let task = tokio::spawn(async move {
    timer.after(Duration::from_secs(5))?.await;
    Ok::<_, qubit_clock::TimeError>(())
});

let observed = clock.wait_for_next_deadline_async().await;
assert_eq!(Duration::from_secs(5), observed.elapsed_since_origin());

clock
    .advance_to_next_deadline()
    .expect("有效 timer 应存在未来 deadline");
task.await??;
Ok(())
}
```

`wait_for_next_deadline_async()` 是状态观察器。每次 poll 都返回当前最早、严格位于
未来且仍然有效的 deadline；已取消和已到期 waiter 会被忽略。返回值只是快照，所以
并发驱动应使用 `advance_to_next_deadline()` 原子完成选择和推进。
[用户手册](doc/user_guide.zh_CN.md#11-wait_for_next_deadline_async-的精确语义)
详细说明了完整协调契约、count 启动屏障、多阶段示例、Tokio runtime affinity、
wall reanchor、trait object 注入和错误处理。

## 许可证

使用 Apache License 2.0 许可。
