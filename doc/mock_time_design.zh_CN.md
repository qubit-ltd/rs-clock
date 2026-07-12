# Manual Time 设计说明

旧的 `MockTimeline`、`MockClock` 和 `MockSleeper` 已被以下职责明确的类型替代：

- `ManualMonotonicClock`：保存并显式推进 monotonic time。
- `ManualWallClock`：将 manual monotonic time 映射到 wall-time anchor。
- `ManualBlockingSleeper`：同步等待 manual monotonic deadline。
- `ManualAsyncSleeper`：异步等待 manual monotonic deadline。

这些类型通过同一个 `Arc<ManualMonotonicClock>` 建立共享关系。`MonotonicInstant` 只是固定时间点，不参与构造 clock 或 sleeper。

```rust
let monotonic_clock = Arc::new(ManualMonotonicClock::new());
let wall_clock = ManualWallClock::from_clock(
    UNIX_EPOCH,
    Arc::clone(&monotonic_clock),
);
let blocking_sleeper = ManualBlockingSleeper::from_clock(
    Arc::clone(&monotonic_clock),
);
let async_sleeper = ManualAsyncSleeper::from_clock(
    Arc::clone(&monotonic_clock),
);
```

完整语义与并发约束见 [重构设计方案](clock_refactoring_design.zh_CN.md)。
