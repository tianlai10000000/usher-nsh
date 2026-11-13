# 状态管理验收审计（SM6）

本记录核对当前已实现的状态管理与路由，不将更新产品界面、消息、账户等规划能力计入完成范围。实施清单见 [scaffolding](../scaffolding.md)，详细测试结果见 [测试方案](testing.md)。

## 1. 共享来源审核

2026-11-12 至 13 扫描 App、ui/pages、ui/state、ui/router、ui/features、ui/layout 及 Rust 应用服务。原始命令和输出保留在 target/test-reports/sm6-state-audit.txt。

| 状态 | 唯一来源与入口 | 验证 |
| --- | --- | --- |
| 外观、语言、主题、侧栏、状态栏、背景资源元数据 | SettingsService / SettingsStore → SettingsState → state/settings；命令集中在 state/transport | 设置集成、state-ui、原有 UI、原生多窗口偏好同步 |
| 诊断等级与实时健康 | 等级写入 settings；健康和 dropped 由 diagnostic_state 发布，旧 DTO 只保留兼容读取 | 真实日志线程错误发布及订阅测试 |
| 路由、历史、面包屑 | RouterService 中唯一 WindowSnapshot；ui/router 提供导航门面，pages 注册业务页面 | router、routes-check、router-ui、实际 App E2E |
| 分类、计数、预览、操作票据、导出结果 | WindowSnapshot.session；旧 navigation 删除，页面与 Shell 借用同一投影 | 窗口集成、往返保留、真实 HMR / 关闭重开 |
| 更新状态与待执行效果 | UpdateService 的 Machine + 私有领取队列，UpdateRegistry 按宿主设置身份共享 | 同一应用消费者在两个后端执行，隔离目录、并发领取、交接及恢复屏障；未接入产品 UI |
| 消息 / 账户 | 尚无业务数据源；布局只定义契约 | 后续批次，不创建占位共享 Store |

所有设置写入经 state/transport，窗口写入经 window-transport。WindowBackdrop 直接桥接只读取平台能力和图片字节，不写偏好；二进制和 Blob URL 不属于共享业务状态。Rust 的 SETTINGS 是目录注册表，STATUS 是诊断状态发布源，LOGGER 是 I/O 资源拥有者，NEXT_STORE_ID 只分配进程身份；均不是页面级重复业务状态。

## 2. 保留的局部状态

| 文件 / 对象 | 保留理由与清理 |
| --- | --- |
| ThemeSettings：accentDraft、draft、copiedStyle、scheme | 当前编辑器的未提交草稿、复制反馈和编辑明暗面，不跨页面保留；提交走设置门面 |
| DiagnosticsSettings.range；SettingsPage.width | 当前视图日期选择与布局测量 |
| menuPresence：drawerOpen、dockedLocal、mounted、wanted、sliding | 单个 Shell 的抽屉与动画目标；dockedLocal 随持久偏好投影更新，持久写入仍经 setSidebarDocked，计时器随 owner 清理 |
| windowChrome.measuredHeight | 原生布局测量，属于当前窗口控制器 |
| WindowBackdrop：materialReady、imageSource、active、objectURL | 原生材质就绪及图片资源句柄；替换 / 卸载撤销 URL、丢弃旧加载结果 |
| router guards / pending；window operations Map | 当前绑定的回调与请求标识，用于拒绝旧回执；不存第二份历史或业务结果，owner 清理后不可执行 |
| state/projection 与 runtime/nsh | 通用只读投影及 NSH 响应式实现，版本接收、取消和乱序在此统一处理 |

扫描未发现新的跨页面可写业务 signal。上述清单是人工语义审核，不把关键词扫描当作所有未来旁路的静态证明。新共享需求仍须登记到状态管理方案第 8.3 节。

## 3. 后端与依赖边界

usher-state 无默认 feature 时无正常依赖；backend-watch 仅增加 Tokio sync 与 pin-project-lite，无 runtime、网络或计时器。usher-test-support 无依赖且仅通过 dev-dependencies 使用。更新领域 crate 仍不依赖 usher-state、Tokio 或 NSH；只有应用适配依赖两者。原更新 API 仅增加 Machine 的 Eq / PartialEq，没有改字段、事件、构造方法或存储格式。相对 06ed67e，锁文件新增的外部包为 matchit 0.9.2（运行时）和 proptest 1.11.0、rand_xorshift 0.4.0、unarray 0.1.4（仅测试）；另有三个本地 crate，无旧版本包被替换。SM5 / SM6 本身没有新增外部依赖。

