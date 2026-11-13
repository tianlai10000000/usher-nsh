# 应用端到端测试

本目录集中真实应用用户旅程，保留页面、Rust 桥接、领域服务、Store 和订阅链路。仅替换测试环境，不使用业务 module mock 返回目标状态。

`navigation.test.tsx` 有三项无头旅程，包含启动 / 后续深链接校验，以及：实际 App 菜单 / 历史 / 快捷键往返后保留计数、设置分类和导出错误反馈；独立引擎之间隔离历史、计数与分类。`fixtures/session.json` 保存确定性启动配置。每例使用无数据目录的内存会话并在 finally 销毁引擎，不修改真实用户设置。

在仓库根目录执行，和其他 NSH 命令串行运行：

```sh
python3 scripts/test.py e2e --target headless
python3 scripts/test.py e2e --target macos
```

命令已接入 CI，检查非零用例、必测组和新生成的 JUnit，报告写入 target/test-reports/。`platform: "macos"` 是无头平台属性，不是原生窗口。macos target 使用 native/ 中的专用装配与 scripts/native_acceptance.py，显式临时 --data-dir，打开同一 VM 的两扇 AppKit 窗口，并在独立源码副本触发真实 HMR。三项生命周期探针验证偏好共享、会话隔离、旧 owner / 守卫 / 订阅失效、内容卸载后关闭及重开身份。完整进程重启恢复与原生输入全产品旅程仍待办；两个 open 引擎的隔离不替代该原生测试。

模块 mock 测试在所属 __tests__/，真实桥接测试页面在 tests/integration/，原有应用回归在 tests/ui/。不为尚无复用的装配建立空 support 模块。无头、原生宿主和发布产物分别报告。

原生执行需解锁的 macOS 图形会话；失败非零退出，不以能力跳过报告成功。逐轮日志、JUnit 及隔离目录审计保留在 target/test-reports，源码副本和测试数据在退出时删除。CI 手动 native_state 作业要求预先配置带 usher-native 标签的 self-hosted macOS runner，远端尚未运行。

分项旅程和结果见 [测试方案第 14 节](../../docs/todo.md#evidence-testing-2-14-sm4-原生窗口与-hmr-验收)。

RT4 adds `native-back.test.tsx`: the same production BackHandler adapter is exercised headlessly and through `nsh test --target macos`. The official native runner isolates each app's data/cache directory. Catalog HMR now changes identity in the isolated Probe source and checks retained history/session plus invalidated old owners. Deep-link delivery uses the official headless `urlScheme` / `openWithUrl` APIs; this does not verify packaged OS scheme registration.

`python3 scripts/test.py packaged-routing` builds a real release .app from an isolated source copy. The fixture only observes the product router; LaunchServices delivers the URLs through the actual release runner. A unique app ID owns symlinks into temporary data/cache roots, without changing HOME. Cleanup unregisters and stops only that app; a process that cannot stop retains its isolation and fails acceptance. Reports are in `target/test-reports/packaged-routing-*/`.

`python3 scripts/test.py native-reload` runs `reload.test.tsx` through the macOS host's real Cmd+R path. The fixture restores a persisted preference into a new VM, resets transient window state, and rejects the old scope/ticket. Each open uses the official runner's isolated data/cache roots. This is separate from HMR and whole-process restart. The suite is part of manual native CI; headless discovery explicitly includes navigation and native-back only.

Local iOS acceptance is pending Xcode first-launch components (CoreSimulator version mismatch); no existing simulator is used or modified. Other platform hosts and actual upgrade journeys remain unverified.


`buttons-native.test.tsx` reuses all seven journeys from `tests/ui/support/button-journeys.ts`. Run it through `python3 scripts/native_button_acceptance.py --junit target/test-reports/native-buttons.xml`, or the configured macOS E2E suite. The script stages sources and a test-only Rust host wrapper; the official runner isolates data/cache per app. The wrapper extends the production registry with an AppKit panel observer/cancel action and retains the production components. It only cancels visible NSOpenPanel sheets attached to this host's windows. The test verifies two real cancellations, sheet closure, no image/error mutation, idle picker state, removal of the loading icon and re-enabled buttons. Headless journeys use `onPick`; native journeys do not. No runtime dependency is patched and no picker result is fabricated. The temporary host and source tree are removed afterward.

### 原生主题动效

`python3 scripts/native_theme_acceptance.py --junit target/test-reports/native-theme-motion.xml` 使用无应用状态桥接的临时项目执行 `theme-motion-native.test.tsx`，验证正常 / 减少动效的弹簧采样、收敛、进度模式切换和浮层交互。结束后清理源码，保留 JUnit 与旁边的截图目录。该入口已接入 `python3 scripts/test.py e2e --target macos` 及其手动 CI 作业。
