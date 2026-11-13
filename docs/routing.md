# 独立路由模块方案

本文规定路由选型、公共接口、集中页面目录、状态边界与实施批次。与 [通用状态管理方案](state-management.md) 配合，项目任务见 [scaffolding.md](../scaffolding.md) 04.10 至 04.13。RT1 已实现独立核心、可替换匹配器、目录与生成契约及测试；RT2 / RT3 已交付窗口服务、守卫票据、集中页面注册和 RouterView，并联合完成 SM4 的代码迁移、无头验证及 macOS 原生多窗口 / 源码 HMR 验收。RT4 的目录升级、深链接和原生返回已实施；macOS 发布包深链接及完整 VM reload 已通过；其余平台与分发 / 升级验收继续按后续批次推进。

## 1. 现状与目标

实施前 ui/navigation 同时定义页面、历史与面包屑，App 按 ID 分支选择页面。RT3 已移除该实现；App 仅装配 Store、Frame 和 RouterView，未知名称返回 invalid_target，合法未知路径显示 NotFound。

目标提供类似 vue-router 的 `useRouter()`、`useRoute()`、`RouterView` 和命名导航，独立处理路由解析、历史、并发导航及生命周期。业务页面与注册信息集中在 `ui/pages/`。所有跨页面数据继续使用 `usher-state`，路由规则不进入通用状态 crate。

## 2. 成熟方案调研与选择

以下为 2026-11-12 官方资料与项目固定 NSH 0.5.0 类型声明的评估，尚未进行依赖安装或兼容性实验。

