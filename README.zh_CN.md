# Prism3 Clock

[![Rust](https://img.shields.io/badge/rust-1.70+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

为 Rust 提供线程安全的时钟抽象，包含单调时钟和模拟时钟实现。

## 概述

Prism3 Clock 为 Rust 应用程序提供了一套全面的时钟抽象和实现，用于处理各种时间相关的场景。它提供强大的、线程安全的时钟实现，遵循 Rust 惯用法和最佳实践。

## 特性

### 🕐 **时钟实现**
- **SystemClock**: 基于系统时间的标准时钟
- **MonotonicClock**: 单调递增时钟（毫秒精度），不受系统时间调整影响
- **NanoMonotonicClock**: 高精度单调时钟（纳秒精度）
- **MockClock**: 可控制的测试时钟，用于测试场景

### 🔒 **线程安全**
- 所有时钟实现都是线程安全的（`Send + Sync`）
- 系统时钟和单调时钟采用不可变设计
- 模拟时钟采用细粒度锁机制

### ⏱️ **精度**
- 毫秒精度，适用于一般场景
- 纳秒精度，适用于高精度测量
- 单调时间源，不受系统时间调整影响

### 🧪 **可测试性**
- 提供模拟时钟用于受控测试
- 可设置时间到特定时间点
- 可编程方式推进时间
- 支持自动递增

## 安装

在 `Cargo.toml` 中添加：

```toml
[dependencies]
prism3-clock = "0.1.0"
```

## 快速开始

### 基本使用

```rust
use prism3_clock::{Clock, MonotonicClock};

// 创建单调时钟
let clock = MonotonicClock::new();

// 获取当前时间
let now = clock.now();
println!("当前时间: {}", now);

// 获取毫秒时间戳
let millis = clock.millis();
println!("自纪元以来的毫秒数: {}", millis);
```

### 系统时钟

```rust
use prism3_clock::{Clock, SystemClock};

let clock = SystemClock::new();
let now = clock.now();
println!("系统时间: {}", now);
```

### 使用单调时钟测量时间

```rust
use prism3_clock::{Clock, MonotonicClock};
use std::thread;
use std::time::Duration;

let clock = MonotonicClock::new();
let start = clock.millis();

// 执行一些操作
thread::sleep(Duration::from_millis(100));

let elapsed = clock.millis() - start;
println!("耗时: {} 毫秒", elapsed);
```

### 高精度计时

```rust
use prism3_clock::{NanoClock, NanoMonotonicClock};

let clock = NanoMonotonicClock::new();
let start = clock.nanos();

// 执行一些操作

let elapsed = clock.nanos() - start;
println!("耗时: {} 纳秒", elapsed);
```

### 使用模拟时钟进行测试

```rust
use prism3_clock::{Clock, ControllableClock, MockClock};
use chrono::Utc;
use std::time::Duration;

let clock = MockClock::new();

// 设置到特定时间
let target_time = Utc::now();
clock.set_time(target_time);

// 推进时间 1 秒
clock.add_duration(Duration::from_secs(1));

// 添加毫秒
clock.add_millis(500, false);

// 每次调用自动递增
clock.add_millis(10, true);

// 重置到初始状态
clock.reset();
```

### 使用时区

```rust
use prism3_clock::{Clock, MonotonicClock};
use chrono_tz::Asia::Shanghai;

let clock = MonotonicClock::with_timezone(Shanghai);
let now_shanghai = clock.now_in_timezone(Shanghai);
println!("上海时间: {}", now_shanghai);
```

## API 参考

### Clock Trait

核心 `Clock` trait 提供：

- `millis()` - 返回自 Unix 纪元以来的毫秒数
- `now()` - 返回当前时间的 `DateTime<Utc>`
- `timezone()` - 返回时钟的时区
- `now_in_timezone(tz)` - 返回指定时区的当前时间
- `with_timezone(tz)` - 创建使用指定时区的新时钟

### NanoClock Trait

高精度时钟的扩展 trait：

- `nanos()` - 返回自 Unix 纪元以来的纳秒数
- `now_precise()` - 返回高精度的 `DateTime<Utc>`

### ControllableClock Trait

可控制时钟的扩展 trait（用于测试）：

- `set_time(instant)` - 将时钟设置到特定时间
- `add_duration(duration)` - 将时钟推进指定时长
- `reset()` - 将时钟重置到初始状态

## 时钟实现

### SystemClock

- 基于系统时间（`std::time::SystemTime`）
- 受系统时间调整影响
- 简单高效
- 适用于：日志记录、时间戳、系统时间同步

### MonotonicClock

- 基于 `Instant`（单调递增）
- 不受系统时间调整影响
- 毫秒精度
- 适用于：性能监控、超时控制、时间间隔计算

### NanoMonotonicClock

- 基于 `Instant`，纳秒精度
- 不受系统时间调整影响
- 计算开销比 `MonotonicClock` 稍大
- 适用于：高精度测量、微基准测试

### MockClock

- 可编程的测试时钟
- 线程安全，采用细粒度锁
- 支持时间设置、推进和自动递增
- 适用于：单元测试、集成测试、时间相关逻辑测试

## 设计理念

### 单调性

传统系统时钟可能受以下因素影响：
- NTP 同步
- 手动时间调整
- 闰秒

这可能导致：
- 负数时间间隔
- 异常大的时间间隔
- 时间倒流

单调时钟通过以下方式解决这些问题：
1. 使用 `Instant` 作为时间源（单调递增）
2. 创建时记录初始基准点
3. 基于经过的时长计算相对时间
4. 确保时间只会向前移动

### 线程安全

- **不可变时钟**（`SystemClock`、`MonotonicClock`、`NanoMonotonicClock`）：
  - 所有字段都是不可变的
  - 天然线程安全（`Send + Sync`）
  - 无锁开销

- **可变时钟**（`MockClock`）：
  - 使用 `Arc<Mutex<T>>` 管理共享状态
  - 细粒度锁策略
  - 防止并发场景下时间倒流

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
```

## 依赖项

- **chrono**: 日期和时间处理
- **chrono-tz**: 时区支持
- **parking_lot**: 高效的互斥锁实现

## 许可证

Copyright (c) 2025 棱芯科技有限公司。保留所有权利。

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

**胡海星** - *棱芯科技有限公司*

---

有关 Prism3 生态系统的更多信息，请访问我们的 [GitHub 主页](https://github.com/3-prism)。

