# 独立路由模块方案

本文规定路由选型、公共接口、集中页面目录和状态边界。与 [状态管理设计](state-management.md) 配合，批次与验收证据统一见 [迭代台账](todo.md)。

## 1. 现状与目标

实施前 ui/navigation 同时定义页面、历史与面包屑，App 按 ID 分支选择页面。RT3 已移除该实现；App 仅装配 Store、Frame 和 RouterView，未知名称返回 invalid_target，合法未知路径显示 NotFound。

目标提供类似 vue-router 的 `useRouter()`、`useRoute()`、`RouterView` 和命名导航，独立处理路由解析、历史、并发导航及生命周期。业务页面与注册信息集中在 `ui/pages/`。所有跨页面数据继续使用 `usher-state`，路由规则不进入通用状态 crate。

## 2. 成熟方案调研与选择

以下记录基于官方资料与 NSH 0.5.0 接口的选型依据。

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
src/state/window/mod.rs      # Window scope, guards and atomic state commits
src/state/window/session/mod.rs      # Page session rules and typed operation receipts
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

为兼容 TSX 热更新和代码包更新，清单随界面包在窗口初始化时注册到应用服务，注册成功后才能发起导航。带 schema 版本及内容身份；Rust 校验未知 schema、重复 ID / 路径、缺失父级、父级环、参数冲突与初始路由，TS 校验所有 pageKey 都有组件。初始化期间显示加载或初始化错误，不挂载错误页面。窗口仅使用与自身页面注册表一致的目录版本；旧 register 继续只允许完全相同目录幂等重注册；新 attach 支持不同 identity 的原子替换。修改目录内容须更新 identity 并重新生成类型；冒用相同 identity 或重新使用本窗口已淘汰的 identity 均拒绝。迁移保留窗口历史与会话，旧版本绑定失效，规则见第 8 节。这里只允许注册数据，不接受动态执行代码。

## 4. 依赖和状态归属

`usher-router` 不依赖 NSH、应用根 crate、文件系统或 `usher-state`，提供给定 RouteCatalog、RouterState、NavigationCommand 的纯转换。应用服务持有不可变目录和唯一窗口 `StoreWriter<WindowSnapshot>`，把转换结果交给通用状态模块提交；Router 不再保存一份独立的可写历史。

TSX `ui/router/` 通过公共状态适配机制投影快照，只从此模块导出路由动作和查询。通用 state 门面不得公开改写 navigation 的 dispatch；所有程序、菜单、快捷键与原生返回入口经过 window_service。UI 组件使用只读布局契约，不能绕过 Router 直接修改历史。

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

## 8. 目录迁移与宿主适配

目录 identity 表示内容版本。迁移先完整校验候选目录，再逐条重新解析历史 URL，保留 entry key、游标和窗口会话，一次发布新快照。失效 URL 转为 NotFound；同 identity 不同内容、已淘汰 identity 与旧绑定的迟到回执均拒绝。回退旧内容也分配新的 identity。RouterView 在 pageKey 改变时重新挂载。

主窗口接收 UrlScheme 启动及后续 open 事件；scheme 和允许的 host 来自 nsh.json 与 ui/config/routing.json。BackHandler 由聚焦窗口消费，无聚焦窗口时由主窗口处理，根节点交还平台，浮层优先由 NSH 关闭。异步守卫通过后才提交 Rust 历史，不创建第二份 NavStack 历史。NSH owner 释放监听并拒绝迟到事件。NavPage 的 popOnBack=false 同时禁用交互式滑动返回，手势支持需要独立适配。

## 9. 目录结构与错误契约


目录构建按目录元数据、ID 索引、路径 / 参数约束、祖先关系分别校验。Router 预建 HashMap ID 索引和祖先链，resolve.rs 独立负责目标格式化、匹配结果检查、参数与 query 校验、Location 构造；目录顺序不影响解析。命名导航仍在生成路径后经过 matcher，以拒绝被静态路由抢占的目标。真实后端故障使用 BackendUnavailable；无效索引、参数集合或无法匹配自身命名路径使用 InvalidMatch。

应用窗口适配名为 WindowService，会话业务规则位于 window_session。错误保留来源，到 bridge 转为 code；非法目标与后端 / 生命周期失败分别映射。窗口提交失败保留原快照、票据计数与守卫元数据，恢复后可重试；不关闭整个窗口。History 只支持界面投影序列化，不是可反序列化的持久化格式。

生成器同时输出 RouteId / PageKey / 参数类型及 routeMetadata 字面量，routes.json 仍是唯一编辑来源；TS 以 satisfies 校验页面、标题文案和父级类型，无双重类型断言。生成格式变化不改变目录内容，现有 identity 保持不变。门面分离守卫求值、重定向循环和票据清理；无票据时只有明确的 router_invalid_target 映射 invalid_target，服务关闭、容量或后端失败均为 failed。