| 候选 | 已有能力 | 本项目取舍 |
| --- | --- | --- |
| [Vue Router](https://router.vuejs.org/guide/) | 命名路由、嵌套视图、守卫、多种 history；基于 Vue 组件系统 | 交互接口参考；memory history 不消除 Vue 运行时依赖，不直接接入 NSH |
| [TanStack Router](https://tanstack.com/router/latest/docs/overview) | 面向 React / Solid，类型化导航、参数、loader 与文件路由 | 能力完整；现有框架适配不是 NSH 适配。复用其 core 仍需新绑定与状态权威协调，首版不采用 |
| [router5](https://router5.js.org/) | 框架与视图库无关，路由指令转状态，支持转换控制 | JS 完整路由候选；仍要验证 QuickJS、生命周期和与 Rust 唯一状态的配合。若以其内部状态为事实来源，会与既定窗口 Store 冲突，首版不采用 |
| [matchit](https://docs.rs/matchit/latest/matchit/) | Rust 路径匹配、命名参数、末尾通配及冲突检测；调研版本 0.9.2 | 推荐作为可替换匹配后端；不是完整客户端路由，不提供历史、守卫或页面渲染 |
| NSH `NavStack` / `NavPage` | 项目 `.nsh/types/nsh/base.d.ts` 已声明原生页面栈、setRoutes 与变更事件，提供平台导航表现 | 保留为原生表现适配候选；不代替业务路由注册、命名解析和参数校验 |

选择：独立内部 crate `usher-router` 负责纯路由规则，内部优先复用 `matchit`；应用服务通过 `usher-state` 发布唯一窗口路由状态；TSX 模块 `ui/router/` 封装桥接与页面渲染。完整业务路由能力由本项目补齐，不声称引入 matchit 就获得完整 vue-router。匹配器通过项目接口替换，不暴露 matchit 参数借用或错误类型。

该选择基于 Rust 状态权威、NSH 界面与多平台边界的适配成本，不代表 JS 路由库本身不成熟。RT1 必须验证依赖许可证、锁定版本、支持目标及构建成本；性能和包体积在实际构建后记录。首版不引入 Vue、React、Solid 或浏览器路由运行时。

## 3. 模块和集中页面目录

以下核心、服务、桥接、页面和门面目录均已实施：

```text
crates/usher-router/        # Pure routing contracts, matching and transitions
src/router_service.rs      # Window scope, commands, guards and state commits
src/router_bridge.rs       # NSH DTOs and typed commands
ui/router/                 # Public facade, transport and NSH RouterView binding
ui/pages/
  routes.json              # Authoritative serializable route catalog
  index.ts                 # Exhaustive page-key to component registry
  home/Page.tsx            # Home route entry
  settings/Page.tsx        # Settings route entry
  not-found/Page.tsx       # Unmatched-location presentation
ui/features/               # Feature views, services and reusable business pieces
```

`ui/pages/` 汇总所有可路由页面入口。首页内容从 App 迁到 home/Page；设置入口迁到 settings/Page，内部可以组合 `ui/features/settings/` 的分类内容。每新增可导航页面都在此目录新增入口并注册；普通业务组件无需搬进 pages。`App.tsx` 仅装配状态、Router 和 Frame，内容通过 `RouterView` 展示，不再写按路由 ID 分支的页面树。

`routes.json` 是路由 ID、路径模板、参数约束、父级元数据、pageKey、标题键及菜单角色的唯一人工维护来源；主页和设置的路径也在此配置，不在 Rust 或 TSX 写第二份。路径是应用配置，新增业务路径时随功能评审，不由通用库给硬编码默认值。首批沿用两个现有页面；设置分类继续属于窗口 session，不同时变成 query 和第二份选中状态。

生成步骤从目录清单产出 TS RouteId / 参数类型及 pageKey 集合，`ui/pages/index.ts` 对生成集合做穷尽组件绑定检查。Rust 使用同一份清单的序列化数据与通用 schema 解析，不另写应用路由枚举。生成器、输入依赖、输出检查及 CI 接入在 RT1 / RT3 实施；生成结果不是另一份可编辑路由表。

为兼容 TSX 热更新和代码包更新，清单随界面包在窗口初始化时注册到应用服务，注册成功后才能发起导航。带 schema 版本及内容身份；Rust 校验未知 schema、重复 ID / 路径、缺失父级、父级环、参数冲突与初始路由，TS 校验所有 pageKey 都有组件。初始化期间显示加载或初始化错误，不挂载错误页面。窗口仅使用与自身页面注册表一致的目录版本；旧 register 继续只允许完全相同目录幂等重注册；新 attach 支持不同 identity 的原子替换。修改目录内容须更新 identity 并重新生成类型；冒用相同 identity 或重新使用本窗口已淘汰的 identity 均拒绝。迁移保留窗口历史与会话，旧版本绑定失效，规则见第 11 节。这里只允许注册数据，不接受动态执行代码。

## 4. 依赖和状态归属

`usher-router` 不依赖 NSH、应用根 crate、文件系统或 `usher-state`，提供给定 RouteCatalog、RouterState、NavigationCommand 的纯转换。应用服务持有不可变目录和唯一窗口 `StoreWriter<RouterState>`，把转换结果交给通用状态模块提交；Router 不再保存一份独立的可写历史。

TSX `ui/router/` 通过公共状态适配机制投影快照，只从此模块导出路由动作和查询。通用 state 门面不得公开改写 navigation 的 dispatch；所有程序、菜单、快捷键与原生返回入口经过 router_service。UI 组件使用只读布局契约，不能绕过 Router 直接修改历史。

目录属于业务配置；参数、当前 location、历史条目、游标与 pending navigation 属于窗口 RouterState；网络数据属于业务 Store；组件、原生栈句柄及离开确认回调归界面适配器。路由不会保存页面函数、任意闭包或用户凭据。路径参数及 query 不作为业务数据仓库。

## 5. 稳定接口与导航语义

| 接口 | 约定 |
| --- | --- |
| `resolve(target)` | 校验命名目标及参数或解析内部 location，返回结构化目标，不改变历史 |
| `push(target)` / `replace(target)` | 异步返回明确 NavigationResult；成功分别追加或替换当前条目 |
| `back()` / `forward()` / `go(delta)` | 访问同一历史，边界返回 no-op；被守卫拒绝时游标不移动 |
| `useRoute()` | 只读当前 ID、params、query、匹配祖先、entryKey 及导航状态 |
| `useRouter()` | 获取绑定窗口的动作和解析门面；不能操作其他窗口 |
| `RouterView` | 按 pageKey 挂载对应页面，复用状态适配器；错误与未找到状态有明确出口 |
| `RouteMatcher` | 通用匹配契约；初始为 matchit 适配，测试匹配器复用同一套外部行为断言 |

NavigationResult 区分 committed、duplicated、cancelled、superseded、not-found、invalid-target 与 failed。底层异常映射稳定错误码。首次迁移可调整旧同步 boolean 接口的调用方，之后替换匹配器或存储后端不得改变这些返回语义。

路径解析与格式化必须互为可测试契约：路径大小写敏感，目录声明规范路径；首版保留尾斜杠差异，不隐式重写业务地址。params 按单个路径段编码、仅解码一次；非法编码、缺参数及普通参数中的路径分隔符拒绝。query 按重复键保留多值，参数类型与默认值由清单声明，未声明字段按清单策略拒绝或保留，不静默猜测。首版不支持任意正则模板、可执行参数校验器或隐式可选路径片段；外部 URL 由独立打开链接能力处理，不当作内部页面。

同一规范 location 的 push 返回 duplicated；push 在后退后截断前进分支；replace 不增加条目；go 在提交时基于匹配的历史版本移动。每个新历史条目有独立 entryKey，参数变化也构成不同 location。面包屑取目录的显式父级关系，不能由历史推导；首版父级表示导航层级，不自动意味着嵌套布局挂载。嵌套路由出口可后续扩展，首批采用单 RouterView。

未知命名路由与无效参数返回错误，保留当前页面；未匹配的合法内部路径产生 NotFound 位置，由配置的 not-found pageKey 展示，保留用户原目标供返回。既有“未知字符串回主页”的行为只允许在旧入口兼容适配中暂时保留，迁移完成后移除并补回归。

## 6. 守卫、并发和页面生命周期

导航流程为：解析目标 → 创建 navigationId → 执行离开 / 进入守卫 → 校验目录与来源版本 → 原子提交历史及当前位置 → 渲染。守卫返回允许、取消或重定向；可能异步的离开确认属于界面效果，回执携带 navigationId。业务权限须由业务服务再次校验，路由可见性不作为授权边界。

首版并发策略为最新导航请求生效，旧 pending 导航被标记 superseded；迟到守卫不得提交。守卫期间旧页面继续有效，取消不改变历史。重定向在同一次导航内处理并仅提交最终目标，检测重复目标环；链长上限由 routes.json 的 redirectLimit 注入；当前配置为 8，守卫环和上限同时检查。守卫异常返回 failed，保留可重试的旧位置；窗口关闭取消 pending 导航并清理回调。

导航状态先于页面挂载确认，页面组件渲染失败由 RouterView 的错误边界展示，可返回或重试；不要假设能回滚页面已经发生的副作用。首版页面离开时卸载，跨页面需要保留的数据从窗口 Store 恢复；不默认缓存全部页面。组件复用键由 entryKey 与 pageKey 决定，并验证 replace、前进后退和参数变化时的重建行为。

## 7. 原生导航与深链接

桌面首版使用内存历史及 RouterView，不借用浏览器 `window.history`。移动返回先由局部浮层消费，未消费再发送 Router.back，历史根节点交回平台既定行为。深链接由宿主适配器提取和校验允许的 scheme / host，再转换成内部目标并经过相同守卫，窗口未就绪时暂存至目录注册完成。

已检查固定 NSH 0.5.0 的公开类型及实现：NavStack 的 `onChange` 在变化后通知；NavPage 的 `popOnBack=false` 能在提交前阻止原生出栈，但同时关闭交互式滑动返回。当前实现使用公开 BackHandler 接住返回并等待守卫，未接入 NavStack 或 iOS 交互式返回。若使用 NavStack，其页面栈只是当前历史分支的渲染投影，带 entryKey 与同步标记防止 setRoutes / onChange 回环，不另存业务历史。若无法满足守卫或布局契约，按 CONTRIBUTING 的组件能力要求报告缺口并取得处理决定，不能自行改写框架或假定事后回推等价于阻止返回。

## 8. 实施计划

| 批次 / 项目任务 | 前置 | 交付和验收 | 状态 |
| --- | --- | --- | --- |
| RT1 / 04.10 | 已获授权 | 建立独立 usher-router；matchit 可替换适配、目录 schema、生成契约、解析格式化、纯历史转换与导航结果；依赖许可证和目标编译验证 | 已完成，见第 9 节 |
| RT2 / 04.11 | RT1、SM1 至 SM3 | router_service / bridge、唯一窗口 Store、目录注册、并发票据与守卫；验证拒绝不改历史、旧回执、窗口隔离、目录版本及关闭 | 已完成，见第 10 节 |
| RT3 / 04.12 | RT2；与 SM4 联合验收 | ui/router、ui/pages 集中目录、注册生成及 RouterView；迁移首页、设置、菜单、快捷键、面包屑与历史按钮；清除 App 路由分支和旧可写 navigation；全共享状态验收 | 已完成代码迁移和无头验收；SM4 macOS 原生验收见测试方案第 14 节 |
| RT4 / 04.13 | RT3、目标宿主可用 | 深链接、原生返回 / NavStack 适配、HMR 与目录变更、各平台宿主及发布回归；明确未支持能力与成本 | 部分完成：代码、无头与 macOS 开发宿主通过；macOS 发布包 URL 与完整 VM reload 验收通过，其他宿主和剩余发布场景待办，见第 11 至 13 节 |

RT1 不依赖状态管理实现；RT2 复用 SM1 至 SM3；RT3 是 SM4 导航迁移的唯一交付，不再做另一版导航 Store。RT4 不阻塞桌面现有两页迁移。账户、消息与更新专用页面在对应功能批准时注册，不为演示路由提前实现业务。

### 8.1 现有页面整改清单

路由模块交付必须包含既有页面及全部导航入口整改，不能只新增 crate 或演示页面。下列整改均已交付代码与无头回归；macOS 原生多窗口和源码 HMR 验收见测试方案第 14 节：

| 当前范围 | 整改目标 | 回归要求 |
| --- | --- | --- |
| `App.tsx` 首页内容和条件页面树 | 首页迁至 `ui/pages/home/Page.tsx`；App 装配 Frame / RouterView；计数归窗口 Store | 首页操作后往返设置仍保留计数；App 无路由 ID 条件渲染 |
| `ui/features/settings/SettingsPage.tsx` | `ui/pages/settings/Page.tsx` 作为唯一可路由入口，内部可继续组合 feature 组件 | 分类、外观、主题、语言、背景、诊断与导出原有能力全部保留；分类通过 session 恢复 |
| `ui/navigation/index.ts` | 路由元数据迁至 routes.json；历史规则与状态接入 Router / State；兼容层只委托新门面 | 无第二份历史或 route signal；整改完成后移除旧可写实现和未知字符串回主页兜底 |
| `Frame.tsx`、TitleBar、Rail、SecondaryMenu、Navigation 组件 | 使用窗口 Router 门面和目录派生菜单 / 面包屑；同步调整类型与调用方 | 菜单、历史按钮、快捷键、面包屑指向同一历史；异步结果及失败有明确处理，不沿用同步 boolean 假设 |
| 页面注册、NotFound 和错误出口 | 真实页面集中注册；缺少 pageKey 绑定在检查阶段失败；合法未知路径显示 NotFound | 可返回 / 重试；无效命名目标保留旧页；不会因注册遗漏静默回主页 |
| 页面与常驻 Shell 共享数据 | 按状态方案 §8.3 全量迁移；页面卸载后从唯一 Store 恢复 | 预览、反馈、导出、计数与 selector 无旁路状态；应用偏好同步、窗口会话隔离 |
| `tests/ui/` 现有用例 | 逐项更新正常入口，保留 window-shell、appearance、settings-layout、diagnostics 等回归 | 不通过删除旧断言、放宽保存成功条件或添加假快照使整改通过 |

完成时登记“原文件 / 入口 → 新入口 → 测试用例 / 命令 → 实际结果”，扫描 App、页面、Shell 和控制器的旧导航 import、直接桥接调用和可写共享 signals；扫描结果结合人工判断局部状态归属，不能一律禁止 NSH 的局部响应式状态。保持既有 testID 与用户行为契约；确需调整时同时更新真实旅程，不保留只供测试操作的产品导航入口。

### 8.2 Mock、命令与测试门禁

测试依据、目录和场景见 [测试方案](testing.md)。RT1 随核心交付目录 / 历史 fixtures、matchit 与参考匹配器的同契约测试；RT2 交付可控守卫、乱序回执及唯一 Store 的应用集成；RT3 交付 RouterView mock 测试和现有页面真实桥接旅程。mock 组件通过不能替代 E2E-01 至 E2E-03 等真实链路验收。

已实现独立入口 `python3 scripts/test.py router`、`python3 scripts/test.py routes-check`，以及 `integration`、`router-ui`、`ui` 和 `e2e --target headless`。macos target 已提供隔离目录下的原生窗口 / HMR 验收，需解锁的 macOS 图形会话；具体执行命令、fixture 加载及非零失败约束以测试方案第 7 节为准。测试目录、命令包装器与 CI 发现机制必须随实现交付，不能只在文档列出命令。

验收包括：重复 ID、参数歧义、静态优先匹配、编码、query 多值、resolve/format 往返、push/replace/go、前进分支截断、导航去重、取消/异常/重定向环、异步乱序、卸载清理、NotFound、窗口隔离、页面注册遗漏与生成产物过期。单测使用同一外部契约切换匹配器；NSH 用例验证真实挂载和按钮语义；原生手势不能用纯函数测试替代。

实施后在仓库根目录运行 `cargo test --locked -p usher-router`、`cargo test --locked --workspace`、`nsh check` 和对应路由 / Shell / 设置用例，生成器检查命令随实现写入 README 与 CI。已执行结果见测试方案第 13 节；不以无头通过代替原生平台验收。


## 9. RT1 实施记录

2026-11-12 实现 `crates/usher-router/`：不可变 Catalog / Router、可注入 MatcherFactory / RouteMatcher、Location / Target、纯 History 转换及稳定导航结果。核心不依赖 usher-state、NSH、文件系统或业务页面；单独的 usher-routes CLI 负责文件读写。首版可移植模板仅支持绝对路径、整段参数和最后的 catch-all，拒绝正则、隐式可选片段、参数前后缀及重复参数名。保留尾斜杠；命名参数被静态路由遮蔽时返回 invalid-target，不静默切换页面。

路径仅解码一次，编码的分隔符、反斜杠、控制字符及点段拒绝；query 重复值保留顺序，键排序，参数约束与默认值来自目录。普通参数不能包含分隔符；catch-all 按路径段编码。合法未知路径返回配置的 NotFound，未知名称与无效参数保留旧历史。push / replace / go 返回新历史，entryKey 和 revision 检查溢出，不原地更改输入。

固定使用 [matchit 0.9.2](https://docs.rs/crate/matchit/0.9.2)，许可证 MIT AND BSD-3-Clause、MSRV 1.66、无运行依赖；默认关闭 matcher-matchit feature。仅测试使用 [proptest 1.11.0](https://docs.rs/crate/proptest/1.11.0)，MIT OR Apache-2.0、MSRV 1.85，关闭默认 features 并仅启用 std。新增锁定包为 matchit、proptest、rand_xorshift 与 unarray；没有 UI 框架依赖。

参考匹配器和目录 / 场景 fixtures 留在 crate 的 tests/，两后端运行相同解析、拒绝、目录与历史契约；属性测试数量 128、固定 seed 20261112 来自 cases.json，失败缩减由 proptest 保留。通用夹具身份校验抽取到 usher-test-support 的 fixture 模块，路由和既有状态 fixtures 共同复用，业务数据和解析器仍留在各自测试模块。

`python3 scripts/test.py router` 分别执行无默认 feature 和 matchit；`routes-check` 当前验证合成目录的生成契约及 CLI 过期 / 缺失报告拒绝，RT3 已追加实际业务目录的新鲜度检查，生成检查单独计为 checks，不冒充测试用例。类型生成只产出 RouteId、PageKey、RouteParams、RouteQuery；JSON 目录仍为唯一人工来源。CI 的独立核心作业已接入两命令及 Clippy。Windows MSVC、Linux GNU、iOS 真机和模拟器目标已通过纯 crate cargo check；这不是对应原生应用验收。完整包体积与发布构建成本仍待 RT4 / SM6。


## 10. RT2 / RT3 与现有页面整改记录

为继续 SM4，先完成其导航依赖 RT2 / RT3。src/router_service.rs 组合纯 Router 与 usher-state，用唯一写入器提交 WindowSnapshot；历史和 session 在同一不可变快照中发布。守卫尚未完成时保留当前页面，Rust ticket 与历史 revision 拒绝迟到回执，session 变化不会误废弃导航；重定向只提交最终目的地，保留 replace 语义并检测环和目录配置的上限。

WindowState 使用引擎 Local 内的 Windows 服务表装配；窗口 ID 由可信 TS 传输适配器捕获 currentWindow，业务门面不暴露窗口选择，动作绑定 Rust StoreId；同目录重复注册复用服务，不同目录内容拒绝且不改变当前历史。订阅复用 SM3 的有界回调与 Stop 清理。VM 关闭主动关闭服务，普通页面仅借用查询 / 导航能力。宿主同 VM 窗口映射、真实源码 HMR 和关闭重开已在 macOS 通过；原子目录升级 / HMR、深链接及原生返回仍待 RT4。

| 原入口 | 当前入口与职责 | 已通过的验证 |
| --- | --- | --- |
| App.tsx 条件页面与计数 signal | ui/pages/home/Page.tsx、settings/Page.tsx；App 仅组合 Frame / RouterView；计数在 Rust session | e2e 实际 App 往返保留计数与分类；ui 29 项 |
| ui/navigation/index.ts 可写历史 | 已删除；ui/router/index.ts 门面 → WindowState → RouterService → usher-router 规则 | router 9 / 14、integration 窗口 8、router-ui 8 项 |
| 手工页面目录与元数据 | ui/pages/routes.json 单一来源，生成 RouteId / PageKey / 参数类型；index.ts 穷尽注册 | routes-check 3 项 + 真实目录检查、nsh check、双语 titleKey 集成 |
| Frame、Rail、TitleBar、SecondaryMenu、Navigation | 全部使用 ui/router；菜单 / 面包屑从目录派生，快捷键和按钮提交同一历史 | 原有 window-shell / tooltips 与 e2e 菜单、快捷键、前进分支 |
| 未知目标回主页 | 合法未知路径渲染 not-found/Page.tsx，无效名称返回 invalid_target 保留旧页 | router 两后端同契约、RouterProbe 真实 NotFound / back |
| 页面直接持有共享反馈与预览 | appearance 读取窗口 session，settings / diagnostics 独立只读投影；动作带 ticket | state-ui 20、窗口集成、WindowLifecycle 同 VM 重挂载 |

RouterView 接收装配入口注入的类型化 PageRegistry，按 entryKey 挂载页面并提供渲染失败重试 / 返回；App 初始化另有错误出口。独立错误注入测试保留真实 RouterView 和历史服务，只替换页面组件，验证首次失败重试不改变 entryKey，以及持续失败页面可以返回既有历史。源码扫描已无旧 navigation import 或第二份 history signal。有关共享状态、测试数据和原生限制分别见状态方案第 12 节、测试方案第 13 节。

## 11. RT4 实施与验收（2026-11-13）

公开深链接使用用户确认的 `usher://app/…`。scheme 由 nsh.json 声明，允许的 host 由路由配置提供；传输层验证地址后进入同一导航守卫，拒绝用户信息、端口、片段和非许可来源。未就绪时仅保留最新地址，卸载后释放监听。

目录更新通过显式 attach 完成：完整验证候选目录，重新解析每个历史 URL，保留 entry key、游标和窗口会话，一次发布新快照。失效地址或收紧的参数规则转为 NotFound；同 identity 不同内容及已淘汰 identity 均拒绝。新绑定携带目录版本，旧绑定与迟到守卫不能提交到新目录。旧 register 与裸 StoreId 桥接继续兼容。

原生返回使用 NSH 0.5.0 公开 BackHandler，同步消费非根返回请求，异步守卫通过后提交 Rust 历史；根节点交还宿主，浮层优先由 NSH 关闭。当前页面不引入第二份 NavStack 历史。NavPage 的 popOnBack=false 可阻止原生提前出栈，但同时禁用交互式滑动返回，因此本批不承诺 iOS 交互式返回手势。

实现使用现有依赖，没有新增第三方库。`usher-router::migrate_history` 隔离匹配后端；应用服务在 `usher-state` 一次发布目录迁移结果，测试继续复用 `usher-test-support` 的可控 future 工具。旧设置桥接及持久化文件格式没有变化。

主窗口接收 UrlScheme 的启动及后续 open 事件；只有聚焦窗口消费系统返回，无聚焦窗口时由主窗口处理。NSH owner 释放 effect，适配器主动移除事件监听并屏蔽迟到事件。根节点返回 false 交还平台；NSH 测试驱动在已注册 BackPress 时可能仍返回成功，因此根节点验收同时检查历史未改变，并以单元契约验证返回 false。

公开 scheme/host 于本批经用户明确确认；配置为 `nsh.json.schemes` 和 `ui/config/routing.json.hosts`。修改目录后须提供新 identity；回退旧内容也应赋予新的 identity，避免旧 HMR 闭包重新附着。目录迁移保留 URL，不按旧 route ID 重定向；新路径可正常导航，旧路径作为 NotFound 保留。RouterView 在 pageKey 改变时重新挂载组件。

验证：Rust 工作区 229 项、路由独立后端 10 / 16 项、应用集成 36 项、router-ui 11 项、无头 E2E 4 项；macOS 4 项（窗口隔离、目录版本 HMR、关闭重开、真实返回及浮层/守卫）。原生 HMR 日志 `target/test-reports/native-run-kb0wdknr`；测试数据根已清理。完整命令与结果见测试方案第 17 节。

macOS 发布包系统 scheme 注册、外部启动与唤醒已在第 12 节补齐。完整 VM reload 已在第 13 节补齐；尚未验收：跨进程持久状态恢复、Windows/Linux/Android/iOS 宿主与发布回归。iOS 交互式滑动返回未实现。上述范围保留 RT4 部分完成状态，不能用无头 UrlScheme 事件注入替代系统注册验证。SM6 上次构建成本报告属于此前快照，本批未重新测量包体积。

## 12. macOS 发布包深链接验收扩展

按用户授权继续验证真实 release `.app` 的 Info.plist URL 声明、LaunchServices 注册、外部 URL 冷启动和已隐藏应用的同进程唤醒。使用 `nsh package --target macos --arch arm64`，保留真实产品 App、路由桥接及官方发布运行器，仅在隔离源码副本添加只读路由观察器，并分配唯一测试应用 ID、关闭测试副本更新源。

发布运行器没有 data-dir 参数；不更改 HOME，不修改 NSH 源码或发布二进制。脚本通过系统目录 API 为唯一测试 ID 建立专属目录链接，指向临时 data/cache/saved-state 根；已有路径（包括悬空链接）一律拒绝，不会覆盖。若系统默认处理程序已指向本轮测试包，直接用 `open <URL>` 验证默认分派；已有其他默认处理程序时使用 `open -a <隔离测试包> <URL>`，避免改写或调用用户已安装的应用。LaunchServices 独立查询确认注册。退出后注销测试包并移除专属链接；如果不能结束测试进程则保留隔离目录，验收失败。

新增命令 `python3 scripts/test.py packaged-routing`，要求解锁的 macOS 图形会话，已加入手工开启的原生 CI 作业；注册、冷启动、唤醒分为三项 JUnit 验收。2026-11-13 解锁会话下完整 3 项通过；报告为 `target/test-reports/packaged-routing-jhaxier9/report.json`。此验收针对本机 ad-hoc 签名 release 构建，不代表 Developer ID 公证、Gatekeeper 下载链路或升级验收。

后续按用户指定顺序继续分批：RT4 的完整 VM reload → 可用目标平台宿主与发布回归；随后 SM6 的完整应用跨平台构建 → 宿主验收 → 远端 CI → 发布运行和实际升级旅程。每批按实际设备、工具链和发布条件记录证据与未完成项。

脚本在构建前及启动前检查 CGSession，锁屏或无前台会话直接失败；清理允许 AppKit 已删除空 saved-state 链接，但拒绝移除被替换的路径。冷 URL 启动与正常前台应用的运行中唤醒作为独立场景。此次默认 handler 由系统解析到隔离测试包；裸 URL 冷启动进入设置页，运行中的隐藏应用收到裸 URL 后恢复并激活，显式应用 URL 打开也保持相同 PID 且 active=true。原 handler 为 null，注销后恢复 null；所有临时根和目录链接均已移除。

仅完整通过的解锁运行计作验收；此前锁屏及清理诊断记录不计入通过数量。Apple 的 [NSApplication.activate 文档](https://developer.apple.com/documentation/appkit/nsapplication/activate%28%29) 说明普通激活请求不保证成功，本项目未新增强制抢焦点行为，也未修改 NSH。

## 13. 完整 VM reload 与本机平台续验

`python3 scripts/test.py native-reload` 已完整通过 1 项真实 macOS 验收。测试使用官方宿主的 Cmd+R 路径重建 VM，未使用组件重挂载替代。`tests/e2e/fixtures/NativeReload.tsx` 装配真实设置、窗口和路由服务；重载前保存 dark 偏好、设置窗口计数 / 分类 / 预览并挂起守卫，通过隔离 Storage 保存旧 scope / ticket 作为回调检查点。重载后新 StoreId、实时订阅和 dark 偏好恢复，历史回到根节点，计数 / 分类 / 预览 / pending 重置，旧回调不能提交。报告为 `target/test-reports/native-reload.json`、`native-vm-reload-0.xml`。VM reload 不等同于整个进程退出重启或应用升级。

官方原生 runner 给每个 open 创建独立 data/cache 目录；本用例不读写真实用户设置，不改 HOME。独立套件进入手工 native_state CI；无头 E2E 明确列出两个可运行文件，macOS 专用 reload 文件不进入无头发现。

本机 iOS 检查发现当前 Xcode 的 simctl 包装器要求 CoreSimulator 1174.21，已安装 1174.9.2；包装器触发首次启动组件安装并停住，`xcodebuild -checkFirstLaunchStatus` 返回 69。已终止本任务的安装查询，未安装组件、创建设备或修改现有模拟器。直接读取已安装 simctl 的可用设备清单成功，但这不算宿主验收。待 Xcode 环境就绪后再创建专用模拟器运行；本轮没有连接 Android 设备、Windows/Linux 宿主或远端 runner，用户确认先推进本机可验证部分。iOS 交互式滑动返回仍未实现，不以环境修复替代功能实现。
