# 独立快捷键模块方案

## 1. 范围与设计决策

快捷键规则独立为 `crates/usher-shortcuts/`。该 crate 只管理通用绑定集合、不透明作用域及冲突、注册计划、初始化状态和过期回执校验；NSH 负责按键识别、平台修饰键、输入法处理、菜单标签和系统注册。具体业务动作继续由 Router、设置及各 Feature 执行。

本文区分当前本地输入实现与系统全局接入目标。核心 crate 无依赖，应用 `src/shortcuts/` 拥有目录、窗口服务、声明缓存及版本化健康状态，`ui/shortcuts/` 持有 NSH keymap 句柄并分发业务命令。HistoryControls 仅显示导航控件；RouterView 按 entryKey 借用页面租约。状态栏读取独立投影，显隐不改变注册。生产目录包括导航和两个全局业务键位；页面机制使用隔离测试宿主验收，不向业务页面添加测试键。Global 已通过 NSH GlobalShortcut 接入进程协调器，默认配置打开设置与切换浅深色；只有系统接受回执才显示已注册。批次和验收证据统一见 [todo.md](todo.md#shortcuts)。

应用层区分三类绑定（这些业务类别不进入核心 crate 的作用域枚举）：

| 类别 | 初始化时机 | 生效范围 | 输入后端 / 拥有者 |
| --- | --- | --- | --- |
| Global | 程序启动，执行宿主就绪后注册 | 应用在后台时也可收到系统事件 | `GlobalShortcut` / 进程协调器及唯一执行宿主 |
| Application | 窗口创建并取得真实身份后 | 当前窗口内跨页面有效 | `keymap` / 窗口服务 |
| Page | 首次进入页面时懒加载，处理器就绪后注册 | 当前路由实例激活时有效 | `keymap` / 窗口服务中的页面租约 |

Application 不等于系统 Global。编辑控件的复制、粘贴、光标移动、Tab 遍历和浮层 Escape 继续由 NSH 控件负责；不建立覆盖整棵组件树的第二套键盘系统。产品默认全局键位见第 10 节；其余动作不自动占用系统键位。多段按键序列、录制改键界面与持久化覆盖不在首期范围。

## 2. 已核查能力与参考来源

以当前锁定的 NSH 0.5.0、本地 `.nsh/types/nsh/index.d.ts`、`.nsh/types/nsh/desktop.d.ts` 及 `nsh-core-0.5.0/src/keymap.rs`、`desktop.rs` 为接口依据。在线参考为 [NSH desktop 文档](https://nsh.dev/modules/nsh-desktop/) 和 [官方仓库](https://github.com/longbridge/nsh)，在线版本不能替代锁定版本的验证。

| 已有能力 | 封装方式与限制 |
| --- | --- |
| `keymap.bind` 返回解绑函数，渲染期间注册随组件清理 | 由唯一窗口适配持有；不能再绑在会随布局隐藏的历史按钮上 |
| `when`、`repeat`、`passThrough` | 保留编辑策略、长按策略和透传策略；未配置 `when` 使用 NSH 自身编辑保护 |
| `keymap.label` | 提供平台展示；菜单、提示及未来设置页不自行拼接键位文本 |
| 同键最新可触发绑定优先 | 应用先做冲突校验，不依赖挂载顺序决定业务优先级 |
| `GlobalShortcut.register` 返回 `Promise<boolean>` | 收到 `true` 才确认注册；`false` 只归类为注册被拒，不能一律断言“被其他应用占用” |
| 重复全局注册替换回调，按键字符串解绑 | 同一规范键只允许一个协调器拥有；普通窗口不得调用 `unregisterAll` |
| NSH Rust 提供快捷键解析类型 | NSH 适配使用其解析器生成核心标准键值，不再实现一套字符串语法 |

NSH 本地 keymap 在原生线程先匹配并消费，再发送 `KeyCommand`；JS handler 的公开返回类型是 `void`，返回 `false` 不构成取消消费。现有 `HistoryControls` 中的布尔返回不能证明不可用历史操作会透传。动态可用性必须参与有效绑定计划；异步更新原生表仍可能有短暂窗口，见第 5 节。

NSH 的窗口 keymap 与系统 Global 的键盘布局语义不能假定完全一致；Linux portal 还可能由用户选择实际键位。当前 Global API 不提供查询实际键位或持续健康探测的完整接口，展示应区分“请求键位”和已确认注册，不虚构实际键位、权限状态或健康心跳。

Slint 参考来自同级仓库的 [快捷键规格](../../usher/docs/specs/shortcuts.md)、[初始化实现](../../usher/src/shortcuts/mod.rs) 和 [macOS 输入桥接](../../usher/src/platform/macos.rs)：窗口显示后校验注册表、绑定回调，再等待平台桥接成功；失败不会宣告就绪。沿用阶段化初始化、单一目录、业务命令复用和失败可见性；不搬入 Slint 类型、物理键监听器、固定延迟、重试次数或模拟百分比。

## 3. 模块与依赖

```text
crates/usher-shortcuts/src/
  lib.rs          Public contracts and exports
  model.rs        IDs, canonical keys, scopes and policies
  registry.rs     Catalog validation and conflicts
  lifecycle.rs    Registration differences, states, effects and receipts
  # Catalog planning lives in registry.rs; error categories live in model.rs.
src/shortcuts/
  mod.rs                    Unified application facade and assembly
  catalog.rs                Lightweight index and loaded catalog validation
  loader.rs                 On-demand module loading and versioned cache
  service.rs                Window/page availability, transitions and state publication
  global.rs                 Process coordinator, VM leases and confirmed registration health
  global_bridge.rs          Process transport and VM teardown fencing
  modules/
    mod.rs                  Explicit module registration
    navigation.rs           Navigation bindings and Router command mapping
    window.rs               Main-window settings intent
    appearance.rs           Application light/dark intent
  adapters/
    mod.rs                  Backend assembly
    nsh.rs                  NSH normalization and capability mapping
  bridge.rs                 Typed bridge DTOs and commands
ui/shortcuts/
  index.ts                  Unified UI facade
  runtime/nsh.ts            Local keymap ownership and cleanup
  runtime/global.ts         VM-lifetime global executor and projection
  global-controller.ts      Serial promises, receipts and late-result cleanup
  transport.ts              Typed bridge adaptation
  status.ts                 Read-only health projection and public actions
  modules/
    navigation.ts           Navigation UI integration when required
ui/config/shortcuts/
  index.json                Lightweight module identities and loading categories
  runtime.json              Registration and retry policies (host integration)
  navigation.json           Navigation binding declarations
```

上述目录由当前本地与全局宿主实现使用；业务模块仅管理动作映射，通用核心不依赖 NSH、usher-state、Router、设置、平台窗口或 I/O。应用服务组合 usher-shortcuts 与 usher-state；后端继续使用 NSH，不新增生产热键库。测试中的 Carbon 独占探针仅作为独立 OS 注册验收工具，不进入产品代码。

`ui/config/shortcuts/` 为分模块应用命令元数据、默认键位和运行策略的单一来源；应用 `catalog.rs` 通过构建/桥接汇总校验，TS 不保留另一张默认表。业务命令处理器在各模块装配处穷尽绑定；启动只检查 Global/Application 模块及轻量索引，页面模块在按需加载后、注册前检查未知命令、缺失处理器及模块版本不一致。核心只识别不透明 BindingId，应用持有 BindingId 到业务 CommandId 的映射；核心不接收业务命令或页面枚举。未来用户覆盖交给统一设置持久化协调器，不由本 crate 写文件。

### 3.1 通用 crate 的强制隔离边界

- 核心只接收标准键、BindingId、ScopeId、OwnerToken、作用域重叠关系、激活集合与 enabled 值；这些身份均不透明，不根据命名推断业务。
- CommandId、RouteId、entryKey、窗口对象、canExecute 计算、业务处理器、默认产品键位、配置读取和持久化全部由应用层持有。核心不调用业务回调，触发事件只携带绑定身份与代次。
- 核心快照只包含注册状态和错误分类；业务名称、文案键、展示标签由应用投影合并。核心不依赖 NSH、Router、业务 Store、UI 或日志实现。
- 作用域重叠关系由调用方声明；未声明关系保守视为可能重叠。核心不定义 Page(RouteId) 或 Application 与 Page 的内建关系。
- 生产依赖和测试依赖均不得反向引用根应用或业务夹具。任意不透明 ID 与内存假后端即可独立验证完整核心契约。

### 3.2 业务快捷键统一目录与模块职责

业务快捷键统一归应用层 `src/shortcuts/` 管理，不放入 `usher-shortcuts`，也不散落在 `src/state/`、Router 实现或页面组件中。该目录使用 `usher-state` 发布快照，但状态库不决定业务模块的目录归属。TS 接入统一归 `ui/shortcuts/`，业务配置统一归 `ui/config/shortcuts/`，三者按相同业务模块命名。

`modules/navigation.rs` 管理导航命令映射、可执行条件和作用域声明，调用既有 Router。`modules/window.rs` 与 `modules/appearance.rs` 分别管理全局设置入口和浅深色意图；TS main-window 适配连接 Router 与设置动作，配置按相同业务模块组织。系统 Global 是注册通道，不是业务模块：某个业务模块可以声明全局或本地绑定，注册机制由共享 service/adapters 执行。

`catalog.rs` 管理轻量模块索引和已加载声明，验证跨模块冲突、缺失处理器和版本；`modules/mod.rs` 显式登记模块描述及加载入口，不在启动时调用全部页面工厂。`loader.rs` 负责按需加载与版本缓存，避免一个大文件包含全部业务分支。每个模块负责自己的命令到既有业务服务的映射，不能复制业务状态。模块之间不直接调用，由统一门面和原有业务服务协作。

页面与布局只通过统一门面申请/释放作用域或读取展示投影，不直接使用 keymap/GlobalShortcut；桥接集中在 `src/shortcuts/bridge.rs`，NSH 注册及清理集中在适配层。需要 TS 挂载接入时才增加 `ui/shortcuts/modules/<module>.ts`，该文件只连接对应页面生命周期和既有动作门面，不复制 Rust 决策或键位表。

依赖方向为页面/宿主 → 应用 shortcuts 门面 → 对应业务模块与既有业务服务；应用 service → usher-shortcuts 通用机制。业务状态经应用 context 转为通用 enabled/作用域输入，核心不反向读取应用。

## 4. 公共契约

下表以已实现的通用公开接口为准。业务模块版本、命令解析和后端能力校验属于应用接口。

| 类型 / 操作 | 语义 |
| --- | --- |
| `BindingId`、`ScopeId` | 调用方提供的不透明身份；业务命令映射仅由应用维护 |
| `CanonicalStroke` | 适配器解析后的按键与精确修饰键值；Primary 已按目标平台展开 |
| `BindingSpec` | 绑定 ID、标准键、ScopeId、输入通道、编辑/长按/透传策略及必需性；不含业务命令 |
| `ScopeRelations` | 调用方声明的重叠关系，不认识页面或路由树 |
| `OwnerToken` | 不透明资源拥有者身份与代次；应用负责映射窗口及页面生命周期 |
| `Catalog::new` | 验证绑定/作用域 ID、能力和重叠关系；业务命令与处理器由应用校验 |
| `Catalog::plan` | 根据外部提供的能力、激活集合和 enabled 值生成计划；不读取业务状态或调用系统 API |
| `Runtime::reconcile / acknowledge / retry / dispose` | 校验完整目标计划，返回 Install/Release 效果；宿主按自身时钟显式调用有预算的 retry，不在核心内调度 |
| `Ticket` | 私有递增序号及 DesiredBinding（owner、revision、绑定）；回执与输入返回原票据 |
| `Receipt` | Installed / InstallFailed / Released / ReleaseFailed；失败保存通用分类，释放失败仍保留资源所有权 |
| `Runtime::snapshot` / `Snapshot` | 资源票据、状态、当前目标和尝试次数；应用聚合阶段和展示，不含业务元数据 |
| `Runtime::accepts(&Ticket)` | 校验一次输入是否仍属当前已安装目标；应用在通过后执行业务，不通过快照广播动作 |

`CanonicalStroke` 是适配器生成的标准文本，不在核心解析语法；NSH 适配使用 Keystroke::parse/reserved/Display。`ResourceKey` 用不透明 DomainId 区分后端资源碰撞域，应用决定全局及各窗口的域映射，不能借不同域绕过业务冲突校验。核心 Catalog 校验声明与作用域，Runtime 还会拒绝同一有效计划中同时占用相同资源的绑定。

Runtime 由单个资源协调 owner 持有，调用方串行投递；完整计划 revision 单调，所有条目必须携带同一 revision。票据序号只在该 Runtime 内唯一，重建时调用方必须更换 OwnerToken，并完成旧资源释放后再接管。相同计划无重复效果，移除/替换立即使旧触发失效；未决安装不得因超时释放所有权，迟到成功先清理再安装后继。暂时失败按构造时传入的总尝试预算显式重试，Rejected/Unsupported 不自动重试；用户确认后可提交新修订重新注册。处置后仍须接收回执直到 is_disposed，释放失败可以继续 retry，不能丢弃 Runtime 宣称清理完成。

后端只执行计划和报告回执；服务是共享注册状态唯一写入口。句柄和 JS 回调不进入快照，平台对象不跨线程传递。计时器由宿主拥有，核心只接收时钟事件；测试使用可控时钟，不实际等待。

冲突按适配器展开 Primary 后的标准值检测，同键必须比较完整修饰键。以下关系是应用提供的策略，不固化在核心中：Application 与所有 Page 重叠；不同互斥页面可复用键位，同一页面多实例在各自窗口隔离。Global 与其他 Global、以及同键本地绑定首期都拒绝，避免前台时双重执行；跨窗口相同 Application 绑定允许。不得依赖“最后注册者获胜”静默覆盖。保留键、非法键和适配不支持的选项明确拒绝。

## 5. 本地分发与页面生命周期

本节由应用 `src/shortcuts/` 及 UI 接入层负责；核心只接收作用域激活、enabled 与 owner 变化，不理解路由、模态或布局。

1. App 装配窗口服务和唯一适配器，历史按钮只发 Router 命令。布局重排、侧栏隐藏和页面跳转不销毁 Application 绑定。
2. Router 提交导航后，旧页面租约失效并解绑；新页面首次进入时请求加载其快捷键模块，加载、校验及处理器装配完成后才领取新租约并注册 Page 绑定。守卫等待或拒绝时，原页面租约仍有效，不启动目标页面快捷键加载。不能单凭 URL 匹配就启用尚未挂载的页面动作。
3. `canExecute` 来自业务服务，不在快捷键中复制历史、忙碌或保存状态。有效计划只包含可执行且当前作用域激活的绑定；业务命令执行时仍再次校验。
4. 编辑保护交给 NSH `when`；应用模态阻断作为独立条件，在打开/关闭命令入口更新，不能依赖下一轮异步视图摘要。页面局部浮层继续遵循 NSH 输入所有权。
5. 触发后核验 owner、目录和注册代次，再调用既有命令入口；异步命令由业务服务管理取消、并发与结果。普通按下默认不接受自动重复；允许重复的命令显式声明且后端必须支持。
6. 同一原生事件只走一条执行入口；菜单 click 复用同一命令处理器。菜单 accelerator 与 keymap 的重叠由适配器明确分配所有权并做一次执行验收，不能靠时间去重丢弃快速合法输入。

公开 `keymap.bind` 没有原生表安装确认、同步动态 predicate 或“handler 拒绝后返还按键”的契约。因此本地状态区分 `Installed`（API 完成注册）与实机验收结果；不能将前者宣称为逐事件输入保证。更新可用绑定表减少无效消费，但路由/模态变化与原生表提交之间仍可能消费一次已失效的按键；代次校验保证它不执行错误业务。若产品要求该窗口也严格透传，须先补 NSH 原生原子更新/输入准入契约，并作为实施门禁；不能用 `return false`、恒定 `passThrough` 或 JS 补发键盘事件伪装解决。

### 5.1 页面懒加载与缓存

懒加载由应用 `src/shortcuts/loader.rs` 和对应 UI 接入层管理，不进入通用 crate。启动只读取包含模块 ID、版本、类别、所属页面及加载入口的轻量索引，不展开全部 Page 绑定、不构造页面处理器、不建立未访问页面的订阅，也不注册其按键。混合声明 Global/Application/Page 的业务模块须拆分加载入口，启动级入口不能顺带初始化页面部分。

页面首次进入按 `NotLoaded → Loading → Validating → Installing → Ready` 推进，失败保存阶段并提供该页面的重试；无快捷键的页面为 NotRequested。加载期间页面可正常显示和使用按钮，但尚未安装的快捷键不能标记为可用。加载失败不回滚已成功提交的路由，也不影响启动级绑定。

按模块 ID 与版本合并并发加载，只缓存成功加载的不可变声明/工厂；不缓存页面实例、闭包、窗口服务引用或原生注册句柄。页面离开立即失效其租约并解绑，已加载模块变为 Inactive。再次进入复用同版本声明，但为新的 entryKey/owner 创建处理器并重新注册。不同窗口可复用不可变声明，每个窗口仍独立装配和持有注册。

加载票据包含模块版本、窗口 opening、entryKey 和请求代次。快速切页、窗口关闭或 HMR 后，迟到结果不得激活原页面或覆盖当前页面状态；模块结果在版本仍有效时可进入声明缓存，但只有仍持有有效请求的页面才能装配注册。失败请求不永久缓存为拒绝的 Promise，重试创建新代次。HMR 使变化模块的缓存失效：当前激活页面重新加载，未激活页面留待下次进入；不重载全部页面。缓存随所属运行时关闭清理，目录移除及版本替换时淘汰旧项。

页面注册前必须校验模块内部冲突，以及与当前 Global/Application 和其他可同时激活作用域的冲突；不能因为懒加载而依赖后注册覆盖。未加载页面不参与运行时全量校验，但 CI 在隔离环境中枚举全部模块并验证完整目录，覆盖潜在共存关系。Global/Application 目录在运行中变化时须重新校验所有已安装 Page 绑定，未加载模块在后续加载时使用最新目录版本。冲突模块不发布可执行注册，不影响已有有效模块。

Rust 静态模块的懒初始化不代表动态装载机器码；目标是延后声明解析、处理器构造、订阅及原生注册。TS 按需加载也须以实际 NSH 构建产物确认是否拆包，不能仅凭动态加载语法宣称包体缩小。先以相同发布配置测量启动至基础就绪、首次进入页面至快捷键就绪、重入延迟、注册数及内存，再评价收益；不预设性能提升比例。

## 6. 初始化、可用性与恢复

```text
Uninitialized → Validating → WaitingForHost → Installing → Ready
                      ↘ Failed                 ↘ Degraded
Ready / Degraded → Recovering → Installing
任意存活阶段 → Disposing → Disposed
```

阶段按 Global 和每个窗口 Local 分开记录，并保留各绑定结果；可选 Global 失败不会禁用正常页面键。没有配置全局键时该通道为 NotRequested，平台不支持时为 Unsupported，均不伪装注册成功。Page 从未加载为 NotLoaded，已加载但未激活为 Inactive，均不算故障，也不计入启动初始化工作量。

以下初始化顺序、宿主恢复触发和清理归属由应用实现；核心仅提供通用注册状态转换：

1. 启动时读取轻量索引，加载并校验 Global 模块；每个窗口创建时加载并校验 Application 模块。页面只在进入时按第 5.1 节加载。纯校验不必等待窗口显示。
2. 取得真实窗口身份、窗口服务及当前执行宿主租约；缺宿主则 WaitingForHost，不在重渲染时重复启动。
3. 安装应用命令分发与状态订阅，然后注册本地计划。绑定在明确的窗口上下文内执行，不根据后来变化的当前焦点猜测目标窗口。
4. Global 在程序启动时由唯一执行宿主按规范键串行注册，等待每个 Promise 结果；该通道与窗口 Application 初始化独立推进，不等待页面或窗口本地绑定完成。新触发若早于注册回执发布，须按同一票据有序处理，不能进入未提交目录。
5. 各初始化范围的必需绑定均取得该后端公开契约能提供的成功回执后，该范围进入 Ready；启动就绪仅聚合已请求的 Global 与当前窗口 Application，不等待任何 Page。当前页面单独聚合其加载和注册状态；Local 的确认强度仅为 Installed，Global 为系统注册接受。快照包含确认级别，不表示系统未来永不撤销。
6. 失败保留阶段、失败绑定和重试入口；未知异常记录脱敏日志。配置错误不自动重试；宿主暂不可用可按配置退避，系统拒绝和用户交互等待不紧密循环重试。

“使用时可用”落实为：只有安装确认后才宣告相应快捷键可用、输入 owner 不附着于临时视图、所有初始化入口幂等、状态不虚报、失效后可以重建。操作系统占用、宿主退出或 portal 拒绝不能被应用强行消除；键盘不可用时保留按钮/菜单操作。

恢复触发包括显式重试、宿主重新就绪、窗口重开及 VM 重建。NSH 未提供持续全局健康查询时不伪造轮询结果，也不在每次聚焦时盲目重新注册。需要恢复时先撤销旧执行租约及所属注册，再由新宿主领取，完成确认后恢复就绪。

### 注册并发与清理

- Local owner 绑定窗口 opening，Page owner 再绑定 entryKey；同名重开获得新身份。卸载页面只释放其 Page 租约，窗口关闭释放该窗口 Local，应用退出释放 Global。
- 全局协调器属于进程而非某个设置目录。多个窗口/数据域竞争同一系统键时必须进入同一仲裁入口。TS 全局执行宿主只保留一份，不能让多个 VM 各自注册覆盖。
- 同一规范全局键的注册、撤销、重新注册必须串行。每个 Promise 完成后检查票据；过期成功必须清理它真正拥有的注册，不能无条件 `unregister(key)` 删除新一代。旧操作尚未结算时不让新一代接管相同键。
- JS API 的按键解绑能力不足以证明跨 VM 崩溃后的句柄清理；实施时验证 NSH VM 销毁行为。若无法确认旧注册已释放，状态保持 Recovering/Failed，不能发布新 Ready；必要时在应用 NSH Rust 适配中使用 `RegisterGlobalShortcut` 的 ID 所有权及 `UnregisterGlobalShortcut`，仍不自行实现 OS 热键。
- 注册超时不代表底层 Promise 被取消；记录 Pending/等待用户状态，停止新的同键操作并拒绝旧代触发，迟到成功仍清理。重试预算和等待策略来自配置，不照搬 Slint 固定次数和时长。
- HMR 同一目录与同一窗口只更新借用门面；目录变化先校验变更索引及受影响的已加载模块，再更新差异；未加载页面继续按需校验。更新失败保留未受影响绑定并报告 Degraded；如果必须先释放旧键才能注册新键，应明确经历 Recovering 和短暂空档，不承诺 OS 原子交换。
- `dispose` 幂等，停止重试、失效票据并清理所属资源；仅全局协调器在确认独占全部注册时才能使用 `unregisterAll`。

## 7. 状态展示与扩展边界

### 7.1 启动状态实时展示与解耦

底部状态栏是快捷键初始化状态的正式查看入口。项目启动时实时显示 Global/Application 的校验、等待宿主、注册、就绪或失败等实际状态；页面进入后另行展示当前页面的懒加载及注册状态，后续恢复及失败也持续更新。快捷键服务独立拥有状态，状态栏只消费只读投影，不负责初始化、注册、计时或状态转换。

```text
usher-shortcuts 通用状态转换
  → src/shortcuts/service.rs 提交并通过 usher-state 发布版本化快照
  → bridge / ui/shortcuts/status.ts 映射只读状态及公开动作
  → App 注入布局契约 → Frame → StatusBar
```

应用启动装配先建立快捷键服务及初始快照，再启动注册流程；状态栏挂载不是启动条件。`ui/shortcuts/status.ts` 负责业务状态投影，布局侧 `ui/layout/contracts/shortcuts.ts` 只定义展示需要的响应式摘要、分项详情和重试动作契约。App 组合二者，StatusBar 不导入核心 crate、注册后端或业务模块内部实现。快捷键目录不得反向依赖 layout 或 StatusBar。未来设置页、诊断视图可以独立消费同一状态源。

订阅遵循现有版本化快照契约：初次连接取得最新状态，后续提交推送更新；避免先读取再订阅造成丢更新，拒绝旧 owner/旧代次/旧版本的回执。每个实际阶段转换立即发布，不通过轮询或固定延迟刷新 UI。连续完成的短阶段可在同一渲染帧合并，不能为展示动画延迟初始化；详情保留当前初始化代次的各阶段结果，即使状态栏晚挂载也能查看。阶段结果为有界结构，不保存无限事件历史。

| 状态 | 状态栏呈现与详情 |
| --- | --- |
| 未开始 / 校验 / 等待宿主 / 注册中 | 显示真实阶段和进行中指示；仅在工作量确定时展示已完成/总数，否则不显示百分比 |
| Ready | 收为常驻就绪入口，可查看各模块、输入通道和绑定结果；本地 Installed 与 Global 注册确认仍明确区分 |
| Degraded / Failed | 保留可见异常摘要，详情列出失败项、稳定原因与可重试性；不因部分成功改为全绿 |
| Recovering | 展示恢复阶段，沿用当前代次，旧代结果不回写当前状态 |
| NotRequested / Unsupported / NotLoaded / Inactive | 详情如实说明，不把未请求全局注册、平台不支持、页面未加载或未激活算作初始化失败 |

状态栏明确区分“启动快捷键状态”和“当前页面快捷键状态”：启动 Ready 不代表所有页面已加载，页面加载中或失败不回退启动阶段，但须在当前页面摘要中可见，不能只保留绿色总状态。未访问页面可在详情中显示为未加载，查看详情不得触发加载；没有页面快捷键时显示未配置。离开页面后不继续把其加载结果当作当前页面状态。

激活状态项打开详情，详情显示阶段结果及按业务模块分组的绑定状态；未加载模块仅显示轻量索引信息，不为展示完整绑定列表执行模块工厂。查看状态的入口在初始化中仍可使用；实际重试按钮按公开动作契约调用服务，并按重试任务状态显示加载和防止重复提交，不直接操作 NSH。各窗口展示本窗口 Local 与进程共享 Global，不混入其他窗口的本地结果。

保留现有状态栏显示偏好：隐藏、卸载或重新挂载状态栏只影响显示与其订阅，不停止初始化、不释放快捷键、不重置注册状态。重新显示立即读取最新快照，初始化在隐藏期间完成或失败也可查看。状态详情关闭同样不取消注册；状态栏渲染失败不影响快捷键服务。断开传输时投影标记状态未知/连接中，不能沿用旧绿色结果声称当前就绪。

文案进入消息目录，布局复用主题 token，保留版本与设置保存状态入口；窄窗口允许按现有布局规则换行。图标配有文本或可访问名称，不只靠颜色区分；减少动效时保留静态进行中指示。响应式契约须保留 getter，避免将启动快照冻结。状态查看不依赖任何已初始化的快捷键，鼠标及常规控件键盘操作均可进入。

### 7.2 其他展示与扩展


工具提示通过同一注册目录加 `keymap.label` 获得标签；未注册的系统键不能显示为“已生效”。日志只记录命令 ID、绑定 ID、owner、generation、阶段和错误分类，不采集原始输入文本。新增页面动作只登记目录、命令处理器和页面租约，不新增原生监听器。

未来改键先校验候选并生成替换计划；保存失败、注册失败与内存已生效分别建模。持久化仍走统一设置事务。全局事件默认只允许应用命令；需要打开窗口时由显式宿主命令执行，不能自动路由到任意后台页面，也不接受悬空页面回调。

## 8. 验证契约

| 层级 | 必须覆盖 |
| --- | --- |
| crate 纯契约 | 标准键冲突、不透明作用域重叠/互斥、非法绑定引用、计划差异、幂等初始化、部分失败、重试预算、过期回执、同键串行接管与 dispose |
| 应用集成 | 分模块目录聚合、缺失业务命令/处理器、跨模块冲突、单进程 Global 仲裁、多窗口隔离、守卫拒绝、entryKey 生命周期、业务可用性、模态即时阻断和目录 HMR |
| 页面懒加载 | 启动不执行未访问页面工厂/订阅/注册，首次进入才加载，离开解绑、重入复用声明并重新注册，并发加载合并、失败重试、快速切页迟到结果拒绝、缓存版本失效、多窗口实例隔离、全目录 CI 冲突校验与冷/热路径性能测量 |
| 状态栏订阅 | 启动与当前页面状态独立、未加载页面不阻止启动 Ready、查看详情不触发加载、页面失败可见、启动阶段实时推进、快速完成后详情可查、首次/晚订阅无丢更新、旧版本拒绝、部分失败与重试、连接中断；隐藏/卸载时初始化继续且重新显示恢复最新状态；多窗口隔离及布局渲染失败不影响注册 |
| NSH 适配 | 真实解析/标签、when/repeat/passThrough、解绑次数、注册 true/false/异常/迟到/未决、VM 销毁清理能力；mock 与真实契约分开 |
| 原生旅程 | 冷启动首次输入、隐藏导航控件、路由切换、输入框/IME/多键盘布局、模态、长按、多窗口、前后台 Global、占用失败与释放重试、窗口重开、HMR 与完整 VM reload |
| 平台边界 | macOS、Windows、Linux 各自记录；Linux portal 授权/取消/实际键位，非桌面 Global Unsupported，不能用本机通过代表三端可用 |

同时检查一次按键仅执行一次、不可用时不执行业务及原生消费边界；不能把单元测试 handler 返回值当作透传验收。全局测试使用隔离应用身份，测试配置选择键位，结束清理所有注册，避免污染用户键位。

crate 契约使用 `cargo test --locked -p usher-shortcuts`，工作区回归执行 `cargo test --locked --workspace`；TS 接入执行 `nsh check` 及既有 UI/真实宿主测试流程，新增测试进入 CI。统一报告命令为 `python3 scripts/test.py shortcuts`（核心）与 `python3 scripts/test.py shortcuts-app`（应用索引、所有已登记模块和 NSH 解析）；已纳入 CI，核心还进入既有七目标编译矩阵。实际命令、平台、结果与剩余缺口只写入 todo.md，不在本设计登记完成状态。

## 9. 本地执行边界

`ShortcutState.attach` 接受真实 Router opening 的 scope，返回独立快捷键 Store 身份。替换执行器会关闭旧服务；旧 scope 的回执或清理不能修改新服务。窗口关闭与 VM Local 清理由桥接收敛，TS 持有的每个 keymap 句柄独立解绑。注册调用成功仅表示 NSH 本地 API 已安装，不声称 OS 同步消费表已确认。

`ModuleLoader` 在一个 VM 中共享不可变索引，每个 OS/模块成功声明缓存一次，锁内串行执行工厂合并并发请求；失败不缓存。索引版本被实例固定，重新装配新索引创建新缓存。工厂仅构造小型内嵌声明，不执行文件、网络或业务 I/O。页面使用 beginPage/finishPage 两步加载票据；finish 前再次核对当前 entry。卸载、重试及快速切页不接受旧票据。

本地重试策略由 `ui/config/shortcuts/runtime.json` 提供：一次安装尝试，不安排自动计时重试；公开重试动作开启新的安装代次，释放失败必须继续持有旧资源直到释放回执成功。NSH VM 的 keymap 表位于根节点，同 VM 多窗口必须按当前焦点提交有效表，并在回调再次检查窗口拥有权。焦点或可用性变化到原生表更新之间仍可能消费一次旧键；回调会拒绝过期业务执行，不能承诺同步透传。

`shortcuts-ui` 使用可控 transport/backend 验证控制器；`shortcuts-headless` 使用隔离 Rust fixture host 走真实桥接；`shortcuts-macos` 额外验证焦点切换和完整 VM reload。测试工厂只编译进 `tests/e2e/shortcuts/host`，配置与产品目录隔离。状态展示依赖消息目录和布局只读契约；未访问模块仅展示索引，详情不执行工厂。

## 10. 启动全局快捷键与业务接入

| 业务模块 | 默认全局键位 | 动作 |
| --- | --- | --- |
| window | Ctrl+Alt+Shift+S | 聚焦主窗口，通过其 Router 导航到设置；仍执行已有路由守卫 |
| appearance | Ctrl+Alt+Shift+T | 通过设置事务切换本应用浅色／深色；System 从 effectiveScheme 推导下一状态，不更改操作系统主题 |

macOS 的 Alt 对应 Option。这两个组合为项目默认值，定义在 ui/config/shortcuts/window.json 和 appearance.json，均由轻量索引登记。选择依据：Apple 的 [Command-Comma](https://support.apple.com/en-us/102650) 是当前前台应用设置入口；[VS Code 的主题选择](https://code.visualstudio.com/docs/reference/default-keybindings) 使用应用内组合序列；[Raycast](https://manual.raycast.com/keyboard-shortcuts) 为命令提供用户自定义全局热键。没有可直接照搬的统一全局设置／主题标准，因此不全局占用 Command-Comma，也不把应用内多段键当作系统热键。默认键仍可能被其他程序占用，失败状态不得误称已生效。

NSH 全局 API 没有本地编辑判断、透传开关和重复事件标志。全局声明要求 editing=always、repeat=true、passThrough=false：接受后端发出的每次回调，不承诺过滤长按重复；其余策略在加载时拒绝。键位同时经 NSH 系统与本地解析器校验，统一标准文本用于 Global/Application/Page 冲突检测。

GlobalService 属于进程，不按设置目录或窗口创建。每个 VM 通过 Local 租约申请执行权；其他 VM 仅订阅共享状态，不能执行注册或确认。单个 VM 的 TS 执行器不归任何组件拥有，原生订阅在渲染 owner 之外建立，状态栏和主窗口业务适配重挂载不重注册系统键。原生注册 Promise 串行结算，返回 true 才标为 registered；false 为拒绝，异常单独报告。安装未决时保留独占状态，显式重试排队，不将超时解释为取消。回执传输失败保留待确认结果及剩余效果，避免重复安装或遗漏后续键。

业务分发仅交给存活的主窗口适配；路由和主题状态仍由原有模块写入。主窗口设置未同步或处于重挂载间隙时，每类动作最多暂存一个待执行意图；新处理器接管后消费，旧处理器清理不得撤销新处理器。全局注册健康只表示系统注册确认，业务失败仍由所属 Router/设置状态报告。

VM 结束先撤销租约和旧票据。NSH 0.5 macOS Surface::stop 清空旧命令并释放其全局注册，替换 VM 可在该宿主契约下建立新代次；其他平台尚未确认 teardown 时，未决资源保留 recovering，拒绝盲目接管。正常 unregister 的公开确认强度为 API 接受命令，不是额外的 OS 释放回执；后续注册仍必须得到系统成功结果。窗口关闭只清理 Local，Global 随 VM/进程宿主释放。

窗口状态与 Global 状态分别发布，UI 只组合只读投影：保留窗口原始阶段作为 window，startup 仅作为组合摘要，详情分别展示 window/global；全局失败／恢复参与启动摘要，但不禁用已可用的页面或窗口按键。无全局声明显示 notRequested，非桌面显示 unsupported。运行期改键界面、跨平台恢复确认和权限交互仍按独立批次推进。


状态栏默认入口采用组件库小号 ghost Button，显示键盘图标与一行启动摘要；页面状态移入 Tooltip 和 Popover 详情，避免未配置页面占用底栏。异常状态使用警示图标与本地化检查提示，完整启动／页面摘要保留在无障碍名称中。交互状态复用主题 buttonLook，尺寸、文字、颜色与间距来自主题 token；详情和异步重试继续使用组件库及既有适配。
