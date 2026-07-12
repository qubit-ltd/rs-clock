# `rs-clock` 质量收口实施计划

> **执行约束：** 在当前会话内按 TDD 顺序执行；未经用户明确要求，不执行 `git add`、`git commit` 或 `git push`。

**目标：** 完成 API 防误用、错误枚举演进、契约文档、边界测试和 CI 文档检查的收口。

**架构：** 保持现有四能力模型和单 mutex manual driver，只做局部属性、文档、测试及 CI 调整。

**技术栈：** Rust 2024、Rust 1.94、Tokio 可选 feature、rustdoc/doctest、现有 `.rs-ci`。

## 全局约束

- 不新增依赖。
- 不新增或拆分生产源码文件。
- 测试继续放在 `tests/` 的镜像目录中；公开文档契约使用 doctest。
- 仅修改 `rs-clock` 和其 `.rs-ci` 子模块，不修改下游源码。

---

### Task 1：锁定 subscription 防误用契约

**文件：**

- 修改：`src/monotonic/manual_advance_subscription.rs`

**接口：**

- `ManualAdvanceSubscription` 被丢弃时注销 callback。
- 忽略 subscription 返回值在 `deny(unused_must_use)` 下必须编译失败。

- [x] 先增加精确检查公开类型属性的集成测试。
- [x] 运行目标测试，确认缺少 `#[must_use]` 时 RED。
- [x] 给 `ManualAdvanceSubscription` 增加带原因的 `#[must_use]`。
- [x] 再次运行目标测试，确认 GREEN。

### Task 2：锁定 `TimeError` 的非穷尽契约

**文件：**

- 修改：`src/error/time_error.rs`

**接口：**

- 下游匹配 `TimeError` 必须保留 wildcard 分支。

- [x] 先增加精确检查枚举属性的集成测试，并增加 wildcard 匹配 doctest。
- [x] 运行目标测试，确认缺少 `#[non_exhaustive]` 时 RED。
- [x] 给 `TimeError` 增加 `#[non_exhaustive]`。
- [x] 再次运行目标测试和 doctest，确认 GREEN。

### Task 3：补齐 manual waiter 契约测试并清理重复校验

**文件：**

- 修改：`tests/monotonic/manual_waiter_future_tests.rs`
- 修改：`src/sleep/manual_blocking_sleeper.rs`

**接口：**

- `wait_for_waiters_async(0)` 立即完成。
- observer 创建前目标已满足时立即完成。
- blocking waiter 注册可以唤醒异步 observer。
- domain mismatch 统一由 manual clock 验证。

- [x] 增加三个 waiter observer 边界测试并运行，建立当前行为基线。
- [x] 删除 `ManualBlockingSleeper` 外层重复 domain 校验。
- [x] 运行 monotonic 与 sleep 测试，确认外部错误行为不变。

### Task 4：修正 Tokio 和 callback panic 契约

**文件：**

- 修改：`tests/sleep/tokio_async_sleeper_tests.rs`
- 修改：`src/sleep/tokio_async_sleeper.rs`
- 修改：`src/sleep/manual_async_sleeper.rs`
- 修改：`src/monotonic/manual_monotonic_clock.rs`
- 修改：`src/monotonic/manual_advance_subscription.rs`

**接口：**

- Tokio sleep future 可在 runtime 外创建，但首次 poll 需要启用 time driver 的 runtime。
- `advance()`、`advance_to()` 和 `advance_to_next_deadline()` 明确声明 callback panic。
- manual async waiter 和 waiter observer 明确在方法调用时注册。

- [x] 增加 runtime 外创建 Tokio sleep future 的回归测试。
- [x] 修正 Tokio panic 文档和 manual registration 文档。
- [x] 为三个推进方法补充 `# Panics`。
- [x] 运行 Tokio、monotonic 和 rustdoc 检查。

### Task 5：补齐用户文档和 doctest

**文件：**

- 修改：`src/lib.rs`
- 修改：`src/wall/manual_wall_clock.rs`
- 修改：`README.md`
- 修改：`README.zh_CN.md`

- [x] 增加可运行的 crate 级 manual blocking sleep 示例。
- [x] 修正 subscription drop 措辞和 wall clock 注释重复词。
- [x] 运行 `cargo +1.94.0 test --doc` 和 README 版本检查。

### Task 6：加强 CI 文档门禁

**文件：**

- 修改：`.rs-ci/ci-check.sh`

- [x] 将 rustdoc 命令改为 `--all-features --no-deps`。
- [x] 增加 `-D missing-docs`。
- [x] 运行完整 `./ci-check.sh`。

### Task 7：下游与知识图谱验证

- [x] 运行 `rs-lock` 的 `cargo +1.94.0 test --all-features`。
- [x] 运行 `rs-retry` 的 `cargo +1.94.0 test --all-features`。
- [x] 确认 `rs-clock`、`.rs-ci`、`rs-lock` 和 `rs-retry` 的 Git 状态及变更范围。
- [x] 重建 codebase-memory 索引并确认新符号可检索。