状态核心 Memory / watch 契约使用同一测试调用方；SM5 的应用消费者也分别注入两个后端，验证发布、领取、停止与恢复。故障注入验证提交失败保持队列和领域状态，不能提前执行效果。路由线性参考匹配器与 matchit 的替换契约另外运行。

## 4. 平台与宿主

目标编译与实际宿主分别记录，不能互相替代。当前产品支持的 macOS 开发宿主通过隔离源码 / 数据根中的真实 AppKit 多窗口及源码 HMR 验收；完整进程重启、更新安装、真实输入全产品旅程和其他平台宿主仍由对应功能 / 平台批次验收。

`python3 scripts/test.py state-targets` 已通过，读取 tests/config.json 的 platform_compile。四个库（usher-state / watch、usher-router / matchit、usher-update、usher-test-support）分别编译以下七个目标，Cargo JSON 中 28 组实际库产物均存在，不能只用 Finished 摘要或缺目标的输出通过：

| 平台 | 目标 | 纯库编译 | 实际宿主 |
| --- | --- | --- | --- |
| macOS | aarch64-apple-darwin、x86_64-apple-darwin | 通过 | 本机 Apple Silicon 原生验收；Intel 宿主未运行 |
| Windows | x86_64-pc-windows-msvc | 通过 | 未运行 |
| Linux | x86_64-unknown-linux-gnu | 通过 | 未运行 |
| Android | aarch64-linux-android | 通过，安装对应 rust-std 后执行 | 未运行 |
| iOS | aarch64-apple-ios、aarch64-apple-ios-sim | 通过 | 未运行 |

CI 从同一配置读取并安装 rust-std，再调用相同命令；没有新增运行时依赖。原生 macOS CI 仍要求专用解锁图形 runner，本轮没有触发远端 CI。纯库编译日志与产物证据位于 state-target-compile.log、state-target-artifacts.jsonl；应用 crate 另行使用隔离的 CARGO_TARGET_DIR=target/sm6-application-targets 执行同一七目标检查；完整命令保留在 sm6-application-targets.log。该轮失败于 NSH 依赖 ring 的 C 构建：缺少 x86_64-linux-gnu-gcc 和 Windows SDK 的 assert.h，不能计作完整应用跨平台通过。

随后对可用的四个 Apple 目标完整运行 `cargo check --locked -p usher-nsh --target aarch64-apple-darwin --target x86_64-apple-darwin --target aarch64-apple-ios --target aarch64-apple-ios-sim`，全部通过；四个 libusher_nsh 元数据文件存在，日志为 sm6-application-apple-targets.log。这是应用逻辑编译，不等同于 iOS / Intel 原生宿主运行。Windows、Linux、Android 的完整应用构建和各平台运行仍未验收。

本轮 macOS 原生回归 3 项全部通过，日志 native-run-mhvfodk9：两个窗口 StoreId 1 / 2 经真实 HMR 保留，两个旧 owner 清理，副窗口关闭重开取得 5，偏好仍为 dark；native-isolation.json 的临时源码 / 数据根已实际确认删除。

## 5. 同条件发布构建成本

复验命令：

```sh
python3 scripts/state_cost.py --baseline 06ed67e
```

脚本将指定 Git 基线和当前工作区分别复制到临时目录，顺序构建全新的 Cargo target；两者使用同一工具链、缓存的 registry、standard / arm64 和 .cargo/config.toml 发布配置（opt-level=z、fat LTO、单 codegen unit、strip）。不启动、安装或发布产物，不需要用户数据目录。日志、逐文件输入哈希和测量结果保留在 target/test-reports/state-cost-*/，临时源码及构建目录在结束时删除。

该比较覆盖基线以来的状态 / 路由等全部未提交源码差异，不能将差值单独归因于 Tokio 或 SM5；SM5 尚未接入产品入口，其未使用代码可能被链接器消除。单次墙钟时间只作构建成本记录，不代表统计性能结论。2026-11-12 至 13 实测如下，原始证据位于 target/test-reports/state-cost-9j3qic7y/comparison.json。

| 指标 | 基线 06ed67e | 当前源码 | 差值 |
| --- | --- | --- | --- |
| .app 普通文件总字节（不计符号链接） | 23,886,721 | 24,148,603 | +261,882（约 255.74 KiB，+1.10%） |
| 主可执行文件字节 | 23,770,272 | 24,018,608 | +248,336（约 242.52 KiB） |
| 单次冷 target 构建墙钟秒 | 328.96 | 334.17 | +5.21（约 +1.59%） |

