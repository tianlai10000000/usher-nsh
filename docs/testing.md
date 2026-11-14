# 单元、集成与端到端测试方案

本文规定测试分层、mock 与隔离契约、用户旅程及命令 / CI 组织。测试随所属功能交付；批次、完成状态与执行证据统一维护于 [迭代台账](todo.md)。

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

## 2. 宿主能力与证明边界

Rust crate 契约、应用集成、TS 投影和真实桥接分层执行。`--lib` 不自动运行独立 Cargo 集成目标；统一入口显式发现用例并拒绝零测试或遗漏必测组。CI 与本地复用同一配置。

`nsh test` 支持 TS / TSX、JUnit、trace 与 macOS 原生目标。`open({ platform: "macos" })` 只设置无头环境属性；modules 替身、URL 注入与手动时钟不能替代真实宿主。原生目标不使用包含 modules 的无头夹具。

NSH storage 初值不是 Rust 设置目录。原生测试使用独立应用 ID 与临时 data/cache/saved-state 根，不修改 HOME 或真实用户数据。开发宿主、打包运行、签名分发与实际升级分别验收，不能相互替代。

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

所有命令在仓库根目录执行。当前可用入口包括 `cargo test --locked --workspace`、`cargo test --locked -p usher-update`、`nsh check`、`nsh test tests/ui/window-shell.test.tsx` 和 `python3 -m unittest discover -s scripts/tests -p 'test_*.py' -v`；实际执行记录见 [迭代台账](todo.md)。

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

CI 从统一配置运行状态 feature 组合、路由契约、应用集成、TS / UI 和目录检查。纯 crate 可在无 NSH 环境运行；应用和 UI 使用具备对应宿主的 runner。原生 macOS 作业由 workflow_dispatch 的 native_state 开关触发，要求 usher-native 标签和解锁桌面，不假定托管 runner 提供这些能力。平台编译、原生宿主、打包分发与实际升级是独立的证明范围。远端执行和门禁覆盖状态见 [迭代台账](todo.md#testing)。

### 公共属性转发与连续输入

公共控件契约用例必须让底层值真实变化，并在同一个挂载实例中再次输入；固定 checked / value 只证明事件被触发，无法证明新值重新投影。状态栏回归在完整 App 中选择 Gallery 主题，连续四次点击并检查 Shell 状态栏显隐、忙碌恢复与后续导航。原生脚本复制实际 App.tsx 为隔离的 ProductionApp.tsx，避免用只含 SettingsPage 的夹具替代 Shell 验收。

`python3 scripts/native_button_acceptance.py --filter 'status bar remains responsive' --junit target/test-reports/statusbar-focused.xml` 可用于单项诊断；filter 运行不执行完整截图像素对照，不计为完整验收。交付必须去掉 filter 跑完整套件。证据和完成状态仅在 todo.md 维护。


## 业务状态模块隔离测试

`ui/state/__tests__/business-modules.test.ts` 使用 settings/session、延迟 Promise、picker、诊断健康与 owner hooks 的可控替身，直接调用生产模块。覆盖公共请求互斥、失败重试与重连、只读/校验反馈、开始操作与结果返回之间的销毁、取消/超限/读取失败、预览票据清理、独立诊断导出和语言快照投影。测试不访问真实文件或用户配置，不把 mock 导出与系统选择器等同于真实宿主验收。

模块 mock 已纳入 state-ui 的递归发现及必测前缀；真实 NSH 响应式 owner 由 appearance-host 夹具验证，真实桥接与页面往返使用原 state-ui/UI 套件。picker 取消和公共按钮交互沿 `scripts/native_button_acceptance.py` 的隔离 macOS 宿主执行，保留生产调用入口、Rust 状态服务和 AppKit 控件。验收状态和报告仅记入 todo.md。

## 状态模块边界回归

`ui/state/__tests__/decode.test.ts` 使用注入的 wire 和命令 mock 覆盖读取、订阅、回执的非法快照、最后有效值与迟到回调；真实 NSH 桥接仍由 integration 用例独立验证。更新 controller mock 随实现位于 `ui/state/updates/__tests__/`，两处均进入 state-ui 发现目录。

`tests/integration/settings_state.rs` 通过真实临时文件覆盖不同偏好分区并发提交、重新打开、无写权限及重试。`scripts/tests/test_state_boundaries.py` 防止旧门面回流、通用 crate 引入业务依赖、更新状态回到 feature 视图以及设置模型直接读写文件。生成启动主题由 Rust 完整内容比较，强调色 finish 与前景色由主题回归验证。原生 HMR、多窗口和 VM reload 继续使用隔离宿主脚本，不能以 mock 代替。
