# Qubit Clock

[![Rust CI](https://github.com/qubit-ltd/rs-clock/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-clock/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-clock/coverage-badge.json)](https://qubit-ltd.github.io/rs-clock/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-clock.svg?color=blue)](https://crates.io/crates/qubit-clock)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

时间是一项容易被忽略的依赖。代码一旦直接调用 `SystemTime::now()`、
`Instant::now()` 或休眠函数，就会与机器时钟绑定：测试不得不真实等待，时间边界难以
覆盖，时钟跳变还可能让结果变得不确定。

`qubit-clock` 将时间变成可注入的依赖。应用组件只依赖精简的时钟和定时器 trait；
IoC 组装层在生产环境中提供标准实现，在测试中提供固定或可手动推进的实现。同一份
业务代码可以在两种环境中运行，不需要测试专用分支，也不需要真实等待。

详细文档：

- [中文用户手册](doc/user_guide.zh_CN.md)
- [English User Guide](doc/user_guide.en.md)
- [API 文档](https://docs.rs/qubit-clock)

## 第一个例子

下面的会话对象保存单调时钟的截止时间，不直接读取全局时钟。构造函数接收
`Arc<dyn MonotonicClock>`，因此时间如何推进由调用方决定：

```rust
use qubit_clock::{
    ManualMonotonicClock, MonotonicClock, MonotonicInstant, StdMonotonicClock,
    TimeError,
};
use std::{sync::Arc, time::Duration};

struct Session {
    clock: Arc<dyn MonotonicClock>,
    expires_at: MonotonicInstant,
}

impl Session {
    fn new(
        clock: Arc<dyn MonotonicClock>,
        ttl: Duration,
    ) -> Result<Self, TimeError> {
        let expires_at = clock.deadline_after(ttl)?;
        Ok(Self { clock, expires_at })
    }

    fn is_expired(&self) -> bool {
        self.clock.now() >= self.expires_at
    }
}

fn main() -> Result<(), TimeError> {
    // 生产环境组装时使用操作系统提供的单调时钟。
    let _production = Session::new(
        Arc::new(StdMonotonicClock::new()),
        Duration::from_secs(30),
    )?;

    // 测试注入手动时钟，可以立即到达时间边界。
    let clock = ManualMonotonicClock::new_shared();
    let session = Session::new(clock.clone(), Duration::from_secs(30))?;
    assert!(!session.is_expired());

    clock.advance(Duration::from_secs(30))?;
    assert!(session.is_expired());
    Ok(())
}
```

这个测试无需等待 30 秒，就能覆盖恰好过期的边界。变化只发生在组装层；
`Session` 内部没有 mock 开关或测试专用逻辑。

## 组件概览

| 需求 | Trait | 真实时间实现 | 确定性测试实现 |
|---|---|---|---|
| 现实世界时间戳 | `WallClock` | `StdWallClock` | `FixedWallClock`、`ManualWallClock` |
| 耗时与截止时间 | `MonotonicClock` | `StdMonotonicClock`、`TokioMonotonicClock` | `ManualMonotonicClock` |
| 异步截止时间 | `Timer` | `StdTimer`、`TokioTimer` | `ManualTimer` |
| 阻塞等待 | `BlockingSleeper` 适配器 | 组合可独立推进的定时器 | 组合由测试推进的 `ManualTimer` |

墙上时钟时间可能跳变，适合表示对外有意义的时间戳。同一时钟域中的单调时间永不
倒退，适合测量耗时、实现重试和超时。每个单调时钟都可通过
`clock.new_timer()` 直接创建同域定时器。

## 安装

```toml
[dependencies]
qubit-clock = "0.12"
```

需要基于 Tokio 的时钟、定时器及相关运行时错误时，启用 `tokio` feature：

```toml
[dependencies]
qubit-clock = { version = "0.12", features = ["tokio"] }
```

该 feature 提供 `TokioMonotonicClock`、`TokioTimer` 及相关运行时错误。手动时间与
执行器无关，不需要启用它。测试如果需要确定性地模拟定时器故障，可以在开发依赖中
启用默认关闭的 `test-util` feature。

## 定时器与等待

如果组件不仅要检查当前时间，还要等待截止时间，应注入 `Arc<dyn Timer>`。
`Timer::after` 创建相对截止时间，`Timer::at` 接受绝对
`MonotonicInstant`。`ManualMonotonicClock` 可以创建同域的手动定时器，测试只需推进
逻辑时间，不必等待调度器或操作系统。

`BlockingSleeper` 将定时器适配给同步代码。调用线程停驻后，定时器后端必须仍能独立
推进。[用户手册](doc/user_guide.zh_CN.md)详细说明了手动时间协调、Tokio 运行时
所有权、墙上时钟投影、取消和错误处理。

## 在相关库中的应用

`rs-command` 用相同的注入方式实现命令超时，无需真实等待；`rs-id` 则将 ID 时间戳与
分配等待分离。`rs-lock` 用它测试带超时的等待，`rs-retry` 用它测试重试间隔、单次尝试
超时和总耗时预算。这些库按需注入 `WallClock`、`Timer` 或 `MonotonicClock`，生产代码中
不需要另写一套模拟等待算法。

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