两次构建均通过，生产更新服务源码哈希已与最终工作区核对；临时构建树已删除，日志与输入清单保留。构建期间还运行了短 Rust / Python 回归，墙钟受调度及系统负载影响；这里只报告观察值，不能据此断言性能退化或改进。未运行这些发布产物，不把成功打包计作发布宿主、签名升级或冷启动性能验收。

## 6. 完成状态与剩余边界

SM5 应用服务交付完成。SM6 本机的迁移审计、后端替换、完整回归、原生 macOS 生命周期、七目标纯库 / 四目标应用编译和发布构建成本已完成；整体标为部分完成。缺口是 Windows / Linux / Android 完整应用工具链与构建、非本机平台宿主、远端 CI，以及分发产物与实际升级旅程（本机 ad-hoc 发布运行已由第 8 节补齐）。这里不把第三方工具链缺失归为状态库失败，也不把纯库编译替代宿主验收；现有证据可供对应平台接入时复验。

## 7. RT4 后的回归更新（2026-11-13）

新增目录迁移仍由窗口 `usher-state` 一次发布，不增加第二份历史或业务共享缓存。已通过 229 项工作区测试、36 项应用集成、21 项状态 UI、11 项路由 UI、29 项应用 UI、4 项无头 E2E 和 4 项 macOS 原生验收；后者包括真实目录版本 HMR 和异步守卫返回。结果详见 [测试方案第 17 节](testing.md)。

SM1–SM5 状态不变，SM6 的完整跨平台应用 / 宿主、远端 CI 和分发 / 升级缺口仍保留；本机 ad-hoc 发布运行和 VM reload 的补充证据见下一节。第 5 节发布构建成本来自 RT4 之前的源码快照，不代表本批重新测量。

## 8. 本机发布运行、VM reload 与环境复核（2026-11-13）

RT4 解锁会话下 macOS arm64 ad-hoc 发布包 3 项通过：真实系统 URL 注册、外部冷启动、运行中隐藏应用收到 URL 后恢复和激活，保持同一 PID。临时命名空间和系统注册清理完成。它补齐本机 release 运行的路由验收，不等同于升级或公证分发。完整 Cmd+R VM reload 另有 1 项通过，持久偏好恢复且窗口会话重置、旧回调失效；报告见测试方案第 18、19 节。

当前源码复验通过：Rust 工作区 229 项、Python 104 项、四库七目标 28 组产物、应用库 Apple 四目标，以及 nsh check / actionlint。未重测第 5 节的包体积成本。未更改共享状态接口或持久化格式。

iOS 模拟器清单可读取，但当前 Xcode 首次启动检查返回 69，CoreSimulator 安装版 1174.9.2 低于包装器要求 1174.21，原生验收未运行；没有修改模拟器数据或安装系统组件。Windows / Linux / Android 的工具链和宿主、远端 CI 仍不可用。上一轮旧更新模拟改写 HOME，故未运行；本轮隔离改造见第 9 节，完整实际升级仍依赖更新方案批次 4、5 和真实升级产物。SM6 继续保持部分完成。

## 9. 更新启动模拟的隔离改造

已将旧 HOME 重定向替换为唯一测试应用 ID 和每例临时 data/cache/saved-state。脚本拥有应用进程，确认退出后再清理；停止失败或命名空间归属改变时保留隔离。产品设置、桥接、状态机与持久化格式没有变化。原生测试脚本复用同一链接保护模块，新增 16 项隔离 / 清理单测，Python 全部 120 项通过。真实 SIGTERM 中断后的测试应用、密钥、数据树和链接检查无残留，失败 / 中断运行未累计为通过。

本轮验证范围仍是预置 next 代码包的启动、健康晋升与拒绝 / 回滚行为，不覆盖应用主动检查、下载、整包安装、公证分发或远端 CI；SM6 保持部分完成。最终完整场景结果见测试方案第 20 节。

最终完整运行 update-sim-d8uds6dq：健康晋升、启动回滚、降级拒绝、篡改拒绝共 4 项通过，0 失败，7 项 pending；16 个唯一隔离路径全部清理，密钥、源码和临时应用也已删除。证据为 report.json、cleanup-audit.json 和逐场景应用日志。该命令已纳入手工 native_state CI，本轮只完成本机执行；实际检查 / 下载 / 安装与跨平台缺口没有变更。
