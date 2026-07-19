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
qubit-clock = "0.10"
```

需要 Tokio clock 和 timer 类型时启用对应 feature：

```toml
[dependencies]
qubit-clock = { version = "0.10", features = ["tokio"] }
```

该 feature 仅公开 `TokioMonotonicClock` 和 `TokioTimer`。Manual timer 与 manual
协调 future 不绑定 executor，也不需要启用该 feature。下文异步示例只选择 Tokio
来运行和派生任务；复制到测试中时，需要直接声明 Tokio：

```toml
[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt"] }
```

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

这里的“确定性”是指逻辑时间、deadline 选择和 deadline 完成都由测试显式控制；
相同 deadline 的 waiter 唤醒顺序，以及 executor poll 就绪任务的顺序均不作保证。

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

    let reached = clock.advance_to_next_deadline_async().await;
    assert_eq!(Duration::from_secs(5), reached.elapsed_since_origin());
    task.await??;
    Ok(())
}
```

`advance_to_next_deadline_async()` 会等待有效的未来 deadline，再原子推进到该时刻
仍然存在的最早 deadline。取消竞争会触发重新等待，取消 driver future 不会移动
manual time。[用户手册](doc/user_guide.zh_CN.md#manual-time-coordination)详细说明了
快照、count barrier、多阶段协调、runtime affinity、wall reanchor、trait object
注入和错误处理。

同步 driver 线程可使用 `advance_to_next_deadline_after_waiters()`：它等待当前 waiter
数量条件，并在同一个时钟状态锁内完成推进，从而消除观察与推进之间的取消窗口。

## 测试

```bash
# 使用默认 feature 集运行测试
cargo test

# 使用项目声明的全部 feature 运行测试
cargo test --all-features

# 运行项目 CI 检查
./ci-check.sh

# 检查代码覆盖率
./coverage.sh
```

## 许可证

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

本项目基于 Apache License 2.0 授权。完整许可证文本请参阅
[LICENSE](LICENSE)。

## 贡献

欢迎贡献。请遵循 Rust API 指南，及时更新公共 API 文档与测试，并在提交
Pull Request 前运行 `./align-ci.sh`格式化代码，运行`./ci-check.sh`对齐CI要求。

## 作者

**Haixing Hu** - *Qubit Co. Ltd.*

仓库地址：[https://github.com/qubit-ltd/rs-clock](https://github.com/qubit-ltd/rs-clock)
