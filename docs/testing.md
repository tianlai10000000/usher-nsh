# 单元、集成与端到端测试方案

本文定义通用状态、独立路由和现有页面整改的测试组织、mock 数据、命令及分批验收。配合 [状态管理](state-management.md)、[路由](routing.md) 和 [项目实施清单](../scaffolding.md) 使用。**SM1 至 SM5、RT1 至 RT3、UT1 已完成应用服务范围。SM4 的 macOS 同 VM 多窗口、隔离数据目录、真实源码 HMR 和窗口关闭重开已通过；跨进程重启、目录升级及其他原生平台仍待后续批次。通用状态证据见第 15、16 节；更新整包的跨进程重启与设置保留见文末批次 6，历次记录保留当时的状态。** 测试必须随所属功能批次交付，不能等全部重构完成后再补。

## 1. GitHub 调研与采用范围

2026-11-12 阅读以下上游仓库的官方文档和测试源码。表中区分上游实践与本项目适配；这些项目不共同规定一套适用于 NSH 的现成测试架构。

| 一手资料 | 已核对的实践 | 本项目采用方式 |
| --- | --- | --- |
| [Rust Book：Test Organization](https://github.com/rust-lang/book/blob/main/src/ch11-03-test-organization.md) | 模块内 `#[cfg(test)]` 单测；crate 的 `tests/` 从公共接口做集成测试；共享辅助代码放子目录 | 按 crate 就近组织规则单测和外部契约测试；测试辅助模块不进入生产依赖 |
| [Tokio：sync_watch.rs](https://github.com/tokio-rs/tokio/blob/master/tokio/tests/sync_watch.rs) | 显式控制 poll / 唤醒；多接收者、最终值、关闭、重新订阅分别断言 | 测试项目封装后的最新值、取消、关闭契约；竞态通过屏障和可控执行顺序触发，不靠 sleep 碰运气 |
| [Vue Router：memory.spec.ts](https://github.com/vuejs/router/blob/main/packages/router/__tests__/history/memory.spec.ts) | 独立内存历史实例；push、replace、往返、截断前进分支和监听清理 | 路由核心无需 UI 即可验证历史；不照搬其起始位置或监听通知语义，以本项目公开契约为准 |
| [Vue Router：router.spec.ts](https://github.com/vuejs/router/blob/main/packages/router/__tests__/router.spec.ts) | 可控 Promise 模拟守卫完成顺序，检验较晚完成的旧导航不会覆盖新导航；单独验证解析及重定向 | 为导航票据、取消、迟到回执编写确定性测试，匹配器替换复用同一组外部断言 |
| [Proptest README](https://github.com/proptest-rs/proptest/blob/main/README.md) | 属性测试补充边界例子；缩减失败输入并持久保存回归样本；避免在测试中复制被测算法 | 用固定 `proptest 1.11.0` 作为 dev-dependency，验证路径往返与历史不变量；保存种子和最小失败输入，保留明确边界单测 |
| [Playwright：Best Practices](https://github.com/microsoft/playwright/blob/main/docs/src/best-practices-js.md) | 验证用户可见行为；每例独立数据；稳定定位；等待可观察条件；受控外部服务与失败诊断 | 将原则用于 NSH 原生测试；使用现有 `nsh/test` 驱动、testID、条件等待和宿主日志，不将浏览器 DOM 驱动当作 AppKit 驱动 |

首版使用 Rust 自带测试框架、现有 `nsh/test`、已有 Python unittest 和必要的 proptest；SM1 使用明确边界与可控并发用例，RT1 已引入固定版本 proptest 作为仅开发依赖，用于路径 / 历史属性测试；不新造测试执行框架、不要求生产代码依赖一个全局“测试 crate”，也不为 NSH 页面引入 Vue / Vitest / jsdom。简单边界使用实现业务 trait 的小型 fake；只有出现明确的复杂 mock 需求时才另行评估库。测试模块的独立性体现为目录、夹具、公共契约和独立命令。

## 2. 当前基础与能力边界

已检查 [CI](../.github/workflows/ci.yml)、[现有 UI 用例](../tests/ui/)、项目固定 NSH 的 `.nsh/types/nsh/test.d.ts`，并读取本机 `nsh test --help`。设计阶段仅核对能力；SM1 已运行核心与工作区测试，SM2 进一步运行设置应用集成及现有外观无头回归；结果分别见第 9、11 节；SM3 的真实订阅无头验证见第 12 节，不能替代原生实验。

- 现有 Rust 测试覆盖设置、主题及更新等；CI 分别运行更新 crate 和应用 `--lib`。SM2 已追加 `integration` 步骤，显式执行 Cargo `settings_state` 目标；`--lib` 本身不会自动执行该目标。
- SM1 已增加独立 Ubuntu state 作业，调用与本机相同的 `python3 scripts/test.py state`，并检查 watch feature 的 Clippy；通用工具抽取后增加 `python3 scripts/test.py test-support`。该作业未在 GitHub 实际运行。设置应用集成已在 macOS rust 作业配置；SM3 的 TS / 桥接与 UI 递归发现已接入 macOS ui 作业；RT1 追加 router / routes-check，RT2 追加窗口集成，RT3 / SM4 追加 router-ui / e2e 无头旅程。
- 现有 UI CI 已使用统一递归发现，覆盖子目录和 `.test.ts` / `.test.tsx` 文件。已有 `window-shell`、`appearance`、`diagnostics`、布局和 AppKit 用例保留并随迁移更新。
- `nsh test` 支持 `.test.ts` / `.test.tsx`、`--filter`、`--junit`、`--trace` 和 `--target macos` 等开发宿主。`open({ platform: "macos" })` 只设置无头环境的平台属性，并不等于启动原生窗口。
- `open` 支持 `modules`、网络响应和手动时钟；`modules` 替身、URL 注入及部分生命周期模拟仅限无头。函数型 module fake 返回 Promise，不能当作同步导出的完全等价替代。原生目标不能沿用含 `modules` 的用例。
- `open` 的 `storage` 是 NSH storage 初值，不是 Rust 设置文件目录。原生设置使用宿主数据目录；本批已通过显式 --data-dir 验证隔离目录和同仓库多窗口，跨进程重启仍待验证。不能通过修改真实用户设置完成测试。
- `--target` 运行开发宿主，不证明打包产物可启动、签名正确或可升级；发布验收继续遵循 [更新方案](update.md) 与 [部署方案](deploy.md)。

## 3. 测试分层与目录

| 层次 | 被测范围 | 允许替换的边界 | 不能据此宣称 |
| --- | --- | --- | --- |
| 规则单测 | 状态转换、路由解析 / 历史、selector、DTO 比较 | 时钟、ID 来源、纯输入 | Rust↔JS 桥接或原生行为通过 |
| crate 契约测试 | 公开 Store API、匹配器接口及替换 | 后端工厂 / 匹配器按同一契约切换 | fake 通过即实际后端通过 |
| 应用集成 | 真实领域服务 + Store + Router + 序列化 + 临时持久化 | 可控 I/O 故障、外部服务 | 跨语言运行时或真实点击已验证 |
| UI 组件 / 投影 | NSH 绑定、RouterView、按钮和错误显示 | transport / native module fake | 真实 Rust 命令与订阅闭环通过 |
| 应用 E2E | 实际 App 页面 → TS 门面 → Rust 桥接 → 服务 / Store → 订阅 → 页面 | 外部网络、测试数据与时钟；保留真实被测链路 | 无头结果等于原生、开发宿主等于发布产物 |
| 发布产物烟测 | 实际打包应用的启动、设置保留、升级 / 回滚 | 隔离账号、测试源、临时签名材料 | 未运行平台的验收通过 |

采用模块就近、应用集成和 E2E 集中的组织方式。状态 / 路由核心、测试工具、设置和窗口应用集成、TS 适配层、真实桥接、现有 UI / 脚本及无头 E2E 均已实现；标注 Planned 的条目保留为后续覆盖：

```text
crates/usher-test-support/src/          # Generic polling and fixture identity helpers
crates/usher-logging/src/               # Logging rule and lifecycle unit tests
crates/usher-logging/tests/             # Public capture/failure/file backend contracts
crates/usher-logging/tests/support/     # Logging-owned mock writers
crates/usher-state/src/                 # Unit tests beside the rules
crates/usher-state/tests/               # Public contracts and concurrency
crates/usher-state/tests/support/       # State-specific fake backend and test machine
crates/usher-state/tests/fixtures/       # Crate-owned state cases
crates/usher-router/src/                # Parser and transition unit tests
crates/usher-router/tests/               # Matcher contracts and properties
crates/usher-router/tests/fixtures/      # Synthetic route catalogs and cases
ui/state/__tests__/                    # Facade and projection tests (SM3)
ui/router/__tests__/                   # Facade/guard contracts and rendering failure injection
ui/features/<feature>/__tests__/        # Planned feature-owned UI tests
tests/integration/settings_state.rs    # Real settings service and persistence (SM2)
tests/integration/state_router.rs      # Window store, guards, sessions and wire contracts
tests/integration/diagnostics.rs       # Settings, runtime preferences and real log archives
tests/integration/update_state.rs      # Shared update state and fenced effect execution
tests/integration/*.test.tsx           # Real subscriptions, RouterView and owner lifecycle
tests/integration/fixtures/            # Settings documents, themes and bridge fixtures
tests/integration/support/             # Application-specific setup and cleanup
tests/fixtures/contracts/              # Planned shared Rust/TS wire scenarios
tests/ui/                              # Existing application UI regressions
tests/e2e/                             # Real bridge and user journeys
tests/e2e/fixtures/                    # Isolated headless session options
tests/config.json                      # Suites, targets, fixtures and time limits
scripts/test.py                        # Thin command orchestration, not assertions
scripts/tests/                         # Script-owned unit tests
```

Rust 契约测试只 import 被测 crate 的公共 API。crate 夹具自包含，不依赖根应用或 NSH；统一跨语言 DTO 样本目录 `tests/fixtures/contracts/` 仍待实施；当前由 Rust wire 断言及真实 JS 桥接分别验证，不宣称已有共享文件。Rust 单测允许读取私有规则，但不用公开生产接口来满足测试的内部窥探。辅助代码先保持局部，确有多个调用方时再复用，禁止循环依赖。

这里的“集成集中”指跨模块的应用测试；Cargo 定义的 crate 公共接口集成测试仍在所属 crate 的 `tests/`。SM2 已在根 `Cargo.toml` 显式声明 `[[test]]` 的 `name = "settings_state"` 和路径 `tests/integration/settings_state.rs`；RT2 已按相同方式声明 `state_router` 目标。Cargo 不会自动递归发现任意嵌套路径，不添加空测试目标。TS 单测和组件测试放模块的 `__tests__/`，SM3 已将状态模块目录纳入配置与 CI 递归发现，后续模块按批次追加范围，不再建立统一 `tests/unit/`；现有 `tests/ui/` 应用回归保留。

TS 纯逻辑用例同样使用 `nsh/test` 的 `test` / `expect`，无需 `open` 窗口；实际响应式跟踪与组件清理另在 UI 层运行。UI helper 只封装重复的用户动作、等待与释放，不复制路由决策、状态转换或实现第二套业务模型。

### 3.1 通用测试支持 crate

[`usher-test-support`](../crates/usher-test-support/src/lib.rs) 是 workspace 内不可发布的测试支持库，只通过 `[dev-dependencies]` 使用，遵循 [Cargo 开发依赖规则](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#development-dependencies)。它没有外部依赖，也不依赖 `usher-state`、业务 crate、NSH 或平台宿主；生产依赖图中不得引入它。

初次抽取 `task::WakeCounter`、`task::poll` 和 `task::ready`；RT1 增加 `fixture::assert_unique_ids`，状态与路由共用非空 / 唯一场景身份校验。每例独立创建唤醒观察器，`poll` 只执行一次，`ready` 要求第一次 poll 就完成，否则立即失败；它不是 executor，不启动运行时、重试或 sleep。工具自身的单测就近放在 `task.rs`，通过独立命令执行，因为测试消费方不会自动执行其 dev-dependency 的单测。

状态测试的参考后端、故障开关、`Machine` / 命令和 `contracts.json` 保留在 `usher-state/tests/`。设置、路由、更新等业务 mock 和夹具也由所属模块维护。以后只有跨模块复用且无业务依赖的工具才进入支持 crate；不预先添加时钟、端口或临时目录抽象。Rust / TS 共享的是应用契约数据，TS helper 由 TS 测试维护，不直接依赖 Rust 测试 crate。

## 4. 单元与契约测试设计

### 4.1 状态管理

`usher-state` 以同一消费者、相同动作和期望结果运行内存参考后端与实际 watch 后端。无默认 feature 的套件仍执行纯转换和参考后端测试，启用 `backend-watch` 后额外运行真实后端；不以 feature 条件把全部契约测试过滤成零条。

| 套件 | 必测行为与判定 |
| --- | --- |
| 纯转换 | Changed / Unchanged、合法与非法动作、仅有效果；失败保持旧快照，效果执行不进入通用 crate |
| 发布与订阅 | 无订阅者时仍保存最新状态；首次订阅与发布交错不丢最终值；慢订阅者允许跳号但收敛到最新；发布顺序与内容一致 |
| 关闭与隔离 | 最终值和终止语义符合接口；重复取消安全；关闭后发布失败；不同 Store 实例与应用 / 窗口作用域不混用 |
| 故障原子性 | 发布预检失败不改变状态或序号；可控后端失败覆盖规定的提交边界；设置写失败、保存重试即使 revision 不变也通知 |
| 并发 | 使用线程屏障、通道和可控投递触发竞争；序号不回绕、提交不交叉；回调不在锁内重入，取消和提交竞争不泄漏 |
| 桥接投影 | 命令回执与订阅乱序、重复快照、旧实例回调、十进制序号跨位数及超过 JS 安全整数；单个待投递快照合并、页面无权释放共享投影 |

业务服务 I/O 故障放应用测试，不把文件系统注入 `usher-state`。Tokio 测试的 poll 技法用于对应异步适配，纯核心测试不因此依赖 Tokio 运行时。有限调度测试不宣称穷尽线程交错；若后续自行实现同步原语，再独立评估模型检查，首版不重写 watch。

### 4.2 路由

| 套件 | 必测行为与判定 |
| --- | --- |
| 路由目录 | 重复 ID / 路径、缺失父级、父级环、未知 schema / pageKey、参数冲突与生成文件过期均失败；注册失败不部分替换目录 |
| 匹配 / 格式化 | 静态与参数优先级、通配边界、Unicode、只编解码一次、非法编码及分隔符、尾斜杠、多值 query；命名目标与合法路径分别处理 |
| 历史 | push、replace、go、边界 no-op、重复导航、后退再 push 截断；entryKey 唯一，取消不改变当前位置与历史 |
| 守卫与竞态 | 允许、拒绝、异常、重定向环与上限；手动释放两个待决守卫，旧请求最后完成也不能覆盖新提交；关闭和目录更新使旧票据失效 |
| 替换 | matchit 适配与简单测试匹配器在约定语法范围内通过同一契约；语法不支持必须明确拒绝，不能让 fake 无条件返回预期匹配 |
| 属性 | 有效目标 format→resolve 等价；任意动作序列的游标始终合法；replace 不增加历史长度；失败转换保持旧状态；断言不复制实现算法 |

守卫效果、窗口身份和唯一 Store 提交属于应用集成；纯 crate 不承担宿主或 UI 测试。Property 测试使用有效输入生成器，已知非法输入另列用例。随机运行报告种子、用例数量和最小失败输入；失败样本进入版本管理并补明确回归。测试数量和超时从测试配置提供，不能以重跑代替修复。

## 5. Mock 数据与隔离契约

夹具使用人工构造数据，不复制用户设置。固定场景记录 `schema_version`、`case_id`、初始输入、动作 / 投递顺序、预期可观察结果；路径、窗口尺寸、超时、固定时间和随机种子由测试配置或具名夹具提供。数值只是测试输入，不作为生产默认值。新配置值按 CONTRIBUTING 评审，复用已有主题、语言与能力配置。

| 夹具组 | 数据及故障 | 使用范围 |
| --- | --- | --- |
| state | 无变化 / 拒绝 / 连续发布、关闭、迟到快照、旧实例、序号精度边界 | 两种后端契约及 TS 投影；测试领域不依赖真实设置业务 |
| settings | 现有 schema、旧 / 未来 schema、损坏数据、写失败 / 重试、两份独立临时目录 | 应用集成；E2E 从真实设置服务读取对应临时数据 |
| routing | 最小有效目录、参数与 query、冲突 / 环、未知路径、历史动作序列 | crate、注册和生成器；与业务真实 routes.json 验证分开 |
| async | 守卫 A / B 受控完成、回执乱序、取消后投递、HMR / 关闭旧绑定 | 单测、桥接组件测试；不在真实 E2E 替换 state / router |
| external | 本地服务的正常、离线、延迟和坏响应；更新相关场景复用 `scripts/update-sim/` | 集成和 E2E；不依赖公网服务稳定性，不复制第二套更新模拟器 |

Fake 实现应校验输入形状，对未声明调用明确失败，不能静默返回成功。Mock 的 DTO 必须由真实序列化契约校验；场景与预期分开加载，期望不能由被测实现自动算出。可控时钟只覆盖被注入的时钟域，NSH 手动时钟不会自动控制 Rust 后台任务。

每例独立建立和释放 Store、窗口、订阅、任务及临时目录；异常路径使用 Rust RAII 或 TS `finally` 清理。只在“同一例的重启 / 多窗口场景”共享特定数据目录。并行例不能共用文件、端口或应用单例；外部服务由系统分配端口并把地址注入配置。报告记录 fixture 身份、seed、平台、NSH / Rust 版本和代码提交，必要时附脱敏后的最后快照及请求顺序。

## 6. 端到端测试模块

### 6.1 驱动与真实链路

沿用 `nsh/test`：无头运行快速用户旅程，`--target macos` 等执行原生开发宿主旅程。App 必须装配真实 Router、State、领域服务和桥接；E2E 不能通过 `modules` 替换这些模块、直接把目标快照写进页面，或绕过动作入口。测试数据只用于准备环境，断言从用户可见页面、真实状态回读和临时持久化结果取得。

TE1 必须先完成数据目录隔离与真实桥接探针：用公开宿主配置或应用装配边界将目录注入生产同款仓库，隔离设置、日志、主题和导出。若 NSH 无法提供必要配置，记录能力缺口并在应用边界设计仅测试构建可用的配置适配；不能假设存在未核对的 NSH 参数，也不能开放发布版的任意状态写入后门。未隔离前，不运行会修改宿主设置的 native E2E。

探针需要证明点击产生真实 Rust 命令、真实 Store 更新及订阅到达页面；另外从 Rust 应用入口提交一次变更，验证未发命令的另一消费者更新，防止测试仅覆盖命令回执。跨窗口测试必须是同一应用作用域内的两个窗口；两个独立 `open` 引擎不自动证明这种共享。若驱动缺少能力，该项标为待验证，不以假快照替代。

### 6.2 必交付的用户旅程

| 编号 | 旅程 | 关键断言 / 所属批次 |
| --- | --- | --- |
| E2E-01 | 从首页进入设置，经按钮 / 快捷键 / 面包屑后退前进，再从历史中间导航 | 页面与当前位置一致、按钮边界正确、前进分支截断；RT3 |
| E2E-02 | 首页操作计数 → 设置修改外观 / 语言 / 分类 → 返回首页 → 再进设置 | 计数与窗口分类保留；设置和常驻 Shell 一致；页面卸载没有第二份 Store；SM4 + RT3 |
| E2E-03 | 设置页修改透明度预览或启动导出后切页再返回 | 共享预览、反馈及结果符合 §8.3 迁移约定，不重复执行任务；SM4 |
| E2E-04 | 两窗口共享偏好，其中一窗导航 / 分类变化，关闭后重开 | 应用偏好同步；历史与窗口会话互不污染；订阅释放，重开取得最新值；SM3 / SM4 |
| E2E-05 | 导航守卫允许 / 拒绝 / 失败、快速重复操作、未知合法路径、无效命名目标 | 拒绝和失败保留页面，NotFound 可返回，迟到结果不跳页；RT2 / RT3。无业务入口的通用分支用独立测试页面，不能添加产品演示入口 |
| E2E-06 | 保存设置后结束进程并重新启动；设置损坏或保存失败后恢复 | 真正落盘与恢复可观察；只读 / 失败不能被当成保存成功；SM2 + TE1 |
| E2E-07 | 组件重挂载、引擎重建、目录更新与关闭时有待决操作 | 分别验证相应保留 / 重置语义，旧回调无效；SM3 / RT4。单纯卸载不是 HMR 实验 |

测试页面只能位于 `tests/`，用测试装配加载；业务 `ui/pages/` 仅包含实际产品页面。原生返回、菜单、快捷键、焦点及窗口缩放在目标宿主验证。NSH 的 `app.forward()` 仅支持 web history，桌面 Router.forward 使用应用按钮或快捷键；不能据此错误调用浏览器专用驱动。

E2E-05 中仅实际 App 可触发的分支计为产品旅程；尚无业务入口的守卫 / 异常分支通过测试页面验证时，报告归类为路由跨层集成，不能冒充现有产品页面 E2E。此类覆盖不要求先增加业务功能，也不免除相应核心契约测试。

### 6.3 等待、诊断与可靠性

优先使用稳定 testID 或明确用户文案。测试异步结果时先 `waitFor` / `waitUntil`，再用 NSH 的同步 expect 检查；等待目标页特有元素，不能拿两页共有的“当前外观”证明已换页。禁止用固定 sleep 等待保存、渲染或导航。

失败保留 JUnit、stdout / stderr、用例和平台配置、动作记录，必要时使用 `--trace` 的挂载 / commit 日志；图形目标可保存截图，无头不声称有真实像素。NSH trace 不是 Playwright Trace Viewer 文件。首轮失败必须报告；诊断性重跑保存两次结果，不能把偶发成功改为稳定通过。截图基线只在相同系统 / NSH / 字体 / 主题条件比较，由评审确认，不能自动接受新基线。

## 7. 验证命令与 CI

所有命令在仓库根目录执行。当前可用入口包括 `cargo test --locked --workspace`、`cargo test --locked -p usher-update`、`nsh check`、`nsh test tests/ui/window-shell.test.tsx` 和 `python3 -m unittest discover -s scripts/tests -p 'test_*.py' -v`；SM1 的实际执行记录见第 9 节。

以下入口均已实现；`unit` 聚合入口仍为后续规划，当前调用会失败，不运行占位测试：

| 命令 | 内部动作与夹具 | 当前结果 |
| --- | --- | --- |
| `python3 scripts/test.py state` | 无默认 feature 及 backend-watch，两后端共用状态契约 | 20 / 35 项 |
| `python3 scripts/test.py test-support` | 无业务依赖的 poll / ready / WakeCounter、夹具身份校验 | 6 项 |
| `python3 scripts/test.py router` | 无默认 feature 及 matcher-matchit，参考匹配器 / 真实后端同契约，固定 seed 属性、生成和历史边界 | 9 / 14 项 |
| `python3 scripts/test.py integration` | 显式 Cargo settings_state、state_router 和 update_state，真实服务及隔离 fixtures | 设置 11 + 窗口路由 8 + 更新 15 项 |
| `python3 scripts/test.py update-state` | 原更新 crate、完整应用 lib、更新双后端集成 | 38 + 75 + 15 项；与工作区 / integration 部分重叠 |
| `python3 scripts/test.py state-targets` | 从配置编译七个目标的状态、路由、更新领域和测试支持库 | 编译检查单列，不计入测试数 |
| `python3 scripts/test.py state-ui` | ui/state/__tests__ 与 tests/integration 全部 TS / TSX；替代投影和真实桥接 | 4 文件、20 项 |
| `python3 scripts/test.py router-ui` | ui/router/__tests__ 门面 mock、RouterView 错误注入与 tests/integration/router.test.tsx 真实桥接 | 3 文件、8 项 |
| `python3 scripts/test.py ui` | tests/ui 原有应用回归；类型检查另外运行 nsh check | 11 文件、29 项 |
| `python3 scripts/test.py routes-check` | 生成器契约测试及 CLI check 真实目录 / 生成文件；检查不写文件，页面绑定由 nsh check 穷尽验证 | 3 项测试 + 1 项检查 |
| `python3 scripts/test.py e2e --target headless` | 实际 App、真实桥接、隔离内存会话、每例显式销毁；无业务 modules mock | 2 项旅程 |
| `python3 scripts/test.py e2e --target macos` | 原生宿主显式临时数据根、同 VM 两窗口、官方 watcher 的真实 HMR；按阶段断言并输出 JUnit | 3 项原生生命周期验收；需解锁的 macOS 图形会话 |

router-ui 的真实桥接 3 项也包含在 state-ui 中；routes-check 的 3 项同样属于 router-matchit。各套件不能直接相加为独立测试总数。生成新鲜度以 required_output 及退出码核对，报告 checks，不伪造 passed 测试数。

`tests/config.json` 是套件发现范围、目标能力和测试参数的单一入口，默认场景无需手工准备数据。命令包装器只负责解析、环境预检、按确定顺序调用现有 runner、清理和聚合报告，不编写业务断言。每个 NSH 文件调用写入唯一 `--junit` 路径，先删除旧报告，再核对进程退出码、非零用例、文件集合、失败 / 跳过节点和必测组。NSH 0.5.0 的目录参数会按应用入口处理，因此包装器先递归发现源文件，排除 `.nsh`、`node_modules`、`target` 生成目录，再逐文件运行；不依赖不存在的目录测试选项。所有 NSH 命令在同一 checkout 内串行执行，避免共享 `.nsh` 生成产物相互覆盖。

未知 suite / target、零测试发现、夹具不合法、缺失 required 场景、报告缺失、原生能力不足或任何子命令失败均非零退出。能力矩阵区分 required 与明确不适用，不能运行时随意 skip 后报告成功。支持单例重放，但不能把过滤后的局部通过冒充完整验收。CI 必须发现新目录，不能继续只用根层 glob。

CI 已配置状态两个 feature 组合、路由契约、应用集成、TS / UI 和路由目录检查；纯 crate 可在无 NSH 环境运行，应用 / UI 先沿用 macOS runner。真实桥接无头旅程已配置为 CI 步骤，远端结果尚未验证；原生 macOS 已在本机通过，CI 仅通过 workflow_dispatch 的 native_state 开关运行，要求带 usher-native 标签且桌面解锁的 self-hosted macOS runner；尚未在远端执行，也未设为 PR 必需门禁。不能假定托管 runner 提供此能力。Windows / Linux / Android / iOS 分别记录编译和原生能力结果，未配置作业不算覆盖。打包和更新演练沿用发布流水线，另记证据。

## 8. 分批实施与完成条件

| 批次 / 任务 | 前置 | 交付与完成条件 | 状态 |
| --- | --- | --- | --- |
| UT1 / 07.06 | SM1、RT1 | 状态与路由参考 / 真实后端契约、故障边界、fixtures、属性测试、命令与 CI 配置 | 已完成；第 13 节 |
| UT2 / 07.07 | UT1、SM2 / SM3 / RT2 | 设置 / 窗口集成、TS 投影与守卫 mock、真实订阅 / 路由桥接、生成检查已交付；RouterView 重试 / 返回错误注入已通过；统一跨语言 DTO fixture 目录待补 | 部分完成 |
| TE1 / 07.08 | SM2 / SM3；与 UT2 并行 | 无头及 macOS 原生目录隔离、同 VM 多窗口、真实 HMR、JUnit / JSON 和 CI 入口已交付；跨进程重启、其他平台及远端原生 runner 待验证 | 部分完成 |
| TE2 / 07.09 | TE1、SM4 / RT3；扩展项随 RT4 | 两项实际 App 旅程及补充桥接测试已通过；E2E-01 至 E2E-07 的范围和原生缺口见第 13 节 | 部分完成 |
| 测试支持抽取 / 07.10 | SM1 / UT1 状态部分 | 独立无业务依赖的 `usher-test-support`；仅开发依赖；状态 mock / fixtures 保留原模块；明确目录归属，工具单测、原契约、命令和 CI 验证 | 已完成，见第 10 节 |

07.01 继续跟踪现有单测覆盖，07.02 汇总跨层和平台验收，07.06 至 07.09 是它们的新设计子任务，不能重复记为两次交付。UT1 在核心接口实现时交付，UT2 在桥接实现时交付；TE1 提前验证运行条件。RT3 + SM4 的完成依赖对应旅程通过，不能因为测试框架已有命令就把页面迁移标为完成。RT4 扩展能力不阻塞既有两页整改，但深链接、HMR 和原生返回不得提前标为已验收。

交付证据必须列出命令、实际非零用例数量、fixture / seed、平台与版本、结果和剩余缺口。用于验证测试有效性的故障注入限定在测试数据或可还原的临时副本，不能把故意损坏的实现留在工作区。文档链接及设计审查通过不等于上述代码测试通过。


## 9. SM1 / UT1 状态部分实施记录

2026-11-12 在 macOS、Rust 1.98.1、Python 3.14.6、NSH 0.5.0 的当前工作区完成验证。核心测试不启动 NSH，不连接网络或操作用户设置。使用 [contracts.json](../crates/usher-state/tests/fixtures/contracts.json) 的固定输入，无随机 seed；并发测试使用屏障与显式 poll，不使用定时 sleep。

| 实际命令 / 验证 | 结果 |
| --- | --- |
| `python3 scripts/test.py state` | 两组通过：无默认 feature 为 1 单测 + 17 契约 / 故障测试 + 2 编译失败文档测试，共 20；watch 组合为 1 + 32 + 2，共 35。两组包含重复的参考契约，不把 55 次执行称作 55 个独立场景 |
| `cargo test --locked --workspace` | 共 136 项通过，包含状态默认 feature 的 20 项；不能代替上行显式 watch 组合 |
| `python3 -m unittest discover -s scripts/tests -p 'test_*.py' -q` | 89 项通过，其中命令包装器 8 项，覆盖空结果、缺必测组、过滤 / 忽略、子命令失败、未知 suite、超时报告与进程树清理调用 |
| `cargo fmt --all --check`；`cargo clippy --workspace --all-targets --locked -- -D warnings` | 通过 |
| `cargo clippy --locked -p usher-state --all-targets --features backend-watch -- -D warnings` | 可选后端及测试通过 |
| `nsh check`；`actionlint -color` | 通过；未运行原生 UI 旅程 |
| 独立依赖树 | 核心无正常外部依赖；watch 仅 Tokio sync 与 pin-project-lite；无 NSH、业务或平台依赖 |

两后端共用 15 组消费契约，覆盖领域变更 / 拒绝 / 仅效果、无消费者发布、快慢消费者、注册竞争、取消与发布竞争、丢弃等待释放任务、同步唤醒时重入读取、关闭前最终值、唯一写入口串行提交，以及非 Clone 状态和 Send / Sync 类型约束。另有后端失败原子性、资源释放和序号耗尽测试。参考实现与故障开关均位于 crate 的 `tests/`；生产默认 feature 为空。

验证测试有效性时，将 crate 复制到临时独立工作区，逐一改坏再运行指定契约。三个变异均产生目标用例失败并以 101 退出，临时副本已清理：

| 故意改坏的逻辑 | 捕获它的用例 |
| --- | --- |
| 发布后序号错误地归零 | `watch::slow_and_fast_consumers` |
| cancel 不记录取消标记 | `watch::cancellation_is_local` |
| 订阅初值只 borrow 而不标为已读 | `watch::latest_without_consumers` |

状态命令读取 [tests/config.json](../tests/config.json)，超时和必测组在配置中声明；任一组合失败整体返回非零，但继续执行另一个组合并保留证据。超时 / 中断终止 runner 的进程树。日志与 JSON 汇总生成于 `target/test-reports/`，不进入版本管理；本次变异记录另保存在 `state-mutations.json`。新增 GitHub state 作业和现有脚本测试作业接入相同入口；远端运行结果尚未取得，Windows 进程树清理分支与其他平台也未实测。

SM1 / 03.08 标为完成。UT1 / 07.06 的状态部分完成，整体保持部分完成：路由 crate、router 命令、路由属性测试尚未授权实施。本批未接入设置 / 领域服务、Rust↔TS 桥接或跨页面状态，未实现 UI / E2E 命令，未声称跨窗口同步已通过。

## 10. 通用测试支持抽取实施记录

2026-11-12 在上述 macOS 工作区完成 07.10。新增 `usher-test-support` workspace 成员，`publish = false`，无外部依赖；`usher-state` 仅通过开发依赖使用它。原有状态契约和 JSON 场景保持不变，工具提取后只调整导入及唤醒计数读取。根 `tests/integration/` 与 `tests/e2e/` 添加目录说明；没有新增空测试，也未迁移无关 UI 用例。

| 实际命令 / 验证 | 结果 |
| --- | --- |
| `python3 scripts/test.py test-support` | 4 项通过：跨线程及两种 wake 方法、单次 Pending / waker 释放、非 Unpin Future 与返回值、Pending 立即失败 |
| `python3 scripts/test.py state` | 无默认 feature 20 项、watch 组合 35 项通过；覆盖和原 SM1 一致 |
| `cargo test --locked --workspace` | 140 项通过，较第 9 节新增工具单测 4 项 |
| `python3 -m unittest discover -s scripts/tests -q` | 89 项通过 |
| `cargo fmt --all --check` | 通过 |
| `cargo clippy --locked --workspace --all-targets --features usher-state/backend-watch -- -D warnings` | 通过，包含工具和真实 watch 后端 |
| `actionlint -color` | 通过；state 作业新增独立 test-support 验证步骤 |
| `cargo metadata --offline --no-deps --format-version 1` 与依赖树 | 工具 crate 无任何依赖，唯一消费方为 `usher-state` 的 dev-dependency；状态核心正常依赖为空，watch 仍仅 Tokio sync / pin-project-lite |

命令包装器从 `tests/config.json` 读取两个 suite，日志与 `test-support.json`、`state.json` 写入 `target/test-reports/`；工作区输出保存为 `workspace.log`。07.10 完成；UT1 的路由部分、UT2、TE1 / TE2 状态不变。GitHub CI 未运行，本批未修改运行时业务代码，也未执行原生 UI / E2E 验收。

## 11. SM2 设置应用集成实施记录

2026-11-12 在 macOS、Rust 1.98.1、Python 3.14.6、NSH 0.5.0 验证。`src/app_state.rs` 装配路径注册表和独立会话，`src/settings_service.rs` 串行调用原设置仓库并发布完整快照。应用的 Rust 公共契约经 `app_state` 导出，外部集成目标直接调用生产服务；没有复制生产模块到测试目标，也没有公开可变仓库。现有 24 个公开桥接函数签名及所有 DTO 定义 / 转换代码与接入前一致。

[`settings_state.rs`](../tests/integration/settings_state.rs) 的 11 项覆盖：初始化与无变化 / 非法命令、不可变快照和快慢消费者、多入口竞争与唯一初始化、写失败及同 revision 重试、目录与引擎会话隔离、旧格式重开、未来 schema 写保护、损坏备份、主题扫描与克隆重试、取消及关闭、背景图片持久化及回读。输入为人工构造的 TOML / JSON 和已有背景图片 fixture；每例独立临时目录，不访问用户设置。重开测试证明文件恢复，不冒充进程重启或原生多窗口实验。

另新增 6 项模块就近单测：慢初始化不持有目录表锁、慢事务不阻塞读者或其他目录、发布失败后关闭且保留已落盘文件、panic 后关闭等待者、后端创建失败、revision 耗尽前拒绝变更。发布故障 fake 仅在设置适配器测试模块；`usher-test-support` 不依赖业务，也未增加业务夹具。SM1 原序号耗尽测试补充了新 `preflight_publish` 接口的边界断言。

| 实际命令 / 验证 | 结果 |
| --- | --- |
| `python3 scripts/test.py integration` | 11 项通过；显式运行 `settings_state` Cargo 目标，检查必测组；报告 `target/test-reports/integration.json` |
| `cargo test --locked -p usher-nsh` | 应用 62 单测 + 11 集成通过，其中单测新增 6 项 |
| `python3 scripts/test.py state` | 无默认 feature 20 项、watch 35 项通过 |
| `cargo test --locked --workspace` | 172 项通过；相较上批 140 项，新增应用 17 项，另外因应用启用 watch，工作区多运行 15 项真实后端契约，不将它们记为新增测试 |
| `python3 -m unittest discover -s scripts/tests -q` | 89 项通过 |
| `cargo fmt --all --check`；`cargo clippy --locked --workspace --all-targets --features usher-state/backend-watch -- -D warnings` | 通过 |
| `nsh check` | TypeScript 检查及 Rust 应用构建通过 |
| `nsh test tests/ui/appearance.test.tsx` | 原有外观、主题与语言无头回归 2 项通过；保留真实旧请求—响应桥接，没有 modules mock；不覆盖新订阅桥接 |
| `actionlint -color`；`python3 scripts/release/shell_inputs.py check-embedded` | 通过；macOS rust 作业追加 integration 步骤 |

工作区及现有 UI 输出保存在 `target/test-reports/sm2-workspace.log`、`sm2-appearance.log`。SM2 / 03.09 完成；UT2 / 07.07 仅设置集成部分完成，路由、跨语言订阅 DTO、TS mock 和嵌套 TS 发现仍待办。SM3、SM4、TE1 / TE2 未实施；没有新增 JS 订阅，也未验收真实多窗口、HMR、进程重启或其他原生平台。远端 GitHub CI 尚未运行。


## 12. SM3 订阅桥接实施记录

2026-11-12 在 macOS、Rust 1.98.1、Python 3.14.6、NSH 0.5.0 完成。本批未新增外部依赖。实现前核对固定版本 `nsh-core` 的 CallCx、Callback、Stop、Local 和 task 源码，以及生成的 NSH context / cleanup 类型：回调仅在所属引擎执行；`call_async` 等待投递结果；Stop 绑定 owner / 模块 / VM 清理。context 使用 Provider，不能假定存在其他框架的 provide API。

`src/state_bridge.rs` 封装通用转发与取消，初始化取得原子订阅初值，每次等待 JS 回调确认后才读取下一快照，watch 在此期间保留最新值，最终值先于终止事件。Stop 唤醒并释放初始化、等待或回调确认中的工作，不关闭共享服务。会话服务在 Local 关闭时主动终止，持有旧 Arc 也不能继续写入。已经排入 NSH 的回调不能撤回，TS active 标志和绑定身份拒绝迟到输入。

`ui/state/contracts.ts`、`projection.ts` 分离消费者契约、传输和响应式实现。NSH 绑定集中在 runtime/nsh.ts，原生 Settings / SettingsState 调用集中在 transport.ts；主题数据契约移到 `ui/theme/contracts.ts`，状态类型不依赖生成的桥接 DTO。版本序号和 StoreId 使用十进制字符串，覆盖跨位数及超出 JS 安全整数；读回、命令完成后的重新读取与通知共用版本接收器。深度只读投影保留最终值，终止或协议错误关闭绑定；业务命令拒绝不自动关闭订阅。App owner 拥有连接 / 销毁权限，SettingsProvider 和 useSettingsStore 仅借出门面，原 appearance 属性接口继续可用。

| 测试位置 | 数据与覆盖 |
| --- | --- |
| `src/state_bridge/tests.rs` | 6 项确定性测试，复用 usher-test-support 的 poll / ready / WakeCounter；初值、慢回调合并、最终值与关闭、待决初始化 / 等待 / 回调取消、任务 drop、错误路径及捕获资源释放 |
| `src/settings_bridge.rs` 单测 | 1 项 Local 关闭测试；保留 Arc 时仍关闭会话、拒绝新写入和订阅 |
| `ui/state/__tests__/projection.test.ts` | 11 项；controlled transport、普通函数响应式替代器、可控 Promise 留在本模块 support/；乱序、重复、异步注册取消、旧实例 / 旧绑定、终止、重连、只读与精度 |
| `tests/integration/state-bridge.test.tsx` | 4 项真实桥接；本地 JSON 初值及测试页面，独立订阅、借用 selector、卸载 / 重挂载、注册即取消、引擎销毁；命令丢弃旧回执，消费方只能通过订阅更新 |
| `tests/integration/state_bridge.test.tsx` | 2 项真实桥接；生产 appearance 装配与原生 Stop 自动 owner 清理（测试组件没有 TS 手工清理）、订阅同 StoreId 重挂载及引擎重建身份隔离 |
| `scripts/tests/test_test_runner.py` | 增加 6 项发现 / 报告断言；嵌套 TS / TSX、排除生成文件、逐文件调用及跨文件必测组、缺失 / 旧 / 空 / 失败 / 跳过 / 不一致 JUnit 均不能误报通过 |

夹具均为本地确定性数据，无随机 seed。Rust 实例按例创建，TS 可控回调没有计时 sleep；NSH 页面使用无数据目录的内存会话，测试不改用户真实设置。无头 open 由测试运行时管理，销毁场景另外显式 dispose；两个引擎证明会话隔离，不代表同仓库原生多窗口同步。

| 实际命令 / 验证 | 结果 |
| --- | --- |
| `cargo test --locked --workspace` | 179 项通过；应用 69 单测 + 11 设置集成，包含本批 7 项 Rust 生命周期测试 |
| `python3 scripts/test.py state` | 无默认 feature 20 项、watch 35 项通过 |
| `python3 scripts/test.py test-support` | 4 项通过 |
| `python3 scripts/test.py integration` | 11 项通过 |
| `python3 scripts/test.py state-ui` | 3 个源文件，投影 11 项 + 真实桥接 6 项通过 |
| `python3 scripts/test.py ui` | 11 个既有源文件、29 项无头 UI 回归全部通过 |
| `python3 -m unittest discover scripts/tests -q` | 95 项通过 |
| `cargo fmt --all --check`、`cargo clippy --workspace --all-targets --locked -- -D warnings` | 通过 |
| `nsh check`、`actionlint -color`、`python3 scripts/release/shell_inputs.py check-embedded` | 通过 |

统一入口从 tests/config.json 发现测试，NSH 逐文件调用，输出新生成的 JUnit、日志和套件 JSON；缺失文件、零用例、失败或跳过、缺少必测组均失败。发现目录时排除 `.nsh`、`node_modules`、`target`。本轮先遇到目录参数被当作应用入口、并行 NSH 命令串用缓存，以及生成的截图步骤被发现三个问题，已分别改为源文件枚举、串行执行和排除生成目录，并完整重跑对应套件；不以单项补跑或忽略失败作为验收。CI ui 作业顺序运行 nsh check、state-ui、ui，报告位于 target/test-reports/；工作区完整输出另存 sm3-root-workspace.log。

SM3 / 03.10 完成，UT2 / 07.07 的设置、订阅、TS 投影与发现部分完成。SM4 的导航、分类、操作反馈、导出、共享预览和主页计数仍未迁移；动态诊断健康数据还没有独立发布源。TE1 / TE2 的原生隔离数据装配、同仓库多窗口、进程重启、真实 HMR 与其他平台未验收，组件重挂载不能替代 HMR；远端 GitHub CI 尚未运行。发布包体积与同条件构建成本留待 SM6。


## 13. RT1 至 RT3 与 SM4 实施记录

2026-11-12，macOS、Rust 1.98.1、NSH 0.5.0、Python 3.14.6。RT1 建立纯 usher-router 及可替换匹配器；继续 SM4 时按依赖顺序完成 RT2 / RT3，历史与 session 共用 Rust Store，现有页面全部接入。原设置桥接与持久化格式保持兼容；未进入 SM5 更新领域。

路由测试使用合成 catalog.json / cases.json、参考线性匹配器与 matchit；128 组属性输入、固定 seed 20261112 来自夹具。属性样本排除被静态路由占用的命名参数，静态遮蔽另有明确拒绝断言。两后端复用相同解析、目录拒绝、历史不变量断言；生成测试验证 reviewed 类型产物、无效 schema 和 CLI 对缺失 / 过期输出非零退出且不修改文件。通用夹具身份校验放 usher-test-support；业务 matcher、catalog 和 mock binding 留在所属测试目录。

| 验证命令 | 最终完整执行结果 |
| --- | --- |
| `python3 scripts/test.py router` | 无默认 feature 9 项、matchit 14 项通过 |
| `python3 scripts/test.py state` | 20 / 35 项通过 |
| `python3 scripts/test.py test-support` | 6 项通过 |
| `python3 scripts/test.py integration` | 11 设置 + 8 窗口路由通过 |
| `python3 scripts/test.py routes-check` | 3 生成器测试及实际 ui/pages 生成新鲜度检查通过 |
| `cargo test --locked --workspace` | 206 项通过，含应用 72 单测、19 集成及各 crate；feature 统一会启用真实后端 |
| `nsh check` | TS 类型与 Rust 应用构建通过，页面注册穷尽生成 PageKey |
| `python3 scripts/test.py state-ui` | 20 项：投影 11、设置桥接 6、路由桥接 3 |
| `python3 scripts/test.py router-ui` | 门面 mock 4、RouterView 错误注入 1、真实桥接 3 项通过 |
| `python3 scripts/test.py e2e --target headless` | 实际 App 2 项旅程通过 |
| `python3 scripts/test.py ui` | 11 个原有文件、29 项全部通过 |
| `python3 -m unittest discover -s scripts/tests -q` | 96 项通过，包含生成检查的退出码 / 证据校验 |
| `cargo fmt --all --check`；`cargo clippy --workspace --all-targets --locked -- -D warnings` | 通过 |
| `actionlint -color`；`python3 scripts/release/shell_inputs.py check-embedded`；`git diff --check` | 通过；新增 session 配置已列入 shell inputs |
| `cargo check -p usher-router --features matcher-matchit --target x86_64-pc-windows-msvc --target x86_64-unknown-linux-gnu --target aarch64-apple-ios --target aarch64-apple-ios-sim` | 四个纯 crate 目标通过；Android 目标未安装，其他原生应用未运行 |

完整最终轮日志为 target/test-reports/sm4-final-*.log，统一命令另有套件 JSON / JUnit；目标编译记录 rt1-targets.log。本机依赖缓存已准备，NSH 最终命令使用 CARGO_NET_OFFLINE=true 避免嵌套生成 manifest 的联网更新；CI 不强制离线。过程中类型错误、静态路由占用的属性样本和 NSH 生成缓存问题均已修正，最终按套件完整串行重跑，不把失败后的单例补跑当作验收。补充 PageRegistry 装配注入及 RouterView 渲染错误测试后，重新完整运行 nsh check、router-ui、state-ui、e2e 和 ui；Rust 实现未再变更。

8 项窗口集成覆盖目录完整注册与拒绝替换、独立服务隔离、取消 / 异常保持历史、真正可控守卫未来完成顺序、session 并发不丢失、重定向循环与上限、关闭、业务目录 / JSON wire。TS 门面额外验证守卫完成时传输关闭仍返回 failed，不产生二次未处理拒绝。操作 ticket 拒绝迟到结果；预览 clear 核对保存时的 ticket，不能清除较新的预览；导出活动期间拒绝重复执行。复用 test-support 的 poll / ready，未依赖 sleep。

动态诊断增加 3 项 Rust 测试：无设置命令时健康 / dropped 直接发布，替换 source 后旧发布者无效，真实日志工作线程遇到不可写目录将错误送入订阅。数据目录使用独立临时对象，未向真实用户日志注入故障。

| 计划旅程 | 本批实际证据 | 保留缺口 |
| --- | --- | --- |
| E2E-01 导航 | 实际 App 菜单、历史按钮、快捷键与前进分支截断；现有 Shell 面包屑回归 | 原生宿主输入 / 返回未在本批重验 |
| E2E-02 跨页数据 | App 计数及设置分类往返保留；原有外观 / 语言 / 主题回归通过 | 同仓库真实多窗口偏好同步 |
| E2E-03 预览 / 导出 | RouterProbe 真实桥接切页保留预览；App 导出错误结果往返保留；Rust 验证执行票据 | 实际文件导出长任务跨页与关闭的原生旅程 |
| E2E-04 隔离 | 两个独立 open 引擎不共享导航、计数、分类 | 不是同仓库的两扇原生窗口；共享偏好及原生重开未验收 |
| E2E-05 守卫 / 错误 | 可控门面 mock、Rust 服务、真实桥接晚回调 / NotFound / back；无效名称拒绝；注入页面渲染错误验证重试不改 entryKey、持续失败可返回 | 原生手势守卫待 RT4 |
| E2E-06 重启 | 保留 SM2 仓库重开 / 文件兼容测试 | 真正进程重启及原生隔离数据目录装配未实施 |
| E2E-07 生命周期 | WindowLifecycle 生产装配同 VM 卸载 / 重挂载保留历史、entryKey、预览和计数；旧 SM3 清理契约仍通过 | 真实 HMR、目录变更迁移、宿主窗口身份；相同目录重复注册可用，修改内容暂拒绝 |

RT1 至 RT3 和 UT1 完成；SM4 标为部分完成，代码迁移 / 无头通过而原生多窗口及 HMR 尚缺证据。UT2 / TE1 / TE2 的剩余覆盖继续跟踪，不开放未隔离的 macos E2E target。CI 已配置 router / routes-check、窗口集成、router-ui 和无头 E2E，GitHub 远端尚未运行。包体积、发布构建成本及真正平台旅程留待 RT4 / SM6。


## 14. SM4 原生窗口与 HMR 验收

核对 NSH 0.5.0 本机固定版本源码：nsh-macos-host 支持 --data-dir，同时将缓存放在其 cache 子目录；nsh/desktop 的多个 Window 可共用一个 VM。原每 VM 单一窗口 Store 因此不能代表真实窗口隔离，本批改为 VM 内按宿主窗口注册，调用持有 Rust 签发的 StoreId；关闭按 StoreId 删除，旧句柄不能操作重开窗口。窗口身份只由 TS 传输适配器在渲染时捕获 currentWindow，不由业务页面传入，也不随焦点变化。

原生验收使用 tests/e2e/native 的专用装配、实际生产投影 / RouterView、两个 AppKit 窗口及官方 nsh dev 的源码监听 / HMR。scripts/native_acceptance.py 创建一次性源码副本和数据根，给宿主显式传 --data-dir；不改 HOME、正式配置、工作区业务源码或框架源码。设置、日志、主题、导出及缓存均受宿主数据根约束。通过修改副本的 Probe 组件触发真实 HMR，组件卸载或手工重挂载不计作 HMR。测试条件用 requestAnimationFrame 轮询可观察状态并有截止时间；Python 只编排构建、宿主、源码修改和报告，不实现第二套路由 / Store。


2026-11-12 在 macOS 原生 AppKit 开发宿主完成验收，NSH 0.5.0、Rust 1.98.1、Python 3.14.6。本轮没有添加依赖或修改框架。Rust 回归新增同 VM 两窗口隔离、关闭及旧 StoreId 不能关闭重开窗口的测试，复用真实 RouterService / 目录。

| 原生阶段 | 实际断言与证据 |
| --- | --- |
| 窗口隔离与偏好共享 | 两个宿主 WindowRef 均报告 visible；主窗口变更计数、路由、分类和预览，副窗口仍为初始会话；仅主窗口发设置命令，副窗口通过真实订阅收到相同偏好；Rust 保存标志为真 |
| 真实 HMR | 修改临时 Probe.tsx，官方 dev server 产生 build 2，宿主日志记录 hot update applied；两个 Rust StoreId 保留为 1 / 2，entryKey / 计数 / 分类 / 预览保留；两个旧 owner 清理，旧动作拒绝、旧守卫返回 cancelled；HMR 后再改变设置，新投影同步，旧设置投影保持 disposed 且不再更新 |
| 关闭重开 | 先卸载副窗口全部内容 owner，再以实际 WindowRef.close 关闭窗口并终止对应 Rust Store；旧 StoreId 读取拒绝；重新打开同名窗口得到新 StoreId 5、默认历史 / 计数，主窗口数据保持，已保存偏好可见 |
| 磁盘隔离 | 宿主参数明确 --data-dir 指向本轮临时树；核对 <data>/<acceptance-app-id>/development/settings.toml 的 appearance.colorMode，并记录该目录的日志文件；检查无符号链接 / 目录逃逸；结束时停止本轮宿主 / watcher 并删除源码副本和数据树 |

原生测试使用生产状态、路由及 RouterView 的专用装配，语义动作由探针调用，未伪造桥接或状态。它验证真实 AppKit 窗口和 HMR 生命周期，不宣称覆盖真实鼠标 / 键盘点击的全部产品旅程、标题栏关闭按钮、进程重启或发布产物；实际 App 用户路径继续由 headless E2E 和现有 UI 回归验证。窗口关闭由 WindowRef.close 发起，主窗口关闭的 VM 清理由既有桥接测试覆盖。

| 命令 | 本轮结果 |
| --- | --- |
| `python3 scripts/test.py e2e --target macos` | 3 项通过；报告 e2e-macos.json、native-state.xml 与 native-isolation.json |
| `cargo test --locked --workspace` | 207 项通过 |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | 通过 |
| `nsh check` | 应用和临时验收源码类型 / Rust 构建通过 |
| `python3 -m unittest discover -s scripts/tests -q` | 97 项通过，包含临时源码 / 清单隔离及禁止复用旧目录 |
| `state-ui`、`router-ui`、`e2e --target headless`、`ui`、`integration`（test.py） | 窗口监听修正后完整串行回归，分别 20 / 8 / 2 / 29 / 19 项通过 |
| `cargo fmt --all --check`、`actionlint -color`、嵌入文件检查、`git diff --check` | 通过 |

每轮日志保留在 target/test-reports/native-run-*/；native-isolation.json 指向最终轮及已清理的临时数据根，普通套件输出为 sm4-native-*.log。本机通过 CARGO_NET_OFFLINE=true 使用已有缓存，命令本身不强制离线。首轮准备阶段遇到沙箱禁止本地监听、临时副本缺少声明 / 入口 / tsconfig、以及测试异步调用 currentWindow 的错误；分别在获准的原生执行环境运行、完整准备副本，并在渲染时捕获 WindowRef 后解决。HMR 后旧订阅不再更新的断言通过后，增加“内容先卸载再关闭”用例，实际捕获 owner 绑定监听提前销毁导致的 Store 泄漏（native-run-4898_6ap）。修正为 Rust 独占领取、窗口生命周期持有的监听；最终完整原生套件再次通过（native-run-wpb6wlvz），失败和修复后的日志均保留。没有把组件重挂载或单项重试当作 HMR 成功。

SM4 / 03.11 在当前支持的 macOS 开发宿主范围标为完成。TE1 / TE2 保持部分完成：跨进程重启、其他原生平台、长导出真实文件旅程及发布产物仍待对应批次；RT4 的目录变更迁移、深链接和原生返回不因此完成。原生 CI 需要预先配置专用 self-hosted 图形 runner；本轮只更新配置，未触发 GitHub 工作流。

## 15. SM5 更新应用服务实施记录

2026-11-12 至 13，在保留旧 usher-update API 和所有 Settings 接口 / TOML 格式的前提下，新增 src/update_service。应用服务通过注入 BackendFactory 使用通用快照与订阅，UpdateRegistry 以宿主设置 StoreId 共享实例。没有新增依赖，没有启动更新网络请求、安装、后台定时器或新设置字段；更新方案批次 3 / 4 / 5 的产品改造继续单独推进。

| 验证 | 本轮结果 |
| --- | --- |
| `python3 scripts/test.py update-state` | 原更新 crate 38 项、应用完整单测 75 项（含新 2 项）、更新应用集成 15 项通过 |
| `cargo test --locked --workspace` | 224 项通过；这是各 crate、应用单测和集成的合计，不与上行重复相加 |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | 通过 |
| 脚本 unittest | 97 项通过 |

15 项集成中 10 项用完全相同的消费方分别运行 Memory / watch 后端，覆盖共享订阅、重复检查并入、独占租约与领取、旧进度 / 回执 / 恢复查询失效、下载进度约束、效果种类校验、下载 / 校验 / 安装顺序、静默失败、取消队列、关闭、draining 和 recovery_required。其余用例验证并发领取只有一个成功、发布 / 交接失败保持旧模型及队列、同目录共享 / 不同目录隔离、注册表销毁及文件字节兼容。更新单测覆盖身份耗尽和最后拥有者 drop。

业务 mock 数据由 tests/integration/fixtures/updates.json 提供；被测规则是真实 Machine，测试直接控制完成先后并用 Barrier 触发竞争，不访问更新服务器。通用 ready 和 fixture 身份校验复用 usher-test-support；状态参考后端仍由 usher-state 测试维护。文件测试用 tempfile 的独立目录，更新活动前后 settings.toml 字节相同。并发消费者代表应用服务入口，并非真实 AppKit 更新窗口 / NSH 下载验收。

核对本机固定 NSH 0.5.0 的 runtime/updater.rs：下载 Promise 内含校验、使用内部串行 operation，JS 公共接口无取消或通用在途任务查询。因此 revoke 不代表物理操作停止；只有原 Promise 已结束 / 任务确已停止才能 settle，安装或结果未知必须用同一 Claim 做可信宿主 reconcile。此处只交付约束，不自动伪造停止确认，也不宣称真实更新安装已验收。后续消息域接入契约见状态管理方案第 14 节。

首次配置 update-state 时，重复运行 ID 被统一入口拒绝；随后过滤式 lib 测试因 73 filtered out 被完整套件校验拒绝。已分别使用唯一 ID 和完整应用 lib 单测，最终完整重跑 update-state 全部通过；没有放宽非零 / 未过滤的验收规则。日志为 sm5-update-state.log、sm5-workspace.log、sm5-clippy.log，初次过滤失败另存 sm5-update-state-initial.log。

## 16. SM6 审核与最终回归

共享状态来源审核、局部状态例外、依赖与后端替换、平台和构建成本见 [验收审计](state-audit.md)。2026-11-12 至 13 完成以下本机验证：

| 命令 / 项目 | 结果 |
| --- | --- |
| `update-state` | 更新 crate 38 + 应用 lib 75 + 更新集成 15，通过 |
| `integration` | 设置 11 + 窗口 / 路由 8 + 更新 15 = 34，通过 |
| `state`、`router`、`test-support` | 20 / 35、9 / 14、6 项通过 |
| `routes-check` | 3 测试及生成新鲜度检查通过 |
| `state-ui`、`router-ui`、`ui`、`e2e --target headless` | 20、8、29、2 项，全部串行通过 |
| `e2e --target macos` | 3 项真实 AppKit / HMR 通过；native-run-mhvfodk9，临时源码 / 数据已删除 |
| `state-targets` | 七目标 × 四库的 28 组产物核验通过，编译结果不计入单测数 |
| 应用 crate Apple 四目标 check | macOS ARM / Intel、iOS device / simulator 通过 |
| 完整应用七目标 check | 未通过：Linux C 交叉编译器、Windows SDK 头文件缺失；不计入通过 |
| Rust workspace / Clippy / fmt | 224 项及检查通过 |
| Python unittest | 99 项通过；新增 2 项矩阵证据完整性测试 |
| nsh check / actionlint / embedded inputs / diff check | 通过 |
| `state_cost.py --baseline 06ed67e` | 两次隔离 standard / arm64 发布构建通过，应用文件总量 +261,882 字节（+1.10%），墙钟 328.96 / 334.17 秒；限制见审计 |

命令均在仓库根执行；本机 NSH / Cargo 使用 CARGO_NET_OFFLINE=true 的已有缓存，CI 不强制离线。日志 sm6-*.log、对应 suite JSON / JUnit 及 state-target-artifacts.jsonl 保留原始证据。完整套件执行不以单例补跑或跳过替代；平台检查初次因摘要标记和 Cargo 简写 package_id 的报告解析被拒，已改为读取 manifest_path 的包名和实际库产物、核对全部目标组合后完整重跑。两个初次报告保留 sm6-state-targets-initial.log / sm6-state-targets-json-initial.log，未放宽“缺产物即失败”的条件。

CI 已纳入更新应用集成和纯库平台矩阵，并保留契约 / 编译报告；本轮未推送或触发远端工作流。SM6 / 03.13 整体仍部分完成，剩余完整平台工具链、其他宿主、远端 CI 及发布运行 / 升级验收见审计第 6 节。

## 17. RT4 目录迁移、深链接与原生返回（2026-11-13）

新增规则测试就近放在 `usher-router/tests/contracts.rs`、`src/router_bridge.rs` 和 `ui/router/__tests__/host.test.ts`；跨模块验证归 `tests/integration/state_router.rs` / `router.test.tsx`。`usher-test-support::task` 验证迁移发布通知，参考 matcher 与 matchit 复用相同迁移契约；覆盖失效路径、收紧 query、前进历史、entry key、异常后端与溢出、同版本冲突、淘汰版本、旧绑定和迟到守卫。

深链接 fixture 使用已确认的公开地址，覆盖启动队列、非许可 scheme/host、凭据/端口/片段、未二次解码的原始路径、异步守卫、取消与销毁。`tests/e2e/native-back.test.tsx` 同时通过无头及 AppKit 实际输入驱动，验证浮层优先、取消不移动、放行后移动和根节点不改历史。NSH 官方 `nsh test --target macos` 给每个 app.open 创建独立数据/缓存根（host 的 `--data-dir`），不读取真实用户设置。该用例接入现有 `e2e-macos` 配置，与原生 HMR 串行执行。

现有隔离 HMR fixture 现在随源码 generation 更新目录 identity，并断言 StoreId、entry key、会话保留和新目录生效；旧 projection、守卫与关闭令牌不能操作新绑定。使用原生报告 `native-isolation.json` 确认临时数据树已删除，日志位于 `target/test-reports/native-run-kb0wdknr`。

| 仓库根目录命令 | 本轮结果 |
| --- | --- |
| `cargo test --locked --workspace` | 229 项通过 |
| `python3 scripts/test.py router` | 无默认后端 10 项、matchit 16 项通过 |
| `python3 scripts/test.py integration` | 36 项通过（设置11、路由10、更新15） |
| `python3 scripts/test.py router-ui` | 11 项通过 |
| `python3 scripts/test.py state-ui` | 21 项通过，包含本批新增桥接用例 |
| `python3 scripts/test.py e2e --target headless` | 4 项通过 |
| `python3 scripts/test.py e2e --target macos` | 4 项通过（HMR/窗口3、原生返回1） |
| `python3 scripts/test.py ui` | 29 项通过 |
| `python3 scripts/test.py routes-check` | 3 项测试及生成产物一致性检查通过 |
| `python3 -m unittest discover -s scripts/tests` | 99 项通过 |
| `nsh check`、`cargo clippy --locked --workspace --all-targets -- -D warnings`、`cargo fmt --all -- --check` | 通过 |

第 17 节初次验收时尚缺系统级 URL scheme 注册、外部进程启动应用和完整 VM reload，现已由第 18、19 节补齐；跨进程持久状态恢复、其他平台真实宿主、发布包升级与 iOS 交互式返回仍未验收。RT4 保持部分完成；SM1–SM5 完成，SM6 的跨平台完整应用构建、其他宿主、发布运行与远端 CI 仍有缺口，见 [状态审计](state-audit.md)。

## 18. macOS 发布包 URL 验收（2026-11-13）

新增 `python3 scripts/test.py packaged-routing`：生成真实 ad-hoc release `.app`，检查 Info.plist、代码签名和 LaunchServices 注册，通过外部 open 命令验证冷 URL 启动及同进程唤醒。UI 源码使用临时副本并添加只读观察器，Rust 源码只读复用当前 checkout；唯一测试应用 ID 的 data/cache/saved-state 命名空间链接到临时目录，不修改 HOME。停止应用后撤销注册并移除链接；无法停止时保留隔离环境并失败，不把运行中的应用切回系统数据目录。

工具单测在 `scripts/tests/test_packaged_routing.py`，覆盖副本不改产品源码、拒绝覆盖既有或悬空路径、只删除拥有的链接、AppKit 已删除空链接的幂等清理、部分日志不能当证据、未解锁会话须拒绝。真实主机辅助工具位于 `tests/e2e/packaged/host.swift`；测试开始及启动前读取 CGSession 和前台应用，避免把锁屏误判为产品故障。

解锁后已完整执行通过 3 项，证据为 `target/test-reports/packaged-routing-jhaxier9/report.json` 和 packaged-routing.json。会话 ready=true；system-default 分派冷 URL 到设置页；正常启动后的隐藏应用收到 URL，PID 保持一致且 hidden=false、active=true；显式应用 URL 激活亦通过。所有测试目录链接与数据根检查无残留，handler 清理后恢复为 null。此前失败/中止运行不累计为通过。

已将该命令接入手工 native_state CI 作业，远端尚未运行。Developer ID 公证、Gatekeeper 下载链路和实际升级仍不在这项本地 ad-hoc 验收的完成范围。

不依赖解锁的本轮检查已通过：`nsh check`、router-ui 11 项、无头 E2E 4 项、Python 脚本 104 项、actionlint 和 git diff --check。已逐一核对本轮所有报告中的临时数据根与专属链接，剩余路径为 0。未修改生产 Rust 状态、桥接及持久化格式。

## 19. 完整 VM reload 与本机续验（2026-11-13）

`python3 scripts/test.py native-reload` 完整通过 1 项。`tests/e2e/reload.test.tsx` 用官方 macOS 原生输入发送 Cmd+R，验证新的 VM / StoreId 恢复持久 dark 偏好，计数、分类、预览、历史和 pending 重置，旧作用域守卫回调不能写入。fixture 在 `tests/e2e/fixtures/NativeReload.tsx`，共用生产 store / router，检查点使用原生 runner 隔离数据根内的 Storage。报告为 native-reload.json、native-vm-reload-0.xml / log；没有用组件卸载代替完整 VM 替换。

首次将 fixture 放在 native/ 下时，NSH 按最近的 App.tsx 识别了另一项目，因缺少根项目 Rust 装配而失败。已迁至上述目录，随后完整重跑通过；首次失败不计通过。headless 套件显式列出 navigation.test.tsx 和 native-back.test.tsx，macOS 专用 reload 独立进入手工 native_state CI。

本轮后续复核：Rust workspace 229 项、Python unittest 104 项、调整发现范围后的完整无头 E2E 4 项、nsh check 和 actionlint 通过；state-targets 的四库七目标共 28 组实际产物通过，应用库 Apple 四目标 check 通过，四个 libusher_nsh 元数据文件存在。证据为 sm6-local-workspace.log、rt4-local-python.log、rt4-local-nsh-check.log、rt4-local-actionlint.log、sm6-local-targets.log、sm6-local-apple.log；这些是本机结果，远端 CI 未运行。

本机 iOS 前置检查失败：Xcode simctl 包装器要求 CoreSimulator 1174.21，已安装 1174.9.2；xcodebuild -checkFirstLaunchStatus 返回 69。只读清单可用，不能视为宿主可运行。已结束本任务的首次启动查询，没有安装系统组件、创建设备或改动已有模拟器；环境就绪后须创建专用设备。用户暂无 Android / Windows / Linux 设备或 CI runner，后续按环境可用性继续，不将缺环境算作通过。

上一轮 scripts/update-sim/sim.py 仍改写 HOME，不符合隔离要求，故未执行（本轮改造及结果见第 20 节）；它的历史 4 项启动模拟证据不能视为新代码的升级回归。续验前须改用唯一测试应用 ID 与专属临时数据命名空间并验证清理，再运行已有有效场景。检查 / 下载 / 安装链路、整包真实归档和分发签名仍按更新方案推进，不能用预置 next 包的启动测试替代完整升级旅程。

## 20. SM6 本机更新模拟隔离改造（2026-11-13）

`scripts/update-sim/sim.sh run --all` 已改为每轮唯一测试应用 ID 和一次性签名密钥；旧壳与新代码包身份一致，测试源码副本移除公开 URL scheme，产品源文件和持久化格式不变。Rust 源码只读复用 checkout；旧壳及代码包构建共享 target/update-sim-build 的编译缓存，运行数据不共享。每个场景有独立 data/cache/saved-state，通过系统 API 返回位置的专属测试 ID 链接访问，不改 HOME。辅助链接保护逻辑从发布路由脚本提取至 scripts/macos_test_data.py，两类验收复用。

新增 16 项单测，全部脚本共 120 项通过：真实子进程继承 HOME 和退出回收、提前退出拒绝、已有目录 / 悬空链接保护、部分创建失败、原有临时根保护、存活进程 / 查询失败保留隔离、路径归属变化、AppKit 移除空 saved-state、应用 ID / scheme 校验、源码副本完整性、构建失败清理、停止失败保留、serve 中断、并发 clean 锁、空 / pending 套件不得通过。单测位于 scripts/tests/test_update_sim.py；证据为 target/test-reports/sm6-update-isolation-unit.log。actionlint、shellcheck 和 diff 检查通过，命令已接入手工 native_state CI，远端未执行。

`serve` 由脚本启动并拥有应用进程，Ctrl+C 后停止、回收并清理，记录 observed，不计自动通过。运行命令使用仓库级独占锁，防止并发模拟 / clean；停止失败、归属变化或清理未完成时保留整个隔离树，返回失败。正常结束删除源码、应用、一次性密钥与数据，日志和 JSON 在 target/test-reports/update-sim-*/ 保留；clean 只移除旧 .nsh/update-sim 缓存，不删除报告或保留的隔离树。

真实 SIGTERM 中断运行记录在 update-sim-4_uik8v1/report.json：总结果 failed，downgrade 被中断，不计为整套通过；清理错误为空，所有专属链接及临时树实际检查无残留。该轮还发现代码包发布未共享旧壳 Cargo 缓存，已统一编译缓存，最终完整重跑结果如下。

最终验收为 `target/test-reports/update-sim-d8uds6dq/report.json`，整轮状态 passed：

| 有效场景 | 实际结果 |
| --- | --- |
| crash-on-start | 两次真实启动后回滚，next / trying / current 均不存在，日志确认未完成健康试运行 |
| downgrade | 拒绝非递增构建号，目录无 current / trying，日志确认降级拒绝 |
| package-update | 健康试运行后 current.nshpkg 与 trust 存在，next / trying / partial 无残留 |
| tampered-package | 拒绝被篡改签名，current / trying 不存在，日志确认签名错误 |

合计 4 passed、0 failed、7 pending，未把中断诊断轮或人工 observed 累计为通过。最终命令日志为 sm6-update-simulation-final.log；每例保留 app.log、状态断言、隔离路径和 cleanup_errors=[]。独立复查报告 cleanup-audit.json 核对 16 个唯一隔离路径，全部不存在；临时源码、应用及一次性密钥所在整树已删除。真实 mock HTTP 源随每例关闭，子进程在解绑路径前回收。该结果验证本机 Apple Silicon 的预置代码包启动，不等同于检查 / 下载、整包实际替换、用户升级迁移或分发验收。SM6 仍为部分完成。

## 更新批次 3 / 4：A 初始化与真实桥接（2026-11-13）

本机 macOS arm64：`cargo test --locked --workspace` 214 项通过（含 20 项独立日志测试、29 项更新状态机契约、17 项更新应用集成）；`cargo clippy --locked --workspace --all-targets -- -D warnings` 与 `nsh check` 通过。`python3 scripts/test.py state-ui` 22 项通过，报告为 `target/test-reports/state-ui.json`；其中新增 `tests/integration/update_bridge.test.tsx` 通过真实 NSH 桥接验证未配置、重复配置拒绝、单执行租约、回执去重、进度校验与订阅释放后的服务保留。最终针对更新桥接的复验报告为 `target/test-reports/update-bridge-final.xml`。

日志核心独立后由工作区测试进入现有 CI；真实桥接测试纳入 state-ui 的必需前缀。设置测试覆盖 auto_check 默认开启、旧文件缺省兼容、默认段省略、落盘重开、未保存状态即时阻止后台检查与重试保存。配置发布失败不创建半配置服务，两个后端共享既有执行契约。

本轮无网络 / 安装执行器，原生结果由测试可控回报。未重跑真实更新模拟、原生多窗口 / VM reload、跨平台编译或发布构建成本，原记录仅代表此前版本。销毁执行 VM 后无租约交接，需重启应用；不能把普通订阅释放测试计作更新执行器的 HMR 验收。

## 更新批次 5：控制器与产品界面（2026-11-13）

用户确认的生产参数为启动延迟 30 秒、周期 6 小时、同版本失败阈值 2 次，统一来自 ui/config/updates.json。A 初始化及 auto_check 默认开启保持不变。

| 验证 | 本机 macOS arm64 结果 |
| --- | --- |
| cargo test --locked --workspace | 216 项通过；新增候选 canInstall 降级及共享检查元数据契约 |
| cargo clippy --locked --workspace --all-targets -- -D warnings；nsh check | 通过 |
| python3 scripts/test.py state-ui | 38 项通过，含 11 项更新控制器 / 投影、5 项更新界面旅程；报告 target/test-reports/state-ui.json |
| python3 scripts/test.py ui | 29 项通过；更新了开发环境提示及设置第四分区的键盘导航预期 |
| python3 scripts/test.py e2e --target headless | 4 项通过 |
| python3 -m unittest discover -s scripts/tests | 120 项通过，含 16 项模拟器隔离与清理契约 |

控制器用确定性时钟和原生适配器覆盖串行检查、包与整包版本不一致拒绝、信任失败、Unsupported 配置、自动开关与周期、卸载 / 重挂时不重做下载、延迟订阅释放、迟到结束回调隔离、进度先于完成回执、安装失败、完成传输失败不重放，以及空领取与新工作并发时保留唤醒。

`tests/integration/updates_feature.test.tsx` 使用真实 Rust UpdateState / 设置桥接和生产组件，只有 NSH 更新 I/O 可控；覆盖整包确认、代码包关闭自动后需确认、进度 / 未知总量、关闭浮层、只读设置禁止重启、手动升级、失败重试、跨分区保留与窄窗。新目录及用例前缀已注册 state-ui，沿现有 CI 执行。

隔离 macOS 开发宿主使用临时源码、应用包装和 --data-dir，未借用真实用户设置。人工检查 Canvas 环、禁用但不淡出的进度按钮、锚定浮层、就绪勾选、安装失败反馈及 Progress 填充；在下载中热更新 UpdatesSettings 组件后状态与原生调用次数保留。该项使用可控原生 I/O，不计作真实下载，也不证明整个执行 VM 重建可恢复。完整 VM 销毁仍无执行者交接，须重启应用。

本地模拟 12 场景通过，报告 target/test-reports/update-sim-d5xkzs26/report.json；中断下载夹具修正后的离线单项复验通过，报告 target/test-reports/update-sim-fozvm6x9/report.json。一次复验在构建阶段因 crates.io DNS 失败，未计入通过。真实本地源的代码包检查、签名下载、再启动晋升及清理细节见 docs/update.md 本轮验收记录。该模拟器的整包归档仍为占位文件；真实整包替换由下节批次 6 独立命令补验，正式 GitHub 发布和其余平台仍待验收。本轮未重测跨平台编译、发布成本或触发远端 CI。

## 更新批次 6：真实整包归档替换（2026-11-13）

`python3 scripts/test.py shell-update` 在解锁的 macOS arm64 桌面执行，脚本为 `scripts/shell_update_acceptance.py`，测试驱动和场景配置在 `tests/e2e/shell-update/`。与占位归档的发现测试分开统计：旧版 0.1.0 / build 1、新版 0.2.0 / build 3 均为实际构建的 standard 应用；归档保留真实签名、权限和符号链接，update-manifest 生成一次性签名清单。临时 App 调用生产门面，控制器、Rust 桥接、下载校验、安装器及重启均未替换。

宿主逐文件比较安装后的 SHA-256 / 权限 / 链接，包含仅旧版有的资源被移除；从新进程的 shellInfo 和 Settings 快照验证运行版本、暗色 / 英文 / 关闭自动检查的测试偏好与已保存状态，同时验证设置文件字节未变。故障场景核对旧版不变且可重开，原生诊断必须匹配注入故障，产品失败分类必须符合文档。恢复正确归档或目录权限后重新执行正常更新动作。

每轮使用唯一测试 ID，复用 AppDataNamespace 的隔离 data/cache/saved-state 链接，不改 HOME。旧进程、NSH 分离出的重启辅助进程和新进程都纳入清理；确认静默后注销测试路径的 LaunchServices 记录，再解绑数据。与本地模拟器共用命令锁。清理无法完成时保留隔离树并判失败；在进入场景前登记清理责任，避免清理期间中断被误判为无活动应用。单测覆盖实际归档、文件清单、会话隔离、延迟重启进程、无法停止、并发锁、启动前失败和清理期间中断；中断保护此轮按脚本单测验证，不计作原生进程中断验收。

本轮控制器修复 NSH 的 `update install: the archive holds …, this app is …` 被通用 install 匹配误判为 Disk 的问题，应用身份不符现在归入 Rejected。`state-ui` 39 项通过（更新控制器 12 项、更新界面旅程 5 项），脚本测试 135 项通过；`nsh check`、actionlint 和 git diff --check 通过。Rust 领域规则和持久化格式本轮未修改，工作区测试结果沿用批次 5，未作为本轮新执行计数。

该命令已注册 tests/config.json 的 5 个必需用例前缀和驱动来源，并加入手动 native_state CI；远端 CI 未触发。报告保留 JSON、JUnit、应用 / 原生诊断日志和构建命令。测试不覆盖 Gatekeeper 下载隔离标记、Apple 公证、真实 GitHub 分发、/Applications 权限组合、两次 rename 之间的崩溃或 AppKit 变体转换。

最终原生完整运行 **5 项通过、0 失败、0 跳过**：success、archive-tampered、wrong-identity、invalid-signature、install-denied。报告 `target/test-reports/shell-update-yft7wbty/report.json` 与统一入口 `target/test-reports/shell-update.json` 均 passed，JUnit 来源和必需用例组齐全。独立 cleanup-audit.json 核对 19 个唯一隔离路径及运行进程，无残留。

两次诊断轮不计入通过：shell-update-nywzbldb 的 JUnit 来源不匹配且发现身份错误误分类；shell-update-zpm_bsmy 的 SHA-256 日志断言和一次清理权限错误导致失败，保留目录经核对后安全回收（cleanup-recovery.json）。最终运行重新构建了全部产物并执行全部 5 项。持续清理权限拒绝会保留隔离；瞬时拒绝也必须经过后续完整静默确认才可清理，相关单测同时覆盖两种情况。


## 23. 更新界面整改（2026-11-13）

自动代码包下载现在显示边缘进度环，不自动弹出面板；新增可控慢下载旅程，验证下载按钮禁用、完成后显示重启 / 稍后、隐藏不可用检查按钮，以及关闭面板保留就绪状态。状态、说明与操作分行，英文长说明约束宽度并换行；强调图标使用默认 13pt 的 size-update-icon 主题 token。

本轮 `state-ui` 40 项、`ui` 29 项、`cargo test --locked --workspace` 216 项通过，`nsh check` 与差异格式检查通过。隔离 macOS 原生宿主人工核对了 25% 下载环、最终箭头尺寸、设置页布局及弹层长说明换行 / 并排操作；使用可控原生 I/O，不计作真实安装验收。最终预览记录在 target/test-reports/update-layout-4c34n_gr，cleanup.json 确认临时源码、应用与数据树已删除。预览重建过程中一次网络解析失败、一次沙箱监听失败不计入验收通过。首次 UI 回归与 state-ui 并行时争用 NSH 测试生成产物，导致 appearance 用例错误加载更新夹具；最终结果采用单独串行完整重跑，失败轮不计入通过。

后续图标与关闭入口调整：按系统图标优先的要求，macOS 改用 SF Symbols 的 square.and.arrow.down（size-update-icon 默认 14pt）；Popover 标题行使用组件库 View / Button 与现有 SystemIcon、iconButtonLook，右上角 × 替代底部关闭 / 稍后按钮。更新界面 6 项回归通过，覆盖错误与就绪状态的标题关闭入口、关闭后保留状态；nsh check 通过。隔离原生宿主人工确认系统符号显示、英文错误换行及 × 关闭操作，记录位于 target/test-reports/update-layout-parf4ohn；临时树已清理。本次未重跑整包安装或完整工作区测试。

## 状态与路由审查整改（2026-11-13）

删除闲置 StateMachine trait 与 StoreWriter::preflight_publish，保留 Transition / apply；设置服务自行检查容量并保持既有保存顺序。WindowService 负责窗口提交，window_session 独立维护页面会话规则；回执、票据及服务错误类型化，桥接保留旧入口和错误码。Router 按阶段验证目录与目标，预建 ID 索引及祖先链；生成强类型元数据，拆分 TS 守卫流程并区分非法目标与服务故障。

新增回归覆盖两个 matcher 的乱序目录及父级环、无效匹配结果、特殊元数据键、窗口发布失败后的重试、守卫 / 目录迁移原子性、票据精度与规范编码、旧新桥接回执兼容，以及无守卫导航和启动守卫时的错误分类。业务 mock 继续留在所属测试模块；usher-test-support 无需改动。

本机 macOS arm64 的最终验证：

| 命令 | 结果 |
| --- | --- |
| cargo test --locked --workspace | 224 项通过，含 2 项编译失败 doctest；设置 / 窗口 / 更新集成为 11 / 13 / 17 项 |
| python3 scripts/test.py state | 无默认后端 20 项、Watch 后端 35 项通过，分别包含 doctest |
| python3 scripts/test.py router | 无默认 matcher 12 项、matchit 19 项通过 |
| python3 scripts/test.py routes-check | 4 项生成测试及真实目录新鲜度检查通过 |
| cargo clippy --locked --workspace --all-targets -- -D warnings | 通过 |
| cargo fmt --all --check；nsh check；git diff --check | 通过 |
| python3 scripts/test.py router-ui | 13 项通过 |
| python3 scripts/test.py state-ui | 41 项通过 |
| python3 scripts/test.py ui | 29 项通过 |
| python3 scripts/test.py e2e --target headless | 4 项通过 |

共享工作区运行期间另有更新界面源码修改，state-ui 的更新页面用例曾超时，该轮不计通过。最终把当时完整源码冻结到独立临时目录，以独立 .nsh 生成目录串行执行 nsh check、state-ui、ui 与无头 E2E。隔离首轮因 crates.io DNS 失败未完成；使用已缓存依赖并设置 CARGO_NET_OFFLINE=true 后，四项完整通过。记录位于 target/test-reports/refactor-isolated-nhz_50q3/：results.json、suites/、workspace-*.log、inputs.json 和 source.tar.gz 分别保存结果、JUnit / 套件报告、工作区检查日志及冻结源码证据。更新界面改动保留并参与该快照验证，不计作本次状态 / 路由实现。

索引改动使用相同 release 配置比较当前实现与提交 39f517a 的路由源码：opt-level=z、LTO=fat、codegen-units=1、strip=symbols、panic=unwind。输入为实际两条路由目录，以及由同一元数据确定性生成的 256 条路由目录；预热 1,000 次，每轮解析 20,000 次 / 构建 30 次，取 5 轮中位数。

| 输入 | 目录构建：修改前 → 后 | 命名解析：修改前 → 后 |
| --- | --- | --- |
| 当前应用，2 条路由 | 2,863 → 3,309 ns | 438 → 478 ns |
| 合成目录，256 条路由 | 463,213 → 378,133 ns | 1,790 → 609 ns |

基准源码、完整输入、构建日志和原始结果在 target/test-reports/router-refactor-cost-nw0d2nmi/，以 cargo run --offline --release --bin router-cost 可复验。小目录有约 40 ns 的解析开销，较大目录收益明显；这不是实际宿主帧耗时或包体积结论。

保留公开 matcher 命名空间、History 的只读投影序列化、useRoute 的即时读取投影，以及主页正在使用的计数状态；这些不是本轮确认的缺陷。设置发生不可回滚 I/O 后的内部故障仍关闭服务，窗口纯内存发布失败则保持原状态供重试，两者不抽成统一关闭策略。本轮未重跑原生窗口 / VM reload、跨平台编译、发布包或远端 CI，此前记录仅代表对应历史快照。


## 24. 更新入口显隐与检查状态（2026-11-13）

Rail 根据投影的 visible 在组件边界挂载更新入口：未检查、已是最新、未配置、不支持及无候选检查失败时隐藏；检查中显示无中心图标的灰色不定进度环，并禁用重复触发；有更新时显示系统下载图标，提示“有可用更新”。下载继续显示带菱形的进度环；有候选的下载 / 安装失败保留警示入口。错误详情仍保留在设置页。检查和下载的动效遵循减少动效偏好。

受控检查夹具增加完成信号，新增旅程覆盖初始隐藏、检查细环、禁用按钮和提示、已是最新后隐藏；已有旅程补充失败隐藏、有可用更新提示及重复出现 / 隐藏。最终 state-ui 41 项（含更新界面 7 项）、ui 29 项及 nsh check 通过。首次尝试在 RailUpdate 内包裹已构造的触发器导致旅程停滞，已改为在 Rail 的组件边界条件挂载，随后完整重跑通过；中断轮不计成功。

隔离 macOS 原生预览人工确认灰环在两次截图中持续旋转、无中心图标、检查完成后入口消失且设置 / 账户位置保持。使用可控原生 I/O，未执行真实升级。最终记录在 target/test-reports/update-layout-p7ucdq4p，cleanup.json 确认临时树删除。前置预览因缓存宿主与当前接口不匹配作废；重新构建生成的 macOS 宿主后完成验证，未修改框架。Rust 领域规则未改，本轮未重跑工作区测试。


## 25. 手动检查失败保留入口（2026-11-13）

检查失败按 Rust 快照的 manual_check 区分来源：后台失败不显示侧栏入口或自动弹出面板，设置页保留原因；手动失败保留侧栏警示入口，使用系统感叹号图标，与可用更新的下载图标区分。设置页和已打开的更新面板保留具体原因与重试动作。关闭面板不清除失败入口，重新检查确认已是最新后才隐藏。

最终 state-ui 42 项通过（含 8 项更新界面旅程），nsh check 与差异格式检查通过。新增自动时钟旅程覆盖后台失败静默、转为手动重试后显示入口；原有离线旅程覆盖弹层内再次检查失败仍保留弹层、关闭后入口仍在、成功后隐藏。检查调用计数保证断言发生在新的检查完成后。初轮错误的弹层选择器，以及点击面板外夹具按钮导致正常关闭的断言失败已修正，最终完整重跑通过。使用可控原生 I/O；本次未重跑真实下载、安装或原生截图验收。

同日样式回调：手动检查失败恢复为实心警示色圆底加系统下载图标，复用原强调形态和主题配色，移除感叹号专用形态及资源配置。按钮保留、弹层重试及后台静默逻辑不变。12 项更新控制器 / 投影测试、8 项更新界面旅程及 nsh check 通过；未重新执行整套 state-ui 或原生截图验收。


## 26. 日志门面、类型控制与诊断页（2026-11-13）

设计与迁移记录见 [log.md](log.md)。测试遵循现有架构：规则单测在日志 crate 的相应模块，公共契约在 crate/tests，日志专有 Capture / 故障 / 门控写入器在 tests/support；设置和文件归档集成在根 tests/integration，NSH 桥接用例走 state-ui，应用页面回归走 ui，原生布局用例在 tests/e2e 并纳入 e2e-macos。未把业务 mock 加进 usher-test-support，也未给测试公开内部队列、路径辅助函数或状态机。

`logging` 与 `logging-mock` 命令已加入 tests/config.json 及 CI。四类日志 × 六过滤阈值 × 六提交等级逐目标核对捕获结果，另遍历全部 32 种开关组合。契约覆盖脱敏先于后端、部分目标写入故障、OFF、旧句柄实时更新、关闭已入队目标、不向新启用目标补写及无效字段。真实文件适配器对照 Capture 的全部类型 / 等级输出。可控通道占满队列测试替代依赖执行速度的洪泛测试。更新桥接单测验证两条输出流保留 INFO / WARN / ERROR 及原字段。

本机 macOS arm64 最终验证：

| 命令 | 结果 | 报告 |
| --- | --- | --- |
| `python3 scripts/test.py logging` | 20 项通过 | target/test-reports/logging.json |
| `python3 scripts/test.py logging-mock` | 13 项通过 | target/test-reports/logging-mock.json |
| `cargo test --locked --workspace` | 241 项通过 | target/test-reports/logging-workspace.log |
| `python3 scripts/test.py integration` | 44 项通过，含新增 3 项诊断集成 | target/test-reports/integration.json |
| `python3 scripts/test.py state-ui` | 43 项通过，含真实诊断桥接 | target/test-reports/state-ui.json |
| `python3 scripts/test.py ui` | 30 项通过 | target/test-reports/ui.json |
| `python3 scripts/test.py e2e --target macos` | 5 项通过 | target/test-reports/e2e-macos.json |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | 通过 | target/test-reports/logging-clippy.log |
| `nsh check` | 通过 | target/test-reports/logging-nsh-check.log |
| `python3 -m unittest discover -s scripts/tests` | 135 项通过 | target/test-reports/logging-scripts.log |

NSH 使用本机缓存依赖，执行时设置 CARGO_NET_OFFLINE=true。首次并行运行 NSH 导致生成夹具路径冲突，随后串行完整重跑 state-ui 与 ui 通过；该失败轮不计为成功。原生首次沙箱拒绝本地监听端口，解除该限制后完整重跑 5 项通过，不以局部补跑替代整套结果。

原生窗口 / HMR 用例使用脚本临时源码与数据根，官方 NSH 原生测试 runner 使用隔离会话；未改 HOME 或真实用户偏好。诊断原生用例执行真实设置开关、等级下拉选择、滚动和 1100 → 560 宽度变化。已查看 diagnostics-native-wide.png、diagnostics-native-export.png、diagnostics-native-compact.png（均位于 target/test-reports），确认中文说明换行、对齐、禁用状态和归档区无重叠。原有多窗口共享、HMR、关闭重开及原生返回测试同轮通过。

终端 / 内存测试输入是合成事件，不代表真实采集器存在。本轮未验收真实盘满 / 强杀、其他平台原生输出 / 分享或远端 CI 执行；这些仍保留在批次 05。原生显示检查不替代导出打开文件夹与完整落盘退出的人工验收。

## 27. 设置标题状态与日志开关闪烁修正（2026-11-13）

设置标题下的实时保存状态文字及其空行移除，失败重试入口仍走原设置操作。日志输出开关继续在串行保存期间阻止重复提交，但已接入项不再应用禁用透明度；只有尚未接入的终端 / 内存项灰显。未修改 Rust 设置事务、日志管线或状态所有权。

回归使用 tests/ui/fixtures/DiagnosticsPending.tsx 在视图边界暂停一次保存，保留真实设置门面执行提交；它是可控延迟的 UI 夹具，不计为真实磁盘延迟或日志后端验收。无头断言标题状态不存在、等待期间点击受保护、完成后值更新；macOS 原生用例截图保存前 / 等待中 / 完成后三个阶段。已查看截图，并通过本机 AppKit 解码比较等待前后的像素：排除顶部夹具状态行后，整个诊断内容区像素一致。证据为 target/test-reports/diagnostics-switch-comparison.json，原生 JUnit 为 diagnostics-ui-fix-native.xml（2 项通过：实际页面布局 / 控件和可控等待画面各 1 项）。

标题行删除后，旧主题编辑和工具提示用例的部分控件落在底部状态栏遮挡位置；主题状态用例改用足够容纳表单的测试视口，语言操作先滚动至可见区域。没有改动生产布局或滚动组件来迎合用例。

本轮最终串行运行 `nsh check` 与 `python3 scripts/test.py ui` 通过，完整 UI 为 31 项、失败 0 项。之前因旧用例遮挡定位失败的运行不计入通过结果；调整后完整重跑，未拼接单项成功。原生诊断专项 2 项通过，截图与像素对照如上；新增夹具用例和原生用例分别进入已有 ui 与 e2e-macos CI 入口。Rust 未修改，本轮未重复运行工作区 Rust 测试。
