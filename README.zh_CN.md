# Qubit Clock

[![CircleCI](https://circleci.com/gh/qubit-ltd/rs-clock.svg?style=shield)](https://circleci.com/gh/qubit-ltd/rs-clock)
[![Coverage Status](https://coveralls.io/repos/github/qubit-ltd/rs-clock/badge.svg?branch=main)](https://coveralls.io/github/qubit-ltd/rs-clock?branch=main)
[![Crates.io](https://img.shields.io/crates/v/qubit-clock.svg?color=blue)](https://crates.io/crates/qubit-clock)
[![Rust](https://img.shields.io/badge/rust-1.70+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

为 Rust 提供线程安全的时钟抽象，包含单调时钟和模拟时钟实现。

## 概述

Qubit Clock 为 Rust 应用程序提供了灵活且类型安全的时钟抽象系统。它提供强大的、线程安全的时钟实现，支持基本时间访问、高精度测量、时区处理、单调时间和测试支持。

## 特性

### 🕐 **时钟抽象**
- **基于 Trait 的设计**：通过正交的 trait 实现灵活的时钟抽象
- **接口隔离**：不强制实现不需要的功能
- **组合优于继承**：通过包装器扩展功能
- **零成本抽象**：只为使用的功能付出代价

### ⏰ **时钟实现**
- **SystemClock**：使用系统墙上时钟时间
- **MonotonicClock**：单调时间（不受系统时间变化影响）
- **NanoMonotonicClock**：纳秒精度的单调时间
- **MockClock**：可控制的测试时钟
- **Zoned\<C\>**：为任何时钟添加时区支持的包装器

### ⏱️ **时间计量器**
- **TimeMeter**：毫秒精度的时间测量，适用于一般场景
- **NanoTimeMeter**：纳秒精度的时间测量，适用于高精度需求
- **人类可读输出**：将耗时格式化为可读字符串
- **速度计算**：计算处理速度（每秒/每分钟处理项数）
- **测试友好**：支持注入模拟时钟以实现确定性测试

### 🔒 **线程安全**
- 所有时钟实现都是 `Send + Sync`
- 系统时钟和单调时钟采用不可变设计
- 模拟时钟采用细粒度锁机制
- 可安全地在线程间共享

### 🌍 **时区支持**
- 将 UTC 时间转换为任何时区
- 为任何时钟包装时区支持
- 基于 `chrono-tz` 提供全面的时区数据库

### 🧪 **测试支持**
- 可控时间的模拟时钟
- 设置时间到特定时间点
- 编程方式推进时间
- 自动递增支持

## 安装

在 `Cargo.toml` 中添加：

```toml
[dependencies]
qubit-clock = "0.1.2"
```

## 快速开始

### 基本使用

```rust
use qubit_clock::{Clock, SystemClock};

let clock = SystemClock::new();
let timestamp = clock.millis();
let time = clock.time();
println!("当前时间: {}", time);
```

### 使用时区

```rust
use qubit_clock::{Clock, ZonedClock, SystemClock, Zoned};
use chrono_tz::Asia::Shanghai;

let clock = Zoned::new(SystemClock::new(), Shanghai);
let local = clock.local_time();
println!("上海本地时间: {}", local);
```

### 使用单调时间进行性能测量

```rust
use qubit_clock::{Clock, MonotonicClock};
use std::thread;
use std::time::Duration;

let clock = MonotonicClock::new();
let start = clock.millis();

thread::sleep(Duration::from_millis(100));

let elapsed = clock.millis() - start;
println!("耗时: {} 毫秒", elapsed);
```

### 使用 MockClock 进行测试

```rust
use qubit_clock::{Clock, ControllableClock, MockClock};
use chrono::{DateTime, Duration, Utc};

let clock = MockClock::new();

// 设置到特定时间
let fixed_time = DateTime::parse_from_rfc3339(
    "2024-01-01T00:00:00Z"
).unwrap().with_timezone(&Utc);
clock.set_time(fixed_time);

assert_eq!(clock.time(), fixed_time);

// 推进时间
clock.add_duration(Duration::hours(1));
assert_eq!(clock.time(), fixed_time + Duration::hours(1));
```

### 高精度测量

```rust
use qubit_clock::{NanoClock, NanoMonotonicClock};

let clock = NanoMonotonicClock::new();
let start = clock.nanos();

// 执行一些操作
for _ in 0..1000 {
    // 一些工作
}

let elapsed = clock.nanos() - start;
println!("耗时: {} 纳秒", elapsed);
```

### 使用时间计量器测量耗时

```rust
use qubit_clock::meter::TimeMeter;
use std::thread;
use std::time::Duration;

let mut meter = TimeMeter::new();
meter.start();
thread::sleep(Duration::from_millis(100));
meter.stop();
println!("耗时: {}", meter.readable_duration());
```

### 高精度时间计量器

```rust
use qubit_clock::meter::NanoTimeMeter;

let mut meter = NanoTimeMeter::new();
meter.start();

// 执行一些操作
for _ in 0..1000 {
    // 一些工作
}

meter.stop();
println!("耗时: {} 纳秒", meter.nanos());
println!("可读格式: {}", meter.readable_duration());
```

### 使用时间计量器计算速度

```rust
use qubit_clock::meter::TimeMeter;
use std::thread;
use std::time::Duration;

let mut meter = TimeMeter::new();
meter.start();

// 处理 1000 个项目
for _ in 0..1000 {
    thread::sleep(Duration::from_micros(100));
}

meter.stop();
println!("处理 1000 个项目耗时 {}", meter.readable_duration());
println!("速度: {}", meter.readable_speed(1000));
```

## 架构

本 crate 围绕几个正交的 trait 构建：

- **Clock**：提供 UTC 时间的基础 trait
- **NanoClock**：纳秒精度的扩展
- **ZonedClock**：时区支持的扩展
- **ControllableClock**：时间控制的扩展（测试用）

这种设计遵循**接口隔离原则**，确保实现只需要提供它们实际支持的功能。

## 时钟实现

### SystemClock

- 基于系统墙上时钟时间
- 受系统时间调整影响（NTP、手动更改）
- 零大小类型（ZST），无运行时开销
- 适用于：日志记录、时间戳、一般时间查询

### MonotonicClock

- 基于 `std::time::Instant`（单调递增）
- 不受系统时间调整影响
- 毫秒精度
- 创建时记录基准点
- 适用于：性能监控、超时控制、时间间隔测量

### NanoMonotonicClock

- 基于 `std::time::Instant`，纳秒精度
- 不受系统时间调整影响
- 比 `MonotonicClock` 精度更高
- 适用于：高精度测量、微基准测试

### MockClock

- 可控制的测试时钟
- 使用 `Arc<Mutex<>>` 实现线程安全
- 支持时间设置、推进和自动递增
- 基于 `MonotonicClock` 保证稳定性
- 适用于：单元测试、集成测试、时间相关逻辑测试

### Zoned\<C\>

- 为任何时钟添加时区支持的包装器
- 泛型支持任何 `Clock` 实现
- 将 UTC 时间转换为指定时区的本地时间
- 适用于：显示本地时间、时区转换

## 时间计量器

### TimeMeter

毫秒精度的时间计量器，用于测量耗时，具有以下特性：

- **灵活的时钟源**：支持任何实现 `Clock` trait 的时钟
- **默认使用 MonotonicClock**：默认使用单调时间以获得稳定的测量结果
- **多种输出格式**：毫秒、秒、分钟和人类可读格式
- **速度计算**：计算处理速度（每秒/每分钟处理项数）
- **实时监控**：无需停止计量器即可获取耗时
- **测试友好**：注入 `MockClock` 实现确定性测试

输出格式示例：
- `123 ms` - 小于 1 秒
- `1.23 s` - 1-60 秒
- `1 m 23.45 s` - 超过 1 分钟

### NanoTimeMeter

纳秒精度的时间计量器，具有与 `TimeMeter` 类似的特性：

- **纳秒精度**：基于 `NanoClock` trait
- **默认使用 NanoMonotonicClock**：使用高精度单调时间
- **人类可读输出**：自动选择合适的单位（ns、μs、ms、s、m）
- **速度计算**：高精度速度计算
- **测试友好**：支持模拟时钟注入

输出格式示例：
- `123 ns` - 小于 1 微秒
- `123.45 μs` - 1-1000 微秒
- `123.45 ms` - 1-1000 毫秒
- `1.23 s` - 1-60 秒
- `1 m 23.45 s` - 超过 1 分钟

## API 参考

### Clock Trait

核心 `Clock` trait 提供：

- `millis()` - 返回自 Unix 纪元以来的毫秒数
- `time()` - 返回当前时间的 `DateTime<Utc>`

### NanoClock Trait

高精度时钟的扩展 trait：

- `nanos()` - 返回自 Unix 纪元以来的纳秒数
- `nano_time()` - 返回高精度的 `DateTime<Utc>`

### ZonedClock Trait

时区支持的扩展 trait：

- `timezone()` - 返回时钟的时区
- `local_time()` - 返回时钟时区的当前时间
- `local_time_in(tz)` - 返回指定时区的当前时间

### ControllableClock Trait

可控制时钟的扩展 trait（用于测试）：

- `set_time(instant)` - 将时钟设置到特定时间
- `add_duration(duration)` - 将时钟推进指定时长
- `reset()` - 将时钟重置到初始状态

## 设计原则

### 接口隔离

本 crate 遵循接口隔离原则，为不同的能力提供独立的 trait：

- 并非所有时钟都需要纳秒精度 → `NanoClock` 是独立的
- 并非所有时钟都需要时区支持 → `ZonedClock` 是独立的
- 只有测试时钟需要可控性 → `ControllableClock` 是独立的

这使得实现只需提供它们需要的功能，保持 API 简洁和专注。

### 单一职责

每个 trait 和类型都有一个明确的目的：

- `Clock` - 提供 UTC 时间
- `NanoClock` - 提供高精度时间
- `ZonedClock` - 提供时区转换
- `ControllableClock` - 提供测试用的时间控制

### 组合优于继承

通过包装器而不是继承来扩展功能：

- `Zoned<C>` 包装任何 `Clock` 以添加时区支持
- 时间计量器通过泛型接受任何 `Clock` 实现

### 零成本抽象

设计确保只为使用的功能付出代价：

- `SystemClock` 和 `MonotonicClock` 是零大小或最小开销
- Trait 方法通常会被内联
- 泛型代码在编译时单态化

## 测试与代码覆盖率

本项目保持全面的测试覆盖，对所有功能进行详细验证。

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行覆盖率报告
./coverage.sh

# 生成文本格式报告
./coverage.sh text

# 运行 CI 检查（测试、lint、格式化）
./ci-check.sh
```

## 依赖项

- **chrono**：日期和时间处理，支持序列化
- **chrono-tz**：全面的时区数据库
- **parking_lot**：高效的互斥锁实现，用于模拟时钟

## 使用场景

### 性能监控

```rust
use qubit_clock::meter::TimeMeter;

let mut meter = TimeMeter::new();
meter.start();

// 执行操作
process_data();

meter.stop();
log::info!("处理耗时: {}", meter.readable_duration());
```

### 超时控制

```rust
use qubit_clock::{Clock, MonotonicClock};
use std::time::Duration;

let clock = MonotonicClock::new();
let deadline = clock.millis() + 5000; // 从现在开始 5 秒

while clock.millis() < deadline {
    if try_operation() {
        break;
    }
}
```

### 测试时间相关逻辑

```rust
use qubit_clock::{Clock, ControllableClock, MockClock};
use chrono::Duration;

#[test]
fn test_expiration() {
    let clock = MockClock::new();
    let item = Item::new(clock.clone());

    // 快进 1 小时
    clock.add_duration(Duration::hours(1));

    assert!(item.is_expired());
}
```

### 基准测试

```rust
use qubit_clock::meter::NanoTimeMeter;

let mut meter = NanoTimeMeter::new();
meter.start();

for _ in 0..10000 {
    expensive_operation();
}

meter.stop();
println!("每次操作平均耗时: {} 纳秒", meter.nanos() / 10000);
```

## 许可证

Copyright (c) 2025 - 2026. Haixing Hu, Qubit Co. Ltd. 保留所有权利。

根据 Apache 许可证 2.0 版（"许可证"）授权；
除非遵守许可证，否则您不得使用此文件。
您可以在以下位置获取许可证副本：

    http://www.apache.org/licenses/LICENSE-2.0

除非适用法律要求或书面同意，否则根据许可证分发的软件
按"原样"分发，不附带任何明示或暗示的担保或条件。
有关许可证下的特定语言管理权限和限制，请参阅许可证。

完整的许可证文本请参阅 [LICENSE](LICENSE)。

## 贡献

欢迎贡献！请随时提交 Pull Request。

## 作者

**胡海星** - *Qubit Co. Ltd.*

---

有关 Qubit 相关开源项目的更多信息，请访问我们的 [GitHub 主页](https://github.com/qubit-ltd)。
