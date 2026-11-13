# 应用集成测试

本目录集中放跨模块的应用集成测试：真实领域服务、Store、Router、序列化及临时持久化协作。模块单测和 crate 公共契约仍就近放在所属模块与 crate 的 `tests/`。

SM2 已实现 `settings_state.rs`，通过真实设置服务、usher-state 和独立临时仓库运行 11 项集成测试。局部 `support/` 管理仓库生命周期，`fixtures/` 保存旧 / 未来版本文档和测试主题，通用 poll / 唤醒断言复用 usher-test-support。根 Cargo.toml 已声明 `settings_state` 测试目标，macOS CI 已接入同一入口。

在仓库根目录执行：

```sh
python3 scripts/test.py update-state
python3 scripts/test.py integration
python3 scripts/test.py state-ui
python3 scripts/test.py router-ui
```

SM3 的 `state-bridge.test.tsx` 与 `state_bridge.test.tsx` 使用真实 Rust 桥接和隔离的无头引擎，覆盖独立消费者、借用 selector、注册中取消、owner 自动清理、重挂载及引擎销毁。`fixtures/StateBridge.tsx`、`fixtures/state-initial.json` 和 `ui/StateSubscription.tsx` 是本模块专用装配。`state-ui` 同时执行模块就近的 TS 投影单测；递归发现 TS 文件并输出 JUnit / JSON，已接入 CI。NSH 命令在同一工作区内串行运行。

`state_router.rs` 已通过 8 项真实窗口服务 / 路由集成，包含目录注册、历史不变量、可控守卫完成顺序、重定向、关闭、会话 / 操作票据及实际目录 / wire 契约。`router.test.tsx` 使用真实桥接验证 RouterView、NotFound、守卫乱序、预览保留和同 VM 重挂载。专用装配保留在 fixtures/，通用 poll / ready 复用 usher-test-support。新增 Rust 测试入口须在根 `Cargo.toml` 显式配置 `[[test]]` 的名称和路径，并接入 `tests/config.json` 与 CI；不要依赖 Cargo 自动递归发现此目录。

应用专用 mock、夹具与装配留在本目录或所属业务模块；无业务依赖的 Rust 通用工具通过开发依赖使用 `usher-test-support`。跨语言契约数据集中在计划中的 `tests/fixtures/contracts/`。

完整边界及验收见 [测试方案](../../docs/testing.md)。

SM5 的 update_state.rs 覆盖更新服务的后端替换、单执行租约、效果提交、迟到回执、停止 / 恢复屏障及设置字节兼容。更新 mock 数据位于 fixtures/updates.json，使用真实更新 Machine 和两个状态后端；隔离目录由 tempfile 管理，通用任务 / fixture 校验复用 usher-test-support。update-state 命令同时验证原更新 crate；integration 命令包含应用集成部分，两者分别报告，统计时不重复累计。
