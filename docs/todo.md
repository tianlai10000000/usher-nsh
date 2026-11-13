# 迭代批次与完成状态

更新日期：2026-11-13。本文件是项目任务、优先级、完成状态、验收缺口与执行证据的唯一维护入口。`docs/` 其他文档只维护模块架构、接口、设计约束和验证方法；README 说明实际产品能力。原 `scaffolding.md` 只保留入口，原主题与状态审计已完整迁入本文件。

## 维护规则

- **P0**：发布前必须处置或明确决定的正确性、安装安全问题；**P1**：当前体验、功能闭环和关键验证；**P2**：平台扩展、能力完善与性能基线；**P3**：后续扩展。优先级不替代依赖关系、用户授权或此前暂缓决定。
- 状态统一为 **已完成、部分完成、待实施、待验收、受阻、暂缓、待决策**。已完成仅指该行约定的范围；受阻须写明解除条件，暂缓须保留原决定。文档归档不构成新的代码验收。
- 迭代开始时选择一个有授权且前置满足的批次；记录范围与状态。完成后在该批次回写变更、实际平台、命令、结果、证据路径和剩余缺口。按范围全部完成才标记已完成，不以配置 CI、编译或 mock 通过替代原生验收。
- 保留原任务编号（01–07、SM、RT、UT/TE、LG、N、U6），避免重复计数。下表与分模块任务为当前状态；文末冻结证据保留当时的结论和测试数量，后续状态覆盖历史描述，不在证据区继续追加待办。
- 不默认重开已完成任务。新发现的问题在对应批次新增编号；设计变化同步模块文档。外部发布、真实密钥及产品取舍按各条目既定条件执行。

## 批次总览

| 批次 | 优先级 | 当前状态 | 下一步 / 前置 |
| --- | --- | --- | --- |
| [U6-01 / U6-02 安装保护](#updates) | P0 | 暂缓；缺口已复现 | 用户此前决定后续处理；实施时先确定恢复和架构校验方案 |
| [TH2 Select 退出动画](#theme) | P1 | 受阻 | 包含上游 #1857 的兼容 NSH / CLI 版本；当前正式版 0.5.0 缺接口 |
| [TH3 / 07.05 聚焦可见性](#theme) | P1 | 待实施 | 先确定屏幕色偏与真实参照面，再修正并验收 |
| [N1–N4 应用内通知](#notify) | P1 | 待实施 | 按核心 → 桥接 → 展示 → 业务串行；N3 先核验 ToastStack |
| [03.03 / 03.06 设置和国际化](#settings) | P1 | 部分完成 | 补写入生命周期、共享文案契约与原生入口 |
| [04 Shell 与 AppKit](#shell) | P1 | 部分完成 | 补真实交互、焦点、系统字号和资源生命周期 |
| [05 / LG 日志与导出](#logging) | P1 | 部分完成 | 门面和诊断页已完成；补产物旅程与故障验收 |
| [RT4 路由扩展](#routing) | P1 | 部分完成 | 本机 URL / VM reload 已验；其他宿主、进程重启和 iOS 手势待补 |
| [UT2 / TE1 / TE2 测试闭环](#testing) | P1 | 部分完成 | 补跨语言 fixture、完整真实旅程与远端 runner |
| [U6-03–U6-06 发布验收](#updates) | P1 | 暂缓 / 待决策 | 保留此前决定；正式发布须另获授权 |
| [01 / SM6 跨平台](#platform) | P2 | 部分完成 | 工具链 → 应用构建 → 宿主 → CI → 发布旅程 |
| [02 系统托盘](#platform) | P2 | 待实施 | 先共享菜单模型，再三端宿主及生命周期 |
| [06.01 / 06.07 版本和公告](#release) | P2 | 部分完成 / 待实施 | 统一构建身份；changelog 单一来源 |
| [07.03 / 07.04 性能](#testing) | P2 | 待实施 | 固定输入、发布配置与同规格 runner |
| [N5 / 06.08 消息中心](#notify) | P2 | 待实施 | 应用内通知闭环后独立接收、持久化和对账 |
| [N6 系统通知](#notify) | P3 | 待实施 | 各平台 API 与成本调研后推进 |
| [TH1 / SM1–SM5 / RT1–RT3 / UT1 / LG1–LG6 已交付基础](#completed) | P1 | 已完成（限已验范围） | 证据保留；剩余平台和发布项不计入完成 |
| DOC1 文档归并 | P1 | 已完成 | 仅文档迁移、链接检查；不计作产品验收 |

<a id="theme"></a>

## 主题与交互动效

| 编号 | 优先级 | 状态 | 范围、前置与完成条件 |
| --- | --- | --- | --- |
| TH1 / 03.07 | P1 | 已完成（生产已用组件、本机范围） | Gallery 部件 finish、尺寸、Switch 弹簧、Slider 悬停、Progress 扫动、Popover / Tooltip 进出、Sheet 抽屉及减少动效。历史验证：nsh check、Rust 241、UI 44、隔离 macOS 动效 2 项通过；静态与动态证据见文末主题审计。未使用的组件不新增空壳适配。 |
| TH2 | P1 | 受阻 | 官方 PR #1857 / `10121ce891433bd168f0fa6ec4e7ad896bf6740e` 已提供 Select 起止样式，0.5.0 尚未包含。可用兼容版本到位后同步 CLI / Rust / 类型；用 popupMotion 的 style / starting / ending 替换 createSelectMotion，移除 onOpen / onClose 动画钩子。覆盖选择、Escape、外部点击、退出中重开、减少动效和原生中间帧。不得只补类型声明或延迟业务 open。 |
| TH3 / 07.05 | P1 | 待实施 | 现有 token 对比度测试通过，但 macOS 深色及 Primer 浅色的实测环未达 3:1。用已知 outlineColor 的纯色样例确认压暗来源，参照页面 / 卡片 / 面板真实像素，修正推导并统一偏移；七主题浅深色截图断言，再分别复核 Windows / Linux。旧像素测量与不确定性保留于原 07.05 证据。 |
| TH4 / 03.07 | P2 | 待验收 / 按需实施 | 全主题、触屏与其他宿主的部件和动效验收；标签条、分段、分体按钮等结构仅在真实产品引入时适配。不能把 TH1 视为整个 NSH 库或跨平台像素一致通过。 |

<a id="settings"></a>

## 状态、设置与国际化

设计：[状态管理](state-management.md)、[交互设计](interaction-design.md)。

| 编号 | 优先级 | 状态 | 下一步与完成条件 |
| --- | --- | --- | --- |
| 03.02 / SM6 | P2 | 部分完成 | 共享状态迁移已经完成；其余缺口合并到平台批次，避免重复建立 Store 或重做 SM1–SM5。 |
| 03.03 | P1 | 部分完成 | 补合并写入 Worker、退出 / 移动后台限时刷盘及各平台路径。更新 auto_check 和日志三开关已有持久化，不再列作未实现；托盘等新偏好随模块接入，覆盖旧数据、保存失败与重试。 |
| 03.06 | P1 | 部分完成 | 补 TSX/Rust 文案参数契约、原生菜单和托盘文案、独立格式地区、日期时间 / 复数规则，以及长译文和系统字号。已有 zh-Hans / en 与系统回退保留。 |

<a id="routing"></a>

## 路由与窗口服务

设计：[路由](routing.md)、[状态管理](state-management.md)。

| 编号 | 优先级 | 状态 | 下一步与完成条件 |
| --- | --- | --- | --- |
| RT4 / 04.13 | P1 | 部分完成 | 目录迁移、深链接、系统返回、macOS 发布包 URL、源码 HMR 与完整 Cmd+R VM reload 已验。后续按可用宿主推进 Windows / Linux / Android / iOS 的返回、目录升级及发布回归；普通跨进程重启与持久偏好恢复单独验证，不能用 VM reload 替代。 |
| RT4-GESTURE | P2 | 待实施 | iOS 交互式滑动返回：处理原生栈、异步守卫和唯一 Rust 历史的一致性；修复模拟器环境不等于实现手势。 |

<a id="shell"></a>

## Shell、AppKit 与背景资源

| 编号 | 优先级 | 状态 | 下一步与完成条件 |
| --- | --- | --- | --- |
| 04.04 | P1 | 部分完成 | 路由参数与系统返回核心由 RT4 覆盖；补路由可用条件、动态标题、产品所需更新页面 / 原生动作，验证移动安全区、系统字号及浮层优先关闭。 |
| 04.05 | P1 | 待验收 / 按需实施 | AppKit 控件族切换的草稿保留、焦点恢复、浮层释放与失败回退；更新任务状态栏投影按实际产品需求接入。 |
| 04.06 / 04.07 | P1 | 部分完成 | 更新和诊断设置入口、更新浮层、Sheet 动效已接入；补分组卡片 / 原生列表观感、窄窗折行、键盘顺序、系统字号、标题栏拖动 / 全屏、二级菜单伸缩及 AppKit 窄菜单。账户入口仍为禁用契约，真实账户能力独立立项。 |
| 04.08 | P1 | 部分完成 | 背景选择取消闭环已原生验收；补系统选择器实际选图、应用重启后显示及其他平台导入 / 资源清理。 |
| 04.09 | P2 | 待验收 | 内容透明度已实现并完成本机截图，补其他平台真实宿主显示；保留文字、控件和阴影。 |

<a id="logging"></a>

## 日志与诊断

设计：[日志](log.md)。

| 编号 | 优先级 | 状态 | 下一步与完成条件 |
| --- | --- | --- | --- |
| LG1–LG6 / 05.04 / 05.05 | P1 | 已完成（既定范围） | 稳定门面、私有后端、应用 / 更新调用迁移、分层测试、三个持久化开关与诊断页；两个不可用采集项继续禁用。 |
| 05.01 / 05.02 | P1 | 部分完成 | 补真实盘满、应用 Quit 尾部完整性、强杀边界与非 macOS 宿主输出；不得把正常 atexit 冲刷测试当作强杀保证。 |
| 05.03 | P1 | 部分完成 | 导出规则与应用集成已有；补真实导出文件夹打开 / 归档内容、长任务旅程、桌面保存对话框与 Android / iOS 系统分享入口。 |
| LG7 | P3 | 待实施（扩展） | 终端 / 内存真实采集未接入；只有产品需要时定义采集契约、成本和隐私边界，合成事件测试不视为采集实现。 |

<a id="updates"></a>

## 更新遗留问题

设计：[更新](update.md)。2026-11-13 用户决定先记录、后续处理；本次文档整理不启动修复或正式发布。批次 3–5 已交付，批次 6 部分完成。真实 shell-update 末轮 11 项通过包含 8 项功能和 3 项边界探测；边界探测通过表示成功复现缺口。

| 编号 | 优先级 | 状态 | 下一步与完成条件 |
| --- | --- | --- | --- |
| U6-01 / V5 | P0 | 暂缓 | 确定安装器或外部自动恢复方案；rename 间隙终止后可恢复启动、保留设置并清理备份，人工恢复不能算通过。 |
| U6-02 / V13 | P0 | 暂缓 | 发布产物架构核对与安装前防御；签名有效但内部架构错误的归档须在替换前拒绝，旧应用不变且可启动。 |
| U6-03 / V5 | P1 | 暂缓（待验收） | 真实浏览器下载、quarantine、首次打开及更新后启动；记录 ad hoc 的实际体验。Apple 签名 / 公证继续按 D6 暂缓。 |
| U6-04 / V8 | P1 | 暂缓（待验收） | 管理员所有权与 ACL 组合下的 canInstall、手动升级及失败后旧版可用性。当前账户可写 /Applications 的本地验收不替代这些条件。 |
| U6-05 / V9 | P1 | 待决策 | 确认是否接受 AppKit 更新为 standard 丢失原生控件能力，再决定清单 / 变体分发。技术转换成功不替代产品决定。 |
| U6-06 / 06.03 / 06.06 | P1 | 暂缓（待授权与验收） | 获准真实发布后核对密钥配对、匿名更新源、release 环境、草稿转正式、latest、构建来源证明、实际下载和双路径升级；代码包签名、换根拒绝与密钥轮换须补远端流水线和分发验证。 |
| 06.02 / 06.04 / 06.05 | P1 | 部分完成 | 双路径、桥接、UI、本地代码包及整包替换已验；剩余发布项归 U6，托盘入口归 02，不重复新增更新器。 |

<a id="release"></a>

## 发布与版本元数据

设计：[部署与发布](deploy.md)、[更新](update.md)。

| 编号 | 优先级 | 状态 | 下一步与完成条件 |
| --- | --- | --- | --- |
| REL1 / U6-06 | P1 | 进行中（2026-11-14 已授权） | 用户授权 GitHub 真实发布及本地双路径更新验收，并已将仓库改为 public。匿名仓库 API 已验证；三项仓库 Secret 存在，release 环境已创建；配置公钥与本地公钥文件、证书发布公钥匹配，证书到期日为 2027-11-12。首次 0.1.0 发布与更高正式版本的实际升级仍待执行。 |
| REL2 | P2 | 暂缓 | Developer ID、公证和 stapler 真正启用后的分发验证；自签名钥匙串实验和 lint 不替代 Apple 公证。Secrets 范围收敛及 release 环境配置与 REL1 一并核查。 |
| REL3 / 06.03 | P1 | 待实施 | 客户端吊销列表的接入与分发：发布侧 verify --revocations 已有，不能代替已安装客户端拒绝吊销包。确定可信下发、更新和离线失败策略，并验收密钥轮换 / 吊销的实际分发。 |

2026-11-14 发布前检查：发现 Cargo 根包版本号变化被误判为外壳变化，导致正常补丁发布无法产出代码包；修正为只忽略根包发布身份，依赖与其他配置仍受门禁保护。脚本测试 141 项通过，包含三处版本文件同步升级、依赖变化与损坏身份回归；`git diff --check` 通过。同步修正 0.1.0 CHANGELOG 中尚未接入更新的过时说明。原提交 aa936a6 的 [Release 演练 #2](https://github.com/tianlai10000000/usher-nsh/actions/runs/43459471107) 门禁通过、release_ready=true，macOS 作业仍在排队，尚不计为构建或签名通过。
| 06.01 | P2 | 部分完成 | 统一运行构建身份、关于弹窗、包元数据与 Release 附件；补版本一致性门禁、说明生成、升级公告和首次升级提示。 |
| 06.07 | P2 | 待实施 | 仓库目标版本 changelog 为唯一公告源；发现新版和首次升级入口，覆盖版本匹配、已读去重、缺失、失败与重试。 |

<a id="notify"></a>

## Notify 与消息系统

设计：[Notify](notify.md)。N0 设计完成，但未运行原生 ToastStack 组合实验。N1–N4 构成首个可用闭环；N5 / N6 不计入首批完成。

| 编号 | 优先级 | 状态 | 前置、交付与验收 |
| --- | --- | --- | --- |
| N0 | P1 | 已完成（设计） | NSH 类型与 Slint 参考核查、模块契约；不代表通知产品已实现。 |
| N1 | P1 | 待实施 | usher-notify 与配置；确定时长、容量、预算和位置，不硬编码；独立核心的预算、去重、替换、动作与失败原子性。 |
| N2 | P1 | 待实施 | 依赖 N1；窗口服务、桥接、Rust / TS 门面、版本订阅、窗口隔离及清理。 |
| N3 | P1 | 待实施 | 依赖 N2；先核验 ToastStack 的焦点暂停、模态层级与安全区，再接 Host、主题和国际化；实测计时、堆叠、HMR 无双时钟 owner，首批接受重挂载重新计时。组件缺口按交互设计规范处理。 |
| N4 | P1 | 待实施 | 依赖 N3；先诊断导出和适用的显式设置操作；更新接入前同步更新设计、明确目标窗口和满载 effect 完成策略，避免重复提示。 |
| N5 / 06.08 | P2 | 待实施 | 消息中心历史、已读、持久化与对账；实时接收另验送达、断线重连、漏收补取、去重和订阅释放。更新公告仍读 changelog，不复制内容源。 |
| N6 | P3 | 待实施 | 各平台 API / 依赖成本调研、权限、系统投递、激活、降级及关闭排空。 |

<a id="platform"></a>

## 平台、托盘与 SM6

| 编号 | 优先级 | 状态 | 下一步与完成条件 |
| --- | --- | --- | --- |
| 01.03 / 01.04 / SM6 | P2 | 部分完成 | 四库七目标和应用 Apple 四目标编译已验；Windows / Linux / Android 完整应用工具链与构建、Intel Mac 和各端宿主待验。iOS 当前 CoreSimulator 版本不匹配 / Xcode 首次启动未就绪，环境就绪后使用专用模拟器；移动发布前补真机签名安装。 |
| SM6 / 03.13 | P2 | 部分完成 | 按完整应用编译 → 宿主 → 远端 CI → 分发与升级推进。本机 macOS 发布 URL、VM reload 已通过；原 06ed67e 成本对照是历史快照，需要新结论时按同条件重测。 |
| 01.05 | P2 | 部分完成 | 源码 HMR 与 VM reload 已验；Rust / 宿主重建的偏好恢复、各目标开发服务器连接和自维护宿主待验。 |
| 02.02 | P2 | 待实施 | 共享托盘菜单及显示 / 隐藏、设置、更新、退出语义动作；本地化、关闭到托盘与失效恢复。 |
| 02.03 / 02.04 | P2 | 待实施 | macOS 菜单和 Dock 唤起、Windows Explorer 重启恢复、Linux KSNI / D-Bus 与无托盘降级；取消、隐藏、重复退出、句柄释放和 HMR 单实例。 |
| 02.05 | P2 | 待验收 / 按需实施 | Windows MSIX 身份；移动返回、前后台恢复、应用内菜单和商店入口。与 RT4 共用返回验收，不重复计数。 |

<a id="testing"></a>

## 测试与性能

设计和运行入口：[测试架构](testing.md)。

| 编号 | 优先级 | 状态 | 下一步与完成条件 |
| --- | --- | --- | --- |
| UT2 / 07.07 | P1 | 部分完成 | 真实桥接、投影、守卫和失败重试已有；补统一跨语言 DTO fixture，覆盖生产协议与兼容边界。 |
| TE1 / 07.08 | P1 | 部分完成 | 隔离、报告、macOS 多窗口 / HMR / VM reload 已验；补普通跨进程重启、其他平台、远端解锁图形 runner 的完整运行。 |
| TE2 / 07.09 | P1 | 部分完成 | 按测试设计 E2E-01–07 补真实鼠标键盘全产品旅程、长导出任务及各平台发布产物；更新的特定重启成功不等于全部旅程通过。 |
| 07.01 / 07.02 | P1 | 部分完成 | 新增领域随实现交付单测 / 契约 / 应用集成与真实旅程；五端烟测、签名与产物验收归所属批次，不另计重复完成。 |
| 07.03 | P2 | 待实施 | benches 中固定派发、快照投影、列表转换和日志格式化输入并保存测量。 |
| 07.04 | P2 | 待实施 | 发布宿主的冷启动、交互、帧时间、CPU、峰值内存、托盘长驻 / 恢复、移动前后台和释放；同规格 runner 保存原始样本及 P50/P95，再定义门禁。 |

<a id="completed"></a>

## 已完成基础与证据使用

SM1–SM4 的独立核心、设置服务、桥接投影、全量现有共享数据迁移，以及 RT1–RT3 的核心 / 集中页面 / 唯一导航入口已交付；SM5 的第二领域复用已交付，其早期取消 / 交接设计由更新 A 方案替代，不按历史接口继续实施。UT1 和 07.10 通用测试支持已交付。01.01 / 01.02、02.01、03.01 / 03.04 / 03.05、04.01–04.03 保留原完成范围。LG1–LG6、TH1、异步按钮反馈与原生选择取消的证据保留如下。

## DOC1 文档归并验收（2026-11-13）

已按 P0–P3 建立当前批次表，保留原任务编号、依赖、完成范围及更新遗留问题的暂缓决定。完整迁入 13 组原文证据（含全部主题 / 状态审计），在忽略链接目标变更后逐组核对正文一致。检查 docs 与根索引共 16 份 Markdown 的本地文件及锚点引用，0 错误；`git diff --check` 通过。本批只调整文档，未运行或重新声明任何产品测试结果。

## 历史证据（冻结，非当前任务表）

以下内容从原文迁入，保留来源、日期、命令、失败轮次、通过数量与限制，以便复核。它们描述当时的源码快照，可能含后来已被覆盖的状态或设计；当前设计以模块文档为准，当前执行计划以本文件上方批次为准。仅迁移相对链接和标题层级，不把旧测试数当作本次复验。后续证据应附在对应活动批次下，并标明日期与验证范围。

<a id="evidence-theme-audit-0"></a>

<details>
<summary>docs/theme-audit.md · 迁入记录 1（冻结证据）</summary>


<a id="evidence-theme-audit-0-nsh-gallery-主题组件审计"></a>

#### NSH Gallery 主题组件审计


<a id="evidence-theme-audit-0-范围与依据"></a>

#### 范围与依据

2026-11-13 检查应用 `ui/` 的全部 NSH 控件入口及共享样式。对照 [NSH Gallery UI](https://github.com/longbridge/nsh/tree/adb0f210793fdda3e3443b7183ef5edd458a762d/examples/nsh-gallery-ui)，固定提交 `adb0f210793fdda3e3443b7183ef5edd458a762d`，与现有主题快照一致；未升级框架版本。官网示例使用 Gallery 的 `look/`，`nsh/base` 仅提供行为和部件接口，不自带 Gallery 外观。

根因是同步内容不完整：原映射包含通用颜色、尺寸和部分按钮 finish，但缺少 `track`、`fill`、`knob`、`knobOn`、`thumb`、`menu`；部件又复用了文字色、边框色和窗口阴影，导致有主题配色却没有相应的控件表面。


<a id="evidence-theme-audit-0-已使用组件逐项结果"></a>

#### 已使用组件逐项结果

| 组件 / 入口 | 对照源码 | 结果 |
| --- | --- | --- |
| Switch：常规、诊断、更新设置 | `look/switch.ts`、`look/switch-geometry.ts` | 开关轨道使用 `track` / primary 和对应边框、内阴影；滑块使用 `knob` / `knobOn` 的填充、渐变、描边和阴影。移除文字对比色作为滑块颜色的错误映射。边框计入几何，桌面 28 × 16、滑块 12，移动距离 12，两端间距一致。 |
| Button / ThemeButton / AsyncButton | `look/button.ts` | 普通选中按钮使用 `finish.on` 与正文色，主按钮使用强调色表面；保留现有内部加载、禁用与 Promise 生命周期。未选中、hover、pressed、focus 继续使用共享样式。默认强调色的文字读取 `primary-foreground`；自定义强调色继续计算对比色。 |
| 标题栏、菜单、窄栏图标按钮 | `look/button.ts`、`look/tool.ts` | 已有透明、悬停、按下及焦点样式；导航选中保留应用的中性菜单表面。不同于 Gallery 工具栏 toggle 的业务角色，不统一改成凸起按钮。 |
| Input / ThemeInput | `look/field.tsx` | 填充改读 `field`，边框改读 `input`；保留 field 阴影、hover 和焦点环。 |
| Select：诊断级别、窄屏设置导航 | `look/listbox.tsx`、`popup.tsx` | 触发器复用 field；列表使用 `finish.menu` 的填充、边框和浮层阴影，圆角单独取 popup token；保留框架键盘高亮与选择逻辑。 |
| Tabs / Tab：设置分类 | `look/tabs.tsx` | 选中指示器补齐 `finish.thumb` 的渐变、边框与阴影。保留应用的纵向导航布局，不套用 Gallery 横向分段控件的背景轨道。 |
| Slider：图片不透明度、内容透明度 | `slider.ts` | 接入 track 内描边、fill 渐变和描边、knob 表面；Gallery 桌面轨道 6、滑块 14、触控带 20，触屏分别为 8、20、32；避免直接用表单高度作为触控带。 |
| Progress：更新下载 | `look/meter.ts` | 与 Slider 共用轨道和填充表面，增加圆角与裁切；确定进度以 180 ms 曲线滑动；不确定进度使用三分之一宽的色带、800 ms 原生往返扫动，减少动效时停在中间。 |
| Popover：更新浮层 | `look/overlay.ts`、`popup.tsx` | 接入共享 popup 表面，不再使用窗口背景和窗口阴影；宽度、避让、关闭行为不变。 |
| Tooltip：按钮提示 | `look/overlay.ts` | 使用 popover / popover-foreground、独立 tooltip 圆角；移除官网 tooltip 没有的窗口阴影。保留现有屏幕边界约束。 |
| Sheet：侧栏抽屉 | `look/overlay.ts`、NSH Sheet | 使用公开的左侧 Sheet 替换无动画 Dialog，框架负责滑入 / 滑出、遮罩淡入 / 淡出、Escape 及减少动效。保留应用菜单内容、尺寸和 scrim。 |
| Avatar：账户入口 | `avatar.ts` | 已有圆形图片和主题化 fallback；当前无账户时显示账户图标。Gallery 的姓名哈希配色是身份展示策略，未引入到无账户入口。 |
| Scroll / Scrollbar：内容区 | `scrollbar-theme.ts` | macOS 自绘滚动条的宽度、缩进、悬停颜色和时序改由主题提供；Gallery 使用 16 点轨道、6 点滑块、hover 8 点，idle / enter / exit 为 2000 / 300 / 500 ms。其余平台继续使用既有系统滚动条策略。 |
| View 分组、卡片、行分隔 | `look/surface.tsx` | 卡片补齐已有 card 阴影；分隔线使用 `line` 而非外边框颜色。内容透明度继续只改变填充。 |
| Text、Image、Canvas、原生图标 / AppKit | 基础绘制与宿主接口 | 按应用文字、背景图片和进度环职责保留；不是缺少 Gallery finish 的独立控件。AppKit 继续由系统绘制。 |

Checkbox、RadioGroup、Combobox、Menu、Table、Tree、DatePicker、Toast 等尚未在生产 `ui/` 引入的组件，不新增空壳样式或演示页面。引入时应在同一映射机制下补充对应部件，不能据此表格声称整个 NSH 组件库已支持。


<a id="evidence-theme-audit-0-实施边界"></a>

#### 实施边界

- 数据入口仍为 `ui/theme/sources/nsh-gallery.json` → `scripts/sync-gallery-theme.py` → `ui/theme/presets/nsh-gallery.json`，新增尺寸保留上游符号出处。`NOTICE` 的既有 Gallery 署名适用。
- 根主题提供语义角色及无装饰 finish 回退，其他主题无需补齐所有新字段。Rust 注册表继续解析继承和引用；启动时 TS 也可读取根主题的角色别名。调色板编辑器更新派生部件角色，避免保留根主题的旧填充。
- 自定义强调色会去掉固定强调色渐变并替换其边缘色，保留阴影。不会把开关滑块改成文字对比色，也不会被 Gallery 的绿色渐变盖住。
- 部件表面与动画通过公开 NSH API 实施，不依赖 Gallery 内部包。应用焦点环继续遵循既有对比度策略。弹簧、悬停与浮层动效的接入及接口限制见下节。


<a id="evidence-theme-audit-0-验证"></a>

#### 验证

新增 `tests/ui/theme-finishes.test.tsx` 和无持久化的 `ThemeFinishes.tsx` 夹具，覆盖明暗主题部件表面、Switch 行程、选中按钮语义和强调色覆盖。已有控件尺寸测试同步断言中性选中按钮的正文色。Rust 编辑器主题测试覆盖派生 field / track / knob / popup 角色。

- `nsh check`：通过。
- `cargo test --locked --workspace`：241 项通过（含 doctest），无失败。
- `python3 scripts/test.py ui`：最新工作区完整重跑通过，13 个文件、40 项测试，无失败。首轮运行期间加载按钮实现与夹具同步发生变化，出现 4 项旧加载断言失败；该轮未记为通过，最终结果以完整重跑为准。
- macOS 原生静态验收：临时源码目录、无状态桥接的夹具，1 项通过；已查看浅色、深色截图，确认开关浅色滑块及描边、轨道、渐变和浮层表面。截图在 `target/theme-audit/light.png`、`target/theme-audit/dark.png`。首轮受沙箱本地服务器限制，获准以隔离项目重跑成功。

原生截图只验收静态样例，不替代完整跨平台、交互状态和动效验收。


<a id="evidence-theme-audit-0-动效补齐2026-11-13"></a>

#### 动效补齐（2026-11-13）

| 行为 | 上游依据与接入 |
| --- | --- |
| Switch | `theme.ts motion.move`：stiffness 504、damping 38。`createSwitchLook` 每个挂载实例只创建一次原生 AnimatedValue，滑块和轨道颜色跟随同一弹簧；快速反向切换不先 stop，不丢失当前位置和速度。卸载 stop，减少动效 set 到目标；不会在 reactive style getter 中创建弹簧。 |
| Tabs | NSH `Tabs` 内核已经按 `spring_move` 移动指示器，布局变动同步落位；保留原生实现，不叠加 JS 动画。 |
| Button / 图标按钮 | 保留 Gallery 的 150 ms ease-out 过渡和按下缩放；主按钮 hover / active 的 0.94 / 0.88 改为主题数据。减少动效取消缩放和过渡。 |
| Slider | 接入 Gallery 的 hover brightness 105%、active 95%、disabled 100% 与状态过渡；拖动直接跟随指针，不给拖动值添加弹簧延迟。 |
| Popover / Tooltip | `popup.tsx` 的 150 ms、`cubic-bezier(0.16,1,0.3,1)`、scale 0.96 → 1、位移 4 / 3 点和 opacity 0 → 1。通过 starting / ending style 接入 NSH 的浮层存活与命中管理；关闭反向回到起始样式。更新浮层使用实际放置方向更新变换原点。 |
| Select | 以公开 onOpen / onClose 钩子和 createMotion 补齐桌面进入动画；移动端仍由原生 sheet 执行动画。当前版本 Select 没有 popupEndingStyle，不通过延迟业务 open 或替换内部菜单伪造退出动画，关闭保持框架默认行为。 |
| Progress | `look/meter.ts` 的确定进度曲线、原生 sweep。只在可见且不确定时运行循环，状态变为确定、减少动效或卸载时停止；无关快照不会反复重启循环。宽度变化由原生 AnimatedValue 更新扫动距离。 |
| 抽屉 | 使用 NSH Sheet 的边缘滑入和遮罩动画，保留框架关闭、快速重开及减少动效契约。 |

所有新增参数来自根主题回退和 Gallery 映射，保留上游符号出处。`ui/theme/motion.ts` 只管理表现层动画；共享业务状态不迁入动画对象。这里没有逐帧 JS 回调或自建计时动画，测试夹具的采样定时器只用于验收。

验证入口：

```sh
nsh check
python3 scripts/test.py ui
python3 scripts/native_theme_acceptance.py --junit target/test-reports/native-theme-motion.xml
```

原生验收已加入 `tests/config.json` 的 `e2e-macos` 套件，由既有原生 CI 入口执行。脚本创建临时源码目录，不加载应用 Rust 设置桥接，结束后清理；截图保存到报告旁的 `native-theme-motion-screenshots/`。本轮 `nsh check` 与 Rust 工作区 241 项检查通过。macOS 正常 / 减少动效 2 项原生验收通过；已检查原生截图，确认确定进度从左端填充至 75%，拖动后更新至 50%，扫动切换不残留位移。原生弹簧采样包含中间位置、轻微回弹和最终收敛；减少动效的四个样本均直接等于目标值。最新完整 UI 回归通过：14 个文件、44 项测试，无失败、无未知样式类警告。

验收发现并修复了两项问题：Sheet 面板的测试标识需要放在其可见内容；Progress 在扫动和确定进度间切换时，不应把同一属性从原生 AnimatedNode 换为 CSS translate，因此两种模式统一使用同一原生位移通道。该修正保持上游的 180 ms 缓动曲线，并避免旧绑定残留。


<a id="evidence-theme-audit-0-select-退出动画的上游复核2026-11-13"></a>

#### Select 退出动画的上游复核（2026-11-13）

官方已在 [PR #1857](https://github.com/longbridge/nsh/pull/1857) 提供正式公开接口，提交为 `10121ce891433bd168f0fa6ec4e7ad896bf6740e`，合入时间为 2026-11-13 06:02 UTC。此次核查的 main 为 `f157b16f22dab29d5c9e8cebf38baf8e5fe37536`。这项支持晚于项目采用的 [v0.5.0](https://github.com/longbridge/nsh/releases/tag/v0.5.0)，核查时最新正式发布仍为该版本；本地 CLI 为 `0.5.0 (f4d68f7)`，Rust NSH 依赖也固定为 `=0.5.0`。

新增的 `popupStartingClass` / `popupStartingStyle` 和 `popupEndingClass` / `popupEndingStyle` 同时适用于 Select、Combobox、DatePicker 的桌面下拉浮层。官方 [Select 源码](https://github.com/longbridge/nsh/blob/10121ce891433bd168f0fa6ec4e7ad896bf6740e/crates/nsh-base/src/select.rs) 和 [dropdown presence 测试](https://github.com/longbridge/nsh/blob/10121ce891433bd168f0fa6ec4e7ad896bf6740e/crates/nsh-base/tests/base/dropdown_presence.rs) 明确了以下契约：

- 关闭后保留浮层直到其 transition 完成，再触发 `onClose`；退出期间不接收点击。
- 开始关闭时即把键盘焦点还给触发器；重复 Escape 或点击外部不会重复取消。
- 退出中重新打开会收回退出过程，不被旧动画卸载；搜索结果在退出期间保留。
- 减少动效或未设置起止样式时立即关闭；触屏 Sheet 保持自己的动效。

当前 0.5.0 的发布源码、本地 Rust crate 和生成的 Select 类型均没有上述四个属性，仅修改主题或添加 TS 类型声明不能获得退出存活管理。本次未升级到未发布源码，也未改变现有 Select 行为。

待框架与 CLI 升级到包含该提交的兼容版本后，删除 `createSelectMotion` 的进入动画控制，诊断级别与紧凑设置导航的 Select 改用已有 `popupMotion(palette)`：将 `style` 合并到 `popupStyle`，`starting` 传给 `popupStartingStyle`，`ending` 传给 `popupEndingStyle`，移除现有 `onOpen` / `onClose` 动画钩子。这样直接复用现有主题的 150 ms、0.96 缩放、4 点位移和曲线，不引入新的参数或延迟业务状态。接入后须补验选择、Escape、外部点击、退出中重开与减少动效，并在 macOS 原生宿主确认退出中间帧；此前的 2 项原生动效验收不代表该未接入能力已通过。

</details>

<a id="evidence-state-audit-1"></a>

<details>
<summary>docs/state-audit.md · 迁入记录 2（冻结证据）</summary>


<a id="evidence-state-audit-1-状态管理验收审计sm6"></a>

#### 状态管理验收审计（SM6）

更新批次 3 至 5 变更说明（2026-11-13）：当前更新服务采用 A 初始化，SettingsService 创建时持有空服务，一次性 Configure；无取消 / 交接 / 恢复屏障。UpdateState 桥接与 auto_check 持久化已实施，日志拆为 usher-logging；批次 5 已接入 TSX 执行器、计时器、更新分区和菜单栏浮层。下文 SM5 / SM6 的接口、测试数量与成本为此前验收记录，不能视为本轮实现的复验结果；本轮实际结果见 [测试方案](testing.md)。


本记录核对当前已实现的状态管理与路由，不将更新产品界面、消息、账户等规划能力计入完成范围。实施清单见 [scaffolding](#evidence-scaffolding-12)，详细测试结果见 [测试方案](testing.md)。


<a id="evidence-state-audit-1-1-共享来源审核"></a>

#### 1. 共享来源审核

2026-11-12 至 13 扫描 App、ui/pages、ui/state、ui/router、ui/features、ui/layout 及 Rust 应用服务。原始命令和输出保留在 target/test-reports/sm6-state-audit.txt。

| 状态 | 唯一来源与入口 | 验证 |
| --- | --- | --- |
| 外观、语言、主题、侧栏、状态栏、背景资源元数据 | SettingsService / SettingsStore → SettingsState → state/settings；命令集中在 state/transport | 设置集成、state-ui、原有 UI、原生多窗口偏好同步 |
| 诊断等级与实时健康 | 等级写入 settings；健康和 dropped 由 diagnostic_state 发布，旧 DTO 只保留兼容读取 | 真实日志线程错误发布及订阅测试 |
| 路由、历史、面包屑 | WindowService 中唯一 WindowSnapshot；ui/router 提供导航门面，pages 注册业务页面 | router、routes-check、router-ui、实际 App E2E |
| 分类、计数、预览、操作票据、导出结果 | WindowSnapshot.session；旧 navigation 删除，页面与 Shell 借用同一投影 | 窗口集成、往返保留、真实 HMR / 关闭重开 |
| 更新状态与待执行效果 | UpdateService 的 Machine + 私有领取队列，UpdateRegistry 按宿主设置身份共享 | 同一应用消费者在两个后端执行，隔离目录、并发领取、交接及恢复屏障；未接入产品 UI |
| 消息 / 账户 | 尚无业务数据源；布局只定义契约 | 后续批次，不创建占位共享 Store |

所有设置写入经 state/transport，窗口写入经 window-transport。WindowBackdrop 直接桥接只读取平台能力和图片字节，不写偏好；二进制和 Blob URL 不属于共享业务状态。Rust 的 SETTINGS 是目录注册表，STATUS 是诊断状态发布源，LOGGER 是 I/O 资源拥有者，NEXT_STORE_ID 只分配进程身份；均不是页面级重复业务状态。


<a id="evidence-state-audit-1-2-保留的局部状态"></a>

#### 2. 保留的局部状态

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


<a id="evidence-state-audit-1-3-后端与依赖边界"></a>

#### 3. 后端与依赖边界

usher-state 无默认 feature 时无正常依赖；backend-watch 仅增加 Tokio sync 与 pin-project-lite，无 runtime、网络或计时器。usher-test-support 无依赖且仅通过 dev-dependencies 使用。更新领域 crate 仍不依赖 usher-state、Tokio 或 NSH；只有应用适配依赖两者。原更新 API 仅增加 Machine 的 Eq / PartialEq，没有改字段、事件、构造方法或存储格式。相对 06ed67e，锁文件新增的外部包为 matchit 0.9.2（运行时）和 proptest 1.11.0、rand_xorshift 0.4.0、unarray 0.1.4（仅测试）；另有三个本地 crate，无旧版本包被替换。SM5 / SM6 本身没有新增外部依赖。

状态核心 Memory / watch 契约使用同一测试调用方；SM5 的应用消费者也分别注入两个后端，验证发布、领取、停止与恢复。故障注入验证提交失败保持队列和领域状态，不能提前执行效果。路由线性参考匹配器与 matchit 的替换契约另外运行。


<a id="evidence-state-audit-1-4-平台与宿主"></a>

#### 4. 平台与宿主

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


<a id="evidence-state-audit-1-5-同条件发布构建成本"></a>

#### 5. 同条件发布构建成本

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


<a id="evidence-state-audit-1-6-完成状态与剩余边界"></a>

#### 6. 完成状态与剩余边界

SM5 应用服务交付完成。SM6 本机的迁移审计、后端替换、完整回归、原生 macOS 生命周期、七目标纯库 / 四目标应用编译和发布构建成本已完成；整体标为部分完成。缺口是 Windows / Linux / Android 完整应用工具链与构建、非本机平台宿主、远端 CI，以及分发产物与实际升级旅程（本机 ad-hoc 发布运行已由第 8 节补齐）。这里不把第三方工具链缺失归为状态库失败，也不把纯库编译替代宿主验收；现有证据可供对应平台接入时复验。


<a id="evidence-state-audit-1-7-rt4-后的回归更新2026-11-13"></a>

#### 7. RT4 后的回归更新（2026-11-13）

新增目录迁移仍由窗口 `usher-state` 一次发布，不增加第二份历史或业务共享缓存。已通过 229 项工作区测试、36 项应用集成、21 项状态 UI、11 项路由 UI、29 项应用 UI、4 项无头 E2E 和 4 项 macOS 原生验收；后者包括真实目录版本 HMR 和异步守卫返回。结果详见 [测试方案第 17 节](testing.md)。

SM1–SM5 状态不变，SM6 的完整跨平台应用 / 宿主、远端 CI 和分发 / 升级缺口仍保留；本机 ad-hoc 发布运行和 VM reload 的补充证据见下一节。第 5 节发布构建成本来自 RT4 之前的源码快照，不代表本批重新测量。


<a id="evidence-state-audit-1-8-本机发布运行vm-reload-与环境复核2026-11-13"></a>

#### 8. 本机发布运行、VM reload 与环境复核（2026-11-13）

RT4 解锁会话下 macOS arm64 ad-hoc 发布包 3 项通过：真实系统 URL 注册、外部冷启动、运行中隐藏应用收到 URL 后恢复和激活，保持同一 PID。临时命名空间和系统注册清理完成。它补齐本机 release 运行的路由验收，不等同于升级或公证分发。完整 Cmd+R VM reload 另有 1 项通过，持久偏好恢复且窗口会话重置、旧回调失效；报告见测试方案第 18、19 节。

当前源码复验通过：Rust 工作区 229 项、Python 104 项、四库七目标 28 组产物、应用库 Apple 四目标，以及 nsh check / actionlint。未重测第 5 节的包体积成本。未更改共享状态接口或持久化格式。

iOS 模拟器清单可读取，但当前 Xcode 首次启动检查返回 69，CoreSimulator 安装版 1174.9.2 低于包装器要求 1174.21，原生验收未运行；没有修改模拟器数据或安装系统组件。Windows / Linux / Android 的工具链和宿主、远端 CI 仍不可用。上一轮旧更新模拟改写 HOME，故未运行；本轮隔离改造见第 9 节，完整实际升级仍依赖更新方案批次 4、5 和真实升级产物。SM6 继续保持部分完成。


<a id="evidence-state-audit-1-9-更新启动模拟的隔离改造"></a>

#### 9. 更新启动模拟的隔离改造

已将旧 HOME 重定向替换为唯一测试应用 ID 和每例临时 data/cache/saved-state。脚本拥有应用进程，确认退出后再清理；停止失败或命名空间归属改变时保留隔离。产品设置、桥接、状态机与持久化格式没有变化。原生测试脚本复用同一链接保护模块，新增 16 项隔离 / 清理单测，Python 全部 120 项通过。真实 SIGTERM 中断后的测试应用、密钥、数据树和链接检查无残留，失败 / 中断运行未累计为通过。

本轮验证范围仍是预置 next 代码包的启动、健康晋升与拒绝 / 回滚行为，不覆盖应用主动检查、下载、整包安装、公证分发或远端 CI；SM6 保持部分完成。最终完整场景结果见测试方案第 20 节。

最终完整运行 update-sim-d8uds6dq：健康晋升、启动回滚、降级拒绝、篡改拒绝共 4 项通过，0 失败，7 项 pending；16 个唯一隔离路径全部清理，密钥、源码和临时应用也已删除。证据为 report.json、cleanup-audit.json 和逐场景应用日志。该命令已纳入手工 native_state CI，本轮只完成本机执行；实际检查 / 下载 / 安装与跨平台缺口没有变更。

</details>

<a id="evidence-testing-2"></a>

<details>
<summary>docs/testing.md · 迁入记录 3（冻结证据）</summary>


<a id="evidence-testing-2-8-分批实施与完成条件"></a>

#### 8. 分批实施与完成条件

| 批次 / 任务 | 前置 | 交付与完成条件 | 状态 |
| --- | --- | --- | --- |
| UT1 / 07.06 | SM1、RT1 | 状态与路由参考 / 真实后端契约、故障边界、fixtures、属性测试、命令与 CI 配置 | 已完成；第 13 节 |
| UT2 / 07.07 | UT1、SM2 / SM3 / RT2 | 设置 / 窗口集成、TS 投影与守卫 mock、真实订阅 / 路由桥接、生成检查已交付；RouterView 重试 / 返回错误注入已通过；统一跨语言 DTO fixture 目录待补 | 部分完成 |
| TE1 / 07.08 | SM2 / SM3；与 UT2 并行 | 无头及 macOS 原生目录隔离、同 VM 多窗口、真实 HMR、JUnit / JSON 和 CI 入口已交付；跨进程重启、其他平台及远端原生 runner 待验证 | 部分完成 |
| TE2 / 07.09 | TE1、SM4 / RT3；扩展项随 RT4 | 两项实际 App 旅程及补充桥接测试已通过；E2E-01 至 E2E-07 的范围和原生缺口见第 13 节 | 部分完成 |
| 测试支持抽取 / 07.10 | SM1 / UT1 状态部分 | 独立无业务依赖的 `usher-test-support`；仅开发依赖；状态 mock / fixtures 保留原模块；明确目录归属，工具单测、原契约、命令和 CI 验证 | 已完成，见第 10 节 |

07.01 继续跟踪现有单测覆盖，07.02 汇总跨层和平台验收，07.06 至 07.09 是它们的新设计子任务，不能重复记为两次交付。UT1 在核心接口实现时交付，UT2 在桥接实现时交付；TE1 提前验证运行条件。RT3 + SM4 的完成依赖对应旅程通过，不能因为测试框架已有命令就把页面迁移标为完成。RT4 扩展能力不阻塞既有两页整改，但深链接、HMR 和原生返回不得提前标为已验收。

交付证据必须列出命令、实际非零用例数量、fixture / seed、平台与版本、结果和剩余缺口。用于验证测试有效性的故障注入限定在测试数据或可还原的临时副本，不能把故意损坏的实现留在工作区。文档链接及设计审查通过不等于上述代码测试通过。



<a id="evidence-testing-2-9-sm1--ut1-状态部分实施记录"></a>

#### 9. SM1 / UT1 状态部分实施记录

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


<a id="evidence-testing-2-10-通用测试支持抽取实施记录"></a>

#### 10. 通用测试支持抽取实施记录

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


<a id="evidence-testing-2-11-sm2-设置应用集成实施记录"></a>

#### 11. SM2 设置应用集成实施记录

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



<a id="evidence-testing-2-12-sm3-订阅桥接实施记录"></a>

#### 12. SM3 订阅桥接实施记录

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



<a id="evidence-testing-2-13-rt1-至-rt3-与-sm4-实施记录"></a>

#### 13. RT1 至 RT3 与 SM4 实施记录

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



<a id="evidence-testing-2-14-sm4-原生窗口与-hmr-验收"></a>

#### 14. SM4 原生窗口与 HMR 验收

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


<a id="evidence-testing-2-15-sm5-更新应用服务实施记录"></a>

#### 15. SM5 更新应用服务实施记录

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


<a id="evidence-testing-2-16-sm6-审核与最终回归"></a>

#### 16. SM6 审核与最终回归

共享状态来源审核、局部状态例外、依赖与后端替换、平台和构建成本见 [验收审计](#evidence-state-audit-1)。2026-11-12 至 13 完成以下本机验证：

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


<a id="evidence-testing-2-17-rt4-目录迁移深链接与原生返回2026-11-13"></a>

#### 17. RT4 目录迁移、深链接与原生返回（2026-11-13）

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

第 17 节初次验收时尚缺系统级 URL scheme 注册、外部进程启动应用和完整 VM reload，现已由第 18、19 节补齐；跨进程持久状态恢复、其他平台真实宿主、发布包升级与 iOS 交互式返回仍未验收。RT4 保持部分完成；SM1–SM5 完成，SM6 的跨平台完整应用构建、其他宿主、发布运行与远端 CI 仍有缺口，见 [状态审计](#evidence-state-audit-1)。


<a id="evidence-testing-2-18-macos-发布包-url-验收2026-11-13"></a>

#### 18. macOS 发布包 URL 验收（2026-11-13）

新增 `python3 scripts/test.py packaged-routing`：生成真实 ad-hoc release `.app`，检查 Info.plist、代码签名和 LaunchServices 注册，通过外部 open 命令验证冷 URL 启动及同进程唤醒。UI 源码使用临时副本并添加只读观察器，Rust 源码只读复用当前 checkout；唯一测试应用 ID 的 data/cache/saved-state 命名空间链接到临时目录，不修改 HOME。停止应用后撤销注册并移除链接；无法停止时保留隔离环境并失败，不把运行中的应用切回系统数据目录。

工具单测在 `scripts/tests/test_packaged_routing.py`，覆盖副本不改产品源码、拒绝覆盖既有或悬空路径、只删除拥有的链接、AppKit 已删除空链接的幂等清理、部分日志不能当证据、未解锁会话须拒绝。真实主机辅助工具位于 `tests/e2e/packaged/host.swift`；测试开始及启动前读取 CGSession 和前台应用，避免把锁屏误判为产品故障。

解锁后已完整执行通过 3 项，证据为 `target/test-reports/packaged-routing-jhaxier9/report.json` 和 packaged-routing.json。会话 ready=true；system-default 分派冷 URL 到设置页；正常启动后的隐藏应用收到 URL，PID 保持一致且 hidden=false、active=true；显式应用 URL 激活亦通过。所有测试目录链接与数据根检查无残留，handler 清理后恢复为 null。此前失败/中止运行不累计为通过。

已将该命令接入手工 native_state CI 作业，远端尚未运行。Developer ID 公证、Gatekeeper 下载链路和实际升级仍不在这项本地 ad-hoc 验收的完成范围。

不依赖解锁的本轮检查已通过：`nsh check`、router-ui 11 项、无头 E2E 4 项、Python 脚本 104 项、actionlint 和 git diff --check。已逐一核对本轮所有报告中的临时数据根与专属链接，剩余路径为 0。未修改生产 Rust 状态、桥接及持久化格式。


<a id="evidence-testing-2-19-完整-vm-reload-与本机续验2026-11-13"></a>

#### 19. 完整 VM reload 与本机续验（2026-11-13）

`python3 scripts/test.py native-reload` 完整通过 1 项。`tests/e2e/reload.test.tsx` 用官方 macOS 原生输入发送 Cmd+R，验证新的 VM / StoreId 恢复持久 dark 偏好，计数、分类、预览、历史和 pending 重置，旧作用域守卫回调不能写入。fixture 在 `tests/e2e/fixtures/NativeReload.tsx`，共用生产 store / router，检查点使用原生 runner 隔离数据根内的 Storage。报告为 native-reload.json、native-vm-reload-0.xml / log；没有用组件卸载代替完整 VM 替换。

首次将 fixture 放在 native/ 下时，NSH 按最近的 App.tsx 识别了另一项目，因缺少根项目 Rust 装配而失败。已迁至上述目录，随后完整重跑通过；首次失败不计通过。headless 套件显式列出 navigation.test.tsx 和 native-back.test.tsx，macOS 专用 reload 独立进入手工 native_state CI。

本轮后续复核：Rust workspace 229 项、Python unittest 104 项、调整发现范围后的完整无头 E2E 4 项、nsh check 和 actionlint 通过；state-targets 的四库七目标共 28 组实际产物通过，应用库 Apple 四目标 check 通过，四个 libusher_nsh 元数据文件存在。证据为 sm6-local-workspace.log、rt4-local-python.log、rt4-local-nsh-check.log、rt4-local-actionlint.log、sm6-local-targets.log、sm6-local-apple.log；这些是本机结果，远端 CI 未运行。

本机 iOS 前置检查失败：Xcode simctl 包装器要求 CoreSimulator 1174.21，已安装 1174.9.2；xcodebuild -checkFirstLaunchStatus 返回 69。只读清单可用，不能视为宿主可运行。已结束本任务的首次启动查询，没有安装系统组件、创建设备或改动已有模拟器；环境就绪后须创建专用设备。用户暂无 Android / Windows / Linux 设备或 CI runner，后续按环境可用性继续，不将缺环境算作通过。

上一轮 scripts/update-sim/sim.py 仍改写 HOME，不符合隔离要求，故未执行（本轮改造及结果见第 20 节）；它的历史 4 项启动模拟证据不能视为新代码的升级回归。续验前须改用唯一测试应用 ID 与专属临时数据命名空间并验证清理，再运行已有有效场景。检查 / 下载 / 安装链路、整包真实归档和分发签名仍按更新方案推进，不能用预置 next 包的启动测试替代完整升级旅程。


<a id="evidence-testing-2-20-sm6-本机更新模拟隔离改造2026-11-13"></a>

#### 20. SM6 本机更新模拟隔离改造（2026-11-13）

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


<a id="evidence-testing-2-更新批次-3--4a-初始化与真实桥接2026-11-13"></a>

#### 更新批次 3 / 4：A 初始化与真实桥接（2026-11-13）

本机 macOS arm64：`cargo test --locked --workspace` 214 项通过（含 20 项独立日志测试、29 项更新状态机契约、17 项更新应用集成）；`cargo clippy --locked --workspace --all-targets -- -D warnings` 与 `nsh check` 通过。`python3 scripts/test.py state-ui` 22 项通过，报告为 `target/test-reports/state-ui.json`；其中新增 `tests/integration/update_bridge.test.tsx` 通过真实 NSH 桥接验证未配置、重复配置拒绝、单执行租约、回执去重、进度校验与订阅释放后的服务保留。最终针对更新桥接的复验报告为 `target/test-reports/update-bridge-final.xml`。

日志核心独立后由工作区测试进入现有 CI；真实桥接测试纳入 state-ui 的必需前缀。设置测试覆盖 auto_check 默认开启、旧文件缺省兼容、默认段省略、落盘重开、未保存状态即时阻止后台检查与重试保存。配置发布失败不创建半配置服务，两个后端共享既有执行契约。

本轮无网络 / 安装执行器，原生结果由测试可控回报。未重跑真实更新模拟、原生多窗口 / VM reload、跨平台编译或发布构建成本，原记录仅代表此前版本。销毁执行 VM 后无租约交接，需重启应用；不能把普通订阅释放测试计作更新执行器的 HMR 验收。


<a id="evidence-testing-2-更新批次-5控制器与产品界面2026-11-13"></a>

#### 更新批次 5：控制器与产品界面（2026-11-13）

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


<a id="evidence-testing-2-更新批次-6真实整包归档替换2026-11-13"></a>

#### 更新批次 6：真实整包归档替换（2026-11-13）

以下五场景为首轮历史结果；本地收尾扩展见本节末尾。

`python3 scripts/test.py shell-update` 在解锁的 macOS arm64 桌面执行，脚本为 `scripts/shell_update_acceptance.py`，测试驱动和场景配置在 `tests/e2e/shell-update/`。与占位归档的发现测试分开统计：旧版 0.1.0 / build 1、新版 0.2.0 / build 3 均为实际构建的 standard 应用；归档保留真实签名、权限和符号链接，update-manifest 生成一次性签名清单。临时 App 调用生产门面，控制器、Rust 桥接、下载校验、安装器及重启均未替换。

宿主逐文件比较安装后的 SHA-256 / 权限 / 链接，包含仅旧版有的资源被移除；从新进程的 shellInfo 和 Settings 快照验证运行版本、暗色 / 英文 / 关闭自动检查的测试偏好与已保存状态，同时验证设置文件字节未变。故障场景核对旧版不变且可重开，原生诊断必须匹配注入故障，产品失败分类必须符合文档。恢复正确归档或目录权限后重新执行正常更新动作。

每轮使用唯一测试 ID，复用 AppDataNamespace 的隔离 data/cache/saved-state 链接，不改 HOME。旧进程、NSH 分离出的重启辅助进程和新进程都纳入清理；确认静默后注销测试路径的 LaunchServices 记录，再解绑数据。与本地模拟器共用命令锁。清理无法完成时保留隔离树并判失败；在进入场景前登记清理责任，避免清理期间中断被误判为无活动应用。单测覆盖实际归档、文件清单、会话隔离、延迟重启进程、无法停止、并发锁、启动前失败和清理期间中断；中断保护此轮按脚本单测验证，不计作原生进程中断验收。

本轮控制器修复 NSH 的 `update install: the archive holds …, this app is …` 被通用 install 匹配误判为 Disk 的问题，应用身份不符现在归入 Rejected。`state-ui` 39 项通过（更新控制器 12 项、更新界面旅程 5 项），脚本测试 135 项通过；`nsh check`、actionlint 和 git diff --check 通过。Rust 领域规则和持久化格式本轮未修改，工作区测试结果沿用批次 5，未作为本轮新执行计数。

该命令已注册 tests/config.json 的 5 个必需用例前缀和驱动来源，并加入手动 native_state CI；远端 CI 未触发。报告保留 JSON、JUnit、应用 / 原生诊断日志和构建命令。测试不覆盖 Gatekeeper 下载隔离标记、Apple 公证、真实 GitHub 分发、/Applications 权限组合、两次 rename 之间的崩溃或 AppKit 变体转换。

最终原生完整运行 **5 项通过、0 失败、0 跳过**：success、archive-tampered、wrong-identity、invalid-signature、install-denied。报告 `target/test-reports/shell-update-yft7wbty/report.json` 与统一入口 `target/test-reports/shell-update.json` 均 passed，JUnit 来源和必需用例组齐全。独立 cleanup-audit.json 核对 19 个唯一隔离路径及运行进程，无残留。

两次诊断轮不计入通过：shell-update-nywzbldb 的 JUnit 来源不匹配且发现身份错误误分类；shell-update-zpm_bsmy 的 SHA-256 日志断言和一次清理权限错误导致失败，保留目录经核对后安全回收（cleanup-recovery.json）。最终运行重新构建了全部产物并执行全部 5 项。持续清理权限拒绝会保留隔离；瞬时拒绝也必须经过后续完整静默确认才可清理，相关单测同时覆盖两种情况。



<a id="evidence-testing-2-批次-6-本地收尾扩展"></a>

#### 批次 6 本地收尾扩展

同一命令扩为 11 个必需场景，要求解锁的 macOS 26+ Apple Silicon 桌面与当前账户可写的 `/Applications`。新增 install-readonly、appkit-to-standard、applications-writable，分别验证真实 canInstall 回退、运行进程原生能力从 AppKit 变为 standard，以及当前账户在 `/Applications` 专属临时目录内替换和重启。目录权限仅修改本轮夹具，不修改 `/Applications` 本身；无法写入时明确失败，不默默跳过。

interrupted-swap 使用只载入测试应用的 DYLD interpose 夹具，在旧应用移走后停止进程，核对备份、从外部恢复并重开旧版；wrong-architecture 用有效 ad hoc 签名的 x86_64 惰性夹具冒充 arm64 归档，在落盘后、重启前截停，避免运行错误架构程序。生产控制器和 NSH 安装器保持原样。两者通过表示边界行为与手动恢复得到验证，不等于安装器具备自动崩溃恢复或架构拒绝能力；已知缺口保存在 case report 的 known_limitation 字段。

gatekeeper-quarantine 仅给测试包写入人工 quarantine 标记，核对有效签名后执行 `spctl --assess`，不打开该包、不改系统信任。拒绝结果证明 ad hoc 的本地策略限制，不替代真实浏览器下载 / 首次打开验收。以上测试沿用原生命令锁和停止后清理原则；源码夹具不进入生产 App。


本轮按用户补充约束增加弹窗内检查按钮的回归：按钮内部加载图标与文案、加载期间拒绝重复激活、关闭弹窗不取消检查；菜单栏仍沿用已有检查环 / 禁用 / 失败保留 / 无更新隐藏规则。`state-ui` 45 项通过（含更新界面旅程 9 项），`nsh check` 通过。


最终完整重跑 `shell-update-w6rsgapu` **11 项通过（8 功能验收、3 已知限制探测），0 失败、0 跳过**。统一入口报告与 JUnit 通过；独立清理审计核对 48 个路径、测试进程，无残留。脚本单测 138 项通过。早期诊断轮 `shell-update-2to7n7fm` 因拦截夹具递归与未注册包注销失败，不计入通过；保留隔离经核对后清理（cleanup-recovery.json），故障夹具增加构建前自检，最终重新构建并跑完全部场景。具体限制与外部待验见更新方案第 13 节。



<a id="evidence-testing-2-23-更新界面整改2026-11-13"></a>

#### 23. 更新界面整改（2026-11-13）

自动代码包下载现在显示边缘进度环，不自动弹出面板；新增可控慢下载旅程，验证下载按钮禁用、完成后显示重启 / 稍后、隐藏不可用检查按钮，以及关闭面板保留就绪状态。状态、说明与操作分行，英文长说明约束宽度并换行；强调图标使用默认 13pt 的 size-update-icon 主题 token。

本轮 `state-ui` 40 项、`ui` 29 项、`cargo test --locked --workspace` 216 项通过，`nsh check` 与差异格式检查通过。隔离 macOS 原生宿主人工核对了 25% 下载环、最终箭头尺寸、设置页布局及弹层长说明换行 / 并排操作；使用可控原生 I/O，不计作真实安装验收。最终预览记录在 target/test-reports/update-layout-4c34n_gr，cleanup.json 确认临时源码、应用与数据树已删除。预览重建过程中一次网络解析失败、一次沙箱监听失败不计入验收通过。首次 UI 回归与 state-ui 并行时争用 NSH 测试生成产物，导致 appearance 用例错误加载更新夹具；最终结果采用单独串行完整重跑，失败轮不计入通过。

后续图标与关闭入口调整：按系统图标优先的要求，macOS 改用 SF Symbols 的 square.and.arrow.down（size-update-icon 默认 14pt）；Popover 标题行使用组件库 View / Button 与现有 SystemIcon、iconButtonLook，右上角 × 替代底部关闭 / 稍后按钮。更新界面 6 项回归通过，覆盖错误与就绪状态的标题关闭入口、关闭后保留状态；nsh check 通过。隔离原生宿主人工确认系统符号显示、英文错误换行及 × 关闭操作，记录位于 target/test-reports/update-layout-parf4ohn；临时树已清理。本次未重跑整包安装或完整工作区测试。


<a id="evidence-testing-2-状态与路由审查整改2026-11-13"></a>

#### 状态与路由审查整改（2026-11-13）

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



<a id="evidence-testing-2-24-更新入口显隐与检查状态2026-11-13"></a>

#### 24. 更新入口显隐与检查状态（2026-11-13）

Rail 根据投影的 visible 在组件边界挂载更新入口：未检查、已是最新、未配置、不支持及无候选检查失败时隐藏；检查中显示无中心图标的灰色不定进度环，并禁用重复触发；有更新时显示系统下载图标，提示“有可用更新”。下载继续显示带菱形的进度环；有候选的下载 / 安装失败保留警示入口。错误详情仍保留在设置页。检查和下载的动效遵循减少动效偏好。

受控检查夹具增加完成信号，新增旅程覆盖初始隐藏、检查细环、禁用按钮和提示、已是最新后隐藏；已有旅程补充失败隐藏、有可用更新提示及重复出现 / 隐藏。最终 state-ui 41 项（含更新界面 7 项）、ui 29 项及 nsh check 通过。首次尝试在 RailUpdate 内包裹已构造的触发器导致旅程停滞，已改为在 Rail 的组件边界条件挂载，随后完整重跑通过；中断轮不计成功。

隔离 macOS 原生预览人工确认灰环在两次截图中持续旋转、无中心图标、检查完成后入口消失且设置 / 账户位置保持。使用可控原生 I/O，未执行真实升级。最终记录在 target/test-reports/update-layout-p7ucdq4p，cleanup.json 确认临时树删除。前置预览因缓存宿主与当前接口不匹配作废；重新构建生成的 macOS 宿主后完成验证，未修改框架。Rust 领域规则未改，本轮未重跑工作区测试。



<a id="evidence-testing-2-25-手动检查失败保留入口2026-11-13"></a>

#### 25. 手动检查失败保留入口（2026-11-13）

检查失败按 Rust 快照的 manual_check 区分来源：后台失败不显示侧栏入口或自动弹出面板，设置页保留原因；手动失败保留侧栏警示入口，使用系统感叹号图标，与可用更新的下载图标区分。设置页和已打开的更新面板保留具体原因与重试动作。关闭面板不清除失败入口，重新检查确认已是最新后才隐藏。

最终 state-ui 42 项通过（含 8 项更新界面旅程），nsh check 与差异格式检查通过。新增自动时钟旅程覆盖后台失败静默、转为手动重试后显示入口；原有离线旅程覆盖弹层内再次检查失败仍保留弹层、关闭后入口仍在、成功后隐藏。检查调用计数保证断言发生在新的检查完成后。初轮错误的弹层选择器，以及点击面板外夹具按钮导致正常关闭的断言失败已修正，最终完整重跑通过。使用可控原生 I/O；本次未重跑真实下载、安装或原生截图验收。

同日样式回调：手动检查失败恢复为实心警示色圆底加系统下载图标，复用原强调形态和主题配色，移除感叹号专用形态及资源配置。按钮保留、弹层重试及后台静默逻辑不变。12 项更新控制器 / 投影测试、8 项更新界面旅程及 nsh check 通过；未重新执行整套 state-ui 或原生截图验收。



<a id="evidence-testing-2-26-日志门面类型控制与诊断页2026-11-13"></a>

#### 26. 日志门面、类型控制与诊断页（2026-11-13）

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


<a id="evidence-testing-2-27-设置标题状态与日志开关闪烁修正2026-11-13"></a>

#### 27. 设置标题状态与日志开关闪烁修正（2026-11-13）

设置标题下的实时保存状态文字及其空行移除，失败重试入口仍走原设置操作。日志输出开关继续在串行保存期间阻止重复提交，但已接入项不再应用禁用透明度；只有尚未接入的终端 / 内存项灰显。未修改 Rust 设置事务、日志管线或状态所有权。

回归使用 tests/ui/fixtures/DiagnosticsPending.tsx 在视图边界暂停一次保存，保留真实设置门面执行提交；它是可控延迟的 UI 夹具，不计为真实磁盘延迟或日志后端验收。无头断言标题状态不存在、等待期间点击受保护、完成后值更新；macOS 原生用例截图保存前 / 等待中 / 完成后三个阶段。已查看截图，并通过本机 AppKit 解码比较等待前后的像素：排除顶部夹具状态行后，整个诊断内容区像素一致。证据为 target/test-reports/diagnostics-switch-comparison.json，原生 JUnit 为 diagnostics-ui-fix-native.xml（2 项通过：实际页面布局 / 控件和可控等待画面各 1 项）。

标题行删除后，旧主题编辑和工具提示用例的部分控件落在底部状态栏遮挡位置；主题状态用例改用足够容纳表单的测试视口，语言操作先滚动至可见区域。没有改动生产布局或滚动组件来迎合用例。

本轮最终串行运行 `nsh check` 与 `python3 scripts/test.py ui` 通过，完整 UI 为 31 项、失败 0 项。之前因旧用例遮挡定位失败的运行不计入通过结果；调整后完整重跑，未拼接单项成功。原生诊断专项 2 项通过，截图与像素对照如上；新增夹具用例和原生用例分别进入已有 ui 与 e2e-macos CI 入口。Rust 未修改，本轮未重复运行工作区 Rust 测试。


<a id="evidence-testing-2-28-更新按钮加载与操作行2026-11-13"></a>

#### 28. 更新按钮加载与操作行（2026-11-13）

检查按钮通过 ThemeButton 的 busy 状态在文案前显示加载圆环，同时禁用按钮；共享 ProgressRing 复用菜单栏已有 Canvas 绘制和减少动效策略。浮层按内容定宽并受安全区域限制，下载与检查操作同排。通用交互要求集中到 [交互设计规范](interaction-design.md)，原 CONTRIBUTING.md 中的布局、主题、国际化与界面降级要求迁入该文档；工程规范和文档索引保留引用。

`nsh check`、`python3 scripts/test.py state-ui`（44 项）与 `python3 scripts/test.py ui`（31 项）完整通过。更新界面旅程在已有用例中增加断言：加载图标位于检查按钮内部，加载期间按钮 disabled，重复 press 返回 ERR_NSH_TEST_DISABLED 且检查次数不增加，完成后按钮恢复可用并移除加载图标。

隔离 macOS 原生专项 1 项通过。临时脚本复制生产界面与可控更新 I/O 夹具到唯一源码目录，通过官方原生 runner 的隔离会话打开应用；已查看 target/test-reports/update-button-ui/actions.png 和 checking.png，核对两个按钮同排、按钮内圆环与禁用外观。首次依赖解析受 DNS 限制、随后沙箱拒绝本地监听，均未计为通过；使用缓存依赖并获准启动本地服务后完整重跑该专项通过。进程退出后清理临时源码，保留截图。该项不代表真实下载、安装或其他平台验收。


<a id="evidence-testing-2-29-异步按钮统一加载反馈2026-11-13"></a>

#### 29. 异步按钮统一加载反馈（2026-11-13）

ThemeButton 与 AsyncButton 通过同一视图事件守卫追踪返回的 Promise；需要等待的操作显式启用加载反馈，在内部显示加载圆环、禁用重复激活。主题、明暗模式、语言和导航等即时切换不显示加载图标。文字按钮预留固定图标槽，图标按钮在固定槽内替换图标，检查与导出保持原文案，原生与 NSH 控件不会随忙碌状态互换。目录、背景选择、诊断导出和更新任务继续使用现有业务状态；下载 / 安装按钮在对应运行阶段保持可见，命令提交完成不会提前消除加载状态。最左侧窄菜单栏本轮未修改。

新增 `tests/ui/async-buttons.test.tsx` 的 7 项用例自动进入已有 UI / CI 发现入口，覆盖文字按钮与图标按钮、重复激活、成功与失败恢复、拒绝的 Promise、即时 Promise 操作不加载、加载前后真实布局尺寸一致、卸载后的完成回调、共享忙碌状态重挂、实际明暗模式 / 主题 / 强调色切换不加载，以及扫描 / 导出入口，以及背景选择取消。安装阶段用受控快照验证表现层，不执行真实安装；更新真实桥接旅程另增下载期间按钮禁用及跨分区恢复断言。

本轮串行完整验证：`nsh check` 通过，`python3 scripts/test.py ui` 37 项、`state-ui` 45 项、`router-ui` 13 项和 `e2e --target headless` 4 项全部通过。报告分别位于 target/test-reports/ui.json、state-ui.json、router-ui.json、e2e.json。Rust 业务代码与桥接契约未修改，未重复运行工作区 Rust 测试。

macOS 原生专项 2 项通过，使用脚本创建的唯一临时源码与官方 runner 的隔离会话、缓存依赖和可控延迟。已查看 target/test-reports/async-button-ui/ 下的 themed-loading.png、icon-loading.png、settings-loading.png 和 export-loading.png，确认按钮内圆环、禁用外观、图标替换及明暗主题显示；JUnit 为同目录 native.xml。进程退出后清理临时源码，保留报告与截图；未修改 HOME 或真实用户设置，不计作真实文件导出、安装重启或其他平台验收。


<a id="evidence-testing-2-gallery-部件主题回归"></a>

#### Gallery 部件主题回归

`tests/ui/theme-finishes.test.tsx` 通过无持久化夹具检查实际控件使用的部件样式，覆盖明暗模式、开关边框与滑块行程、中性选中按钮、输入框、滑杆 / 进度渐变和强调色覆盖。该文件由现有 `python3 scripts/test.py ui` 自动发现。Rust `editor_themes_resolve_with_derived_surfaces` 同时检查新语义角色随自定义调色板更新。静态 macOS 视觉核对使用临时源码目录中的无桥接夹具，不访问应用设置；结果与动效限制见 [主题审计](#evidence-theme-audit-0)。



<a id="evidence-testing-2-加载布局与反馈范围修正2026-11-13"></a>

#### 加载布局与反馈范围修正（2026-11-13）

本次复核通过 `nsh check`、完整 UI 40 项与 state-ui 45 项。UI 中的 7 项按钮专项覆盖即时 Promise 操作不显示 loading、主题 / 明暗模式 / 强调色切换、文字与图标按钮加载前后布局尺寸不变、成功 / 失败恢复、跨视图任务和重复触发保护。完整报告为 target/test-reports/ui.json 与 state-ui.json。

隔离 macOS 原生运行 7 项，6 项通过、1 项超时，整套不记为通过。文字按钮实测在空闲与加载时均为 139 × 28，图标按钮均为 40 × 28；已查看 target/test-reports/button-stability/ 的 async-themed-idle.png、async-themed-loading.png、async-icon-idle.png 和 async-icon-loading.png，确认尺寸、文案位置与相邻控件稳定。原生实际设置切换无 loading 的用例通过。图片选择取消用例超时：NSH 的 onPick 仅接管无头运行，原生运行打开真实文件选择器，因此未完成取消验证；该用例在完整无头 UI 中通过。原生报告为同目录 native.xml，保留失败记录，不视为真实图片选择、导出或安装验收。原生进程已退出，本次临时源码已清理，未修改 HOME 或真实用户设置。



<a id="evidence-testing-2-原生图片选择取消闭环2026-11-13"></a>

#### 原生图片选择取消闭环（2026-11-13）

前述原生图片选择超时已修复。`onPick` 继续仅供无头测试使用；NSH 0.5.0 的原生键盘驱动向宿主窗口发送 Escape，未能取消附属文件面板，因此原生套件使用 `tests/e2e/native-buttons/host` 的独立测试宿主扩展。扩展保留生产模块与组件，只对本进程窗口附属、可见的 `NSOpenPanel` 调用 AppKit 公共 `cancel` 动作，并独立观察面板关闭。测试等待真实取消回调和共享 picker 状态恢复，不伪造选择结果、不改生产入口、不修改 NSH 依赖。

七项按钮旅程提取到 `tests/ui/support/button-journeys.ts`，无头和原生入口复用相同断言。图片选择用例连续两次执行“启动 → 跨分区返回 → 真实选择器取消 → 恢复”，检查 loading 消失、按钮恢复可用、picker 回到 idle、背景记录不变且无错误；原生入口另断言实际面板取消次数与关闭状态。

验证：`nsh check`、无头按钮专项 7 项、完整原生按钮专项 7 项全部通过，失败 / 超时 / 跳过均为 0。原生图片取消用例完成两次真实取消。报告为 `target/test-reports/native-buttons.xml`；此前失败报告保留为历史诊断证据，当前验收已由完整通过结果替代。执行脚本 `scripts/native_button_acceptance.py` 创建临时源码与测试宿主，沿官方 runner 使用隔离数据 / 缓存，并清理临时目录；未修改 HOME 或真实用户设置。套件已接入 `tests/config.json` 的 `e2e-macos`，由既有手动 macOS CI 入口运行；远端 CI 本轮未执行。



<a id="evidence-testing-2-macos-原生套件完整复跑2026-11-13"></a>

#### macOS 原生套件完整复跑（2026-11-13）

执行 `python3 scripts/test.py e2e --target macos`，四组共 13 项全部通过：窗口 / 状态 / HMR / 关闭重开 3 项，原生返回与守卫 1 项，诊断界面 2 项，按钮交互 7 项。图片选择用例完成两次真实面板取消，并验证关闭、加载状态清除与按钮恢复；其他组同轮通过。退出码为 0，四份 JUnit 均无 failure、error 或 skipped，未出现超时。完整汇总为 `target/test-reports/e2e-macos.json`，分组报告为同目录 native-state.xml、native-router-back-0.xml、native-diagnostics-0.xml 与 native-button-interactions.xml。此结果覆盖配置中的完整 e2e-macos 套件，独立的 native-reload、发布包路由和整包更新套件本轮未重跑。

主题动效由 `tests/ui/theme-motion.test.tsx` 覆盖快速反向切换、卸载停止、浮层重开、下拉选择、Tooltip 与 Sheet 关闭，正常 / 减少动效分别运行。`scripts/native_theme_acceptance.py` 在无设置桥接的临时项目中运行 `tests/e2e/theme-motion-native.test.tsx`，检查真实弹簧中间值、收敛、减少动效直达目标及浮层可操作性；该入口已纳入 `e2e-macos`。



<a id="evidence-testing-2-独立-macos-套件回归2026-11-13"></a>

#### 独立 macOS 套件回归（2026-11-13）

串行完整执行 `python3 scripts/test.py native-reload`、`python3 scripts/test.py packaged-routing` 和 `python3 scripts/test.py shell-update`，三套均退出 0 并报告 passed。native-reload 1 项、发布包路由 3 项、整包更新 11 项（8 项功能场景与 3 项边界探针）全部通过；三套 JUnit 均无 failure、error 或 skipped，未出现超时。覆盖 VM 重载与旧回调失效、发布包 URL 注册 / 冷启动 / 同进程唤醒、真实整包替换重启、签名与身份拒绝、权限 / 只读目录、AppKit 到标准版本、中断恢复、错误架构、Gatekeeper 与 Applications 目录场景。汇总报告为 `target/test-reports/native-reload.json`、`packaged-routing.json` 和 `shell-update.json`。结合前轮完整 e2e-macos 的 13 项，本次验证未发现此次按钮交互调整引起的回归。

</details>

<a id="evidence-state-management-3"></a>

<details>
<summary>docs/state-management.md · 迁入记录 4（冻结证据）</summary>


<a id="evidence-state-management-3-9-分批实施计划"></a>

#### 9. 分批实施计划

批次按依赖顺序推进；当前已完成 SM1 至 SM3 和 UT1；RT1 至 RT3、SM4 代码迁移与无头旅程已交付。SM4 已补齐 macOS 原生多窗口 / HMR 验收，SM5 已交付更新应用服务及执行协调器；SM6 已完成本机回归与成本验收，整体保留跨平台 / 远端验收缺口。任务状态与证据在本节记录，scaffolding 03.08 至 03.13 为项目级索引。

| 批次 / 任务 | 前置 | 交付与验收 | 状态 |
| --- | --- | --- | --- |
| SM1 / 03.08 | 已获实施授权 | 创建独立 `usher-state`；实现后端无关接口、工厂注入、纯转换、快照与订阅；watch 为可选实现，同一契约套件验证 watch 与测试后端，消费者代码不变；关闭后端 feature 仍可编译，无 NSH 依赖 | 已完成，见第 10 节记录 |
| SM2 / 03.09 | SM1 | 实现应用作用域与设置适配；保留原有桥接和落盘格式；移除跨目录 I/O 大锁；覆盖多入口并发、有序发布、保存失败及同 `revision` 重试、目录与会话隔离 | 已完成，见第 10 节及测试方案第 11 节 |
| SM3 / 03.10 | SM2 | 实现 Rust NSH 桥接、TS 传输接口和独立 NSH 响应式适配器；组件不暴露 NSH 状态类型；用相同 TS 消费者验证替代传输和测试响应式适配器；覆盖注册竞态、投递、取消与销毁，在固定宿主版本验收 | 已完成，NSH 0.5.0 无头验证，见测试方案第 12 节 |
| SM4 / 03.11 | SM3 | 按 8.3 清单迁移全部现有跨页面共享状态，包括导航、设置分类、请求反馈、导出、共享预览、主页保留计数与派生值；验证应用共享和窗口隔离、往返保留、HMR，清除独立共享 signals 与重复 Store | 已完成：代码、无头及 macOS 原生多窗口 / HMR 验收通过；其他平台按 SM6 跟踪 |
| SM5 / 03.12 | SM4；更新方案相应批次 | 以更新域继续验证底座的效果执行与交接能力；更新状态机留在 `usher-update`，应用适配发布；验证多窗口只执行一次效果、执行者失效交接与迟到票据拒绝；为后续 06.08 记录消息域契约 | 已完成应用服务契约，15 集成 + 2 新单测，兼容现有领域；产品桥接 / UI 仍归更新批次 4 / 5 |
| SM6 / 03.13 | SM5 | 审核全量迁移清单及共享来源扫描，执行后端替换契约与工作区 / CI 回归、目标平台编译及宿主验收；测量成本，回写逐项证据和实际限制 | 部分完成：本机审计 / 全回归 / 成本、七目标纯库和四目标应用编译通过；完整其他平台构建 / 宿主及远端 CI 待验收 |

SM1 至 SM4 完成设置端到端闭环及现有共享数据清单后，才能报告“现有跨页面共享状态已迁移”；SM5 验证更新领域后，才能报告新业务域已接入。后端替换使用同一调用方通过契约测试后，才能报告已验证可替换性。只通过无头用例不能报告跨窗口和 HMR 真机验收通过。各阶段保留旧接口兼容适配，正常入口验证完成后再去除重复的投影同步代码，不长期保留两条独立写入路径。

与更新计划的衔接：更新方案批次 3 的纯业务改造可独立进行；批次 4 的共享状态桥接以 SM1 至 SM3 为前置，批次 5 的界面投影以 SM4 为前置，更新领域效果执行验收纳入 SM5。此依赖不改变更新方案既定串行顺序，不提前实现消息、托盘或账户能力。

与 [路由实施计划](#evidence-routing-4-8-实施计划) 的衔接：RT1 可独立验证纯规则；RT2 依赖 SM1 至 SM3；RT3 与 SM4 合并验收导航迁移。SM4 的其他状态迁移无需等待 RT4 的深链接与原生导航扩展，不因路由而形成循环依赖。


<a id="evidence-state-management-3-10-验证矩阵与交付记录"></a>

#### 10. 验证矩阵与交付记录

测试组织和 GitHub 实践依据统一见 [单元、集成与端到端测试方案](testing.md)。SM1 随核心交付 state fixtures、参考后端与真实 watch 的同契约测试，以及已实现入口 `python3 scripts/test.py state`；SM2 已交付设置真实应用集成及 `integration` 命令，SM3 补 TS mock 投影；SM4 与 RT3 按现有页面旅程联合验收。命令与夹具必须实际进入 CI，不能延迟到 SM6 才补基本测试。

mock 仅用于明确边界和故障注入；真实端到端用例必须经过 Rust 状态服务与桥接订阅。双窗口共享、窗口隔离及跨进程重启分别验证；两个无头引擎或一组假快照不能证明这些能力。完整命令与前置条件见测试方案第 7 节；当前已实现状态核心、设置应用集成、真实订阅桥接与 TS 投影测试；macOS 原生隔离目录、多窗口和 HMR 已交付；跨进程重启与其他平台仍由 TE1 / TE2 跟踪。

| 范围 | 必须覆盖的行为 |
| --- | --- |
| 纯转换 | 合法及拒绝的命令、无变化命令、仅有效果的命令、错误不破坏当前状态 |
| 后端替换 | 相同消费代码运行于 watch 和测试后端；无 Tokio 公共类型；可关闭 watch feature；关闭、错误和最新值语义相同 |
| 提交与投递失败 | 发布失败无半提交；设置适配预检及故障关闭；桥接慢消费者只保留一个待投递值；关闭不漏送；页面释放不影响其他消费者 |
| TS 适配隔离 | 同一页面状态消费者替换传输与测试响应式适配器；NSH 真机验证 getter 跟踪、selector 和批量刷新；共享状态 API 不暴露 NSH Signal |
| 发布顺序 | 并发入口串行提交；可观察变化推进序号；保存结果变化不依赖领域 `revision`；溢出不回绕 |
| 最新值 | 无消费者发布、初次订阅与发布交错、慢消费者跳号后收敛、释放再订阅取得最新值 |
| 一致性 | 订阅与命令回执乱序、同实例旧序号拒绝、实例重建后旧回调拒绝、JS 序号精度 |
| 隔离 | 同路径偏好共享、不同目录及无目录引擎会话隔离；同目录不同窗口的导航、分类、反馈和预览独立；窗口生命周期结束可释放 |
| 持久化 | 旧设置兼容、损坏与未来 schema、写失败及重试、旧保存结果不能确认新状态、资源提交与回收 |
| 生命周期 | 重复取消、注册中卸载、HMR、窗口关闭重开、引擎销毁、Store 关闭、任务与回调可释放 |
| 效果 | 多订阅者仅一个执行者、重复触发合并、取消后迟到回执、执行者销毁与恢复、普通订阅取消不撤销已接受命令 |
| UI | 8.3 全清单迁移；导航及历史、分类、导出、共享预览、主页计数往返保留；应用状态双窗口同步、窗口会话隔离、反馈及局部草稿保留 |
| 平台与成本 | macOS 先验收；Windows、Linux、Android、iOS 分别记录编译和宿主结果；固定发布配置与输入测量成本 |

当前可从仓库根目录运行以下检查；状态两种 feature 组合也可统一执行 `python3 scripts/test.py state`：

```sh
cargo test --locked -p usher-state --features backend-watch
cargo test --locked -p usher-state --no-default-features
cargo test --locked -p usher-update
cargo test --locked --workspace
nsh check
python3 scripts/test.py state-ui
python3 scripts/test.py ui
```

SM1 已执行状态两种组合、工作区测试及 nsh check；更新 crate 的测试随工作区测试通过。SM2 已由适配服务调用原设置仓库，现有 TSX 接口保持不变。界面、宿主与 CI 检查复用 [README 验证入口](../README.zh.md#验证)，新增用例与命令随实施同步中英文 README。性能采样在相同发布配置下比较变更前后，不预设未经测量的性能门限。

| 日期 | 已交付内容 | 验证及限制 |
| --- | --- | --- |
| 2026-11-12 | SM3 / 03.10 与 UT2 桥接部分：引擎订阅、TS 传输 / 版本投影、NSH 适配、owner 清理和递归测试入口 | 工作区 179 项、投影 11 项、真实桥接 6 项、既有无头 UI 29 项、脚本 95 项通过；状态 20 / 35、设置集成 11、工具 4 项通过；fmt / Clippy / nsh check / actionlint / 嵌入文件检查通过。NSH 0.5.0 无头宿主，原生多窗口、HMR 与远端 CI 待验证，见测试方案第 12 节 |
| 2026-11-12 | SM2 / 03.09 与 UT2 设置部分：设置作用域、串行适配、通用发布、兼容桥接、应用集成及 CI | 应用 62 单测 + 11 集成通过；状态 20 / 35、工作区 172、脚本 89、外观无头 2 项通过；fmt、Clippy、nsh check、actionlint 通过。桥接 24 个公开函数签名及 DTO 定义 / 转换不变。未运行远端 CI 或原生订阅 E2E，详见测试方案第 11 节 |
| 2026-11-12 | 测试支持抽取 / 07.10：独立 `usher-test-support`、开发依赖、局部 mock / fixtures、目录规范与 CI 命令 | 工具 4 项、状态 20 / 35 项、工作区 140 项、脚本 89 项通过；fmt、含 watch 的工作区 Clippy、actionlint 通过。生产依赖边界未改变。详见测试方案第 10 节；应用集成和 E2E 仍未实施 |
| 2026-11-12 | SM1 + UT1 状态部分：独立 crate、对象安全后端工厂、纯转换、不可变快照、唯一写句柄、取消订阅、watch 及参考测试后端、state 命令与 CI 作业 | macOS / Rust 1.98.1：核心组合 20 项、watch 组合 35 项通过，含各 2 项编译失败文档测试；工作区 136 项、脚本 89 项通过；fmt、Clippy、nsh check、actionlint 通过。3 处临时源码变异均被捕获，详见测试方案第 9 节。GitHub 新作业未运行，未验收其他平台或原生同步 |
| 2026-11-12 | 补充可替换后端接口、NSH / TSX 适配边界、全量共享数据盘点与扩展验收计划 | 用户补充三项要求；仍仅改文档，全部迁移与后端替换测试待实施 |
| 2026-11-12 | 补充 GitHub 测试实践依据、mock / 契约套件、统一测试命令与真实桥接 E2E 的交付依赖 | 仅调研与设计；测试基础设施及用例待实施，未运行新增命令 |
| 2026-11-12 | 设计、独立 crate 边界、分批计划及文档索引 | 仅文档工作；SM1 至 SM6 均未实施，编译、订阅和原生宿主能力尚未验收 |

依赖核对：关闭 feature 时正常依赖树只有 `usher-state`；启用后为已有锁定的 Tokio 1.54.1（仅 `sync`）及其 pin-project-lite。serde / serde_json 仅用于测试夹具；通用 poll / 唤醒工具已抽取为本地 `usher-test-support` 开发依赖，它自身无依赖。状态参考后端、测试状态机和 JSON 夹具仍在本 crate 的 `tests/`。SM1 和后续抽取均未新增外部包版本。SM2 应用已显式启用 `usher-state/backend-watch`，应用测试增加本地 `usher-test-support` 开发依赖，未新增外部包版本。工作区 feature 合并会启用 watch，但独立 state 命令仍分别验证无默认 feature 和 watch。普通检查及开发构建已通过；尚未进行同条件发布包体积和构建性能对照，不报告成本或性能改善，留待 SM6 的专项测量。


<a id="evidence-state-management-3-11-方案复审记录"></a>

#### 11. 方案复审记录

2026-11-12 再次审核结论：独立核心、可替换后端、NSH 适配与全量共享状态迁移方向可保留，但原文不足以直接作为实现验收依据。以下问题已补入设计；SM1 的接口替换和核心提交失败已通过代码验证，SM2 的设置失败关闭和并发隔离已验证，SM3 的订阅投递、版本与释放已在无头宿主验证；全量 UI 迁移和原生生命周期继续由后续批次验收。

| 问题 | 修订与验证要求 |
| --- | --- |
| 后端工厂的泛型与版本归属不明确 | 按状态类型实例化能力接口；序号和关闭归 Store，后端不重复维护；SM1 编译与替换测试 |
| 领域变更与发布失败可能形成半提交 | 增加线性化点、失败原子性和设置兼容适配的故障关闭规则；SM1、SM2 故障注入 |
| watch 无法限制已经排队的 JS 回调 | 桥接每绑定合并待投递值，覆盖慢消费者与关闭竞态；SM3 |
| 页面可取得 dispose 容易误关共享投影 | 装配拥有释放权限，页面仅释放自己的观察者；SM3、SM4 |
| 字符串序号比较与 HMR 保留假设未验证 | 明确数值比较和组件 / 引擎生命周期区别；SM3、SM4 |
| 导航规则和全局状态接口职责重叠 | 独立路由模块是导航唯一入口，通用 Store 只承载状态；RT1 至 RT3 与 SM4 联合验收 |



<a id="evidence-state-management-3-12-sm4-迁移记录"></a>

#### 12. SM4 迁移记录

2026-11-12 联合 RT2 / RT3 实施。`WindowService` 使用一个 StoreWriter<WindowSnapshot> 发布历史和 session；ui/router 是导航唯一动作入口，普通 session 门面无导航写能力。窗口 Store 由 NSH Local 持有：同 VM 的组件重挂载取得相同历史、entryKey、计数和预览，Local 关闭后拒绝写入；独立无头引擎互相隔离。真实宿主窗口与 VM 的映射及 HMR 尚未据此宣称已验证。

appearance 中的共享 signals 已移除：分类、请求阶段 / 校验、目录与选择反馈、加载问题、透明度预览、导出和主页计数均在 Rust session；调色板、语言、主题和按钮状态只读派生。JS 仅保留执行器互斥、释放标志、重试闭包和资源句柄，不作为可写业务状态源。异步操作使用 Rust ticket，旧完成结果不能覆盖新请求；预览清理也核对 ticket，旧保存完成不能清除较新的草稿。导出由 Rust 持有执行与结果，活动导出拒绝第二次启动。

实时诊断健康和 dropped 计数使用独立 diagnostic_state / DiagnosticState 订阅；日志线程和队列溢出直接发布，不再依赖设置 revision。切换日志管线后旧 source 的回调无效。原 Settings DTO 字段仍保留兼容，但新页面查询使用实时投影。该改动复用通用状态与桥接，未引入新框架。

`ui/navigation` 已删除，App 无 route ID 分支；页面入口集中在 ui/pages，RouterView 根据生成 pageKey 注册表和 entryKey 挂载，并提供 NotFound、初始化失败和渲染失败出口。跨页面扫描覆盖 App、pages、router、appearance 和 Shell；剩余 signals 仅为适配器内部响应式图、抽屉 / 动画 / 测量、当前页面未提交编辑及日期范围。真实桥接、窗口数据往返、同 VM 重挂载和独立引擎隔离的证据见 [测试记录第 13 节](#evidence-testing-2-13-rt1-至-rt3-与-sm4-实施记录)。

首轮 SM4 / 03.11 为“部分完成”；本轮已补齐其 macOS 原生证据并标为完成，详见下节。未进入 SM5 更新领域，未改动更新方案。



<a id="evidence-state-management-3-13-sm4-原生验收与窗口作用域修正"></a>

#### 13. SM4 原生验收与窗口作用域修正

原生验收发现 NSH 同一 VM 可以承载多个 Window，原每 VM 一个 WindowService 只能证明引擎隔离。本批改为 Windows Local 内按宿主窗口名称注册；window-transport 在渲染时捕获 currentWindow().id，取得 Rust 分配的 StoreId，后续读取、订阅、动作和完成回执均绑定此 StoreId。业务状态 / 路由门面签名不变，不暴露窗口选择能力。适配器是可信窗口身份边界，底层注册桥接接收宿主 ID，Rust 不假定 VM 等于窗口。

窗口独占的 hostWindow.on(close) 监听关闭并移除这次 opening 的 Store；Rust 领取标志保证每个 opening 只有一个长期监听，按 StoreId 匹配使旧句柄不能关闭同名重开窗口。长期监听在 owner 外安装，由宿主关闭清理；同步临时监听覆盖注册当轮。普通 owner 清理或 HMR 只取消该投影订阅，即使最后一个内容 owner 已卸载，关闭仍会释放 Store。VM 结束时关闭表内所有服务。设置仍按数据目录共享，持久化和旧 Settings 桥接未修改。

`python3 scripts/test.py e2e --target macos` 在临时源码和数据树中，使用官方 AppKit 宿主、两扇真实窗口以及 nsh dev 的源码监听更新。已验证偏好同步、会话隔离、真实 HMR 保留身份 / 数据、旧投影和待决守卫失效、窗口关闭及同名重开获得新身份。详细条件、命令、失败修复记录和限制见 [测试方案第 14 节](#evidence-testing-2-14-sm4-原生窗口与-hmr-验收)。SM4 完成范围为本机支持的 macOS 开发宿主；目录变更 HMR、完整 VM reload、深链接、原生返回和其他平台仍按相应批次处理。


<a id="evidence-state-management-3-14-sm5-更新领域兼容适配"></a>

#### 14. SM5 更新领域兼容适配

`src/update_service/` 是应用层协调器，业务转换继续调用原 `usher-update::state::Machine`。旧领域 API 仅增加 Machine 的值比较能力；未删除旧签名、通道或工具，不越过更新方案批次 3 的重构顺序。没有修改 Settings DTO、TOML schema、保存路径及原有命令。产品更新按钮仍未启用；实际 NSH 更新调用、定时器及桥接属于更新批次 4 / 5。

服务通过 `BackendFactory<State>` 注入后端，以 StoreReader / Subscription 发布不可变状态；`UpdateRegistry` 复用宿主 SettingsService 身份，同目录窗口共享、不同测试目录隔离。Command 仅允许用户意图，Completion 只由持有 Claim 的执行器提交。执行器的租约、任务队列、效果 ID 保留在协调器，订阅不含可执行效果。

一次变更先复制领域及执行模型，调用既有纯状态机，再提交 usher-state；发布成功后才替换模型并返回领取结果。发布失败时领域票据、队列、租约和在途任务均保持原状。应用内标识不回绕；前置检查保护旧领域票据增量。关闭或 drop 会终止订阅，旧 reader 仅保留最后快照。

撤销执行者进入 draining，旧回执与进度立即失效；原操作真正停止 / Promise 结束后才能 settle。NSH 0.5.0 没有取消或通用在途任务查询 API，不能自动把 owner 销毁解释为下载停止。安装结果未知进入 recovery_required，可信宿主以原 Claim 调用 reconcile 确认未安装或已交接，旧恢复查询也受相同身份校验；此前禁止重试 / 重领。此处验证的是应用契约，不代表真实安装与跨进程恢复已完成。

更新集成夹具位于 tests/integration/fixtures/updates.json；业务执行反馈与故障控制留在测试侧，任务等待 / fixture 身份断言复用 usher-test-support。参考 Memory 后端继续归 usher-state 的契约测试维护，集成测试通过测试源码路径复用，不放入无业务依赖的 test-support crate。相同消费者用例在 Memory / watch 两个后端执行。设置测试只写 tempfile 创建的目录，并比较更新活动前后的设置文件字节。


<a id="evidence-state-management-3-后续-0608-消息域契约"></a>

#### 后续 06.08 消息域契约

消息域维护自己的消息 ID、去重键、接收游标、已读状态和持久化事务；其应用服务持有唯一 StoreWriter。可靠接收队列先确认持久化，再发布当前列表的完整快照，watch 合并不能替代接收确认或事件日志。外部推送 / 拉取由单执行拥有者执行，结果携带消息域代次和请求票据；普通页面只读投影、提交已读 / 删除动作。账户切换关闭旧作用域并拒绝旧回执，退订不撤销已接受的业务动作。该契约不复用更新专用 Claim / Phase，也不把可靠队列塞进通用状态 crate；06.08 实施时补齐该领域测试。


<a id="evidence-state-management-3-15-sm6-全量审核"></a>

#### 15. SM6 全量审核

逐项状态来源、局部例外、后端替换、平台矩阵与成本边界见 [状态管理验收审计](#evidence-state-audit-1)。真实命令结果集中记录在 [测试方案](testing.md)，不把纯 crate 跨目标编译计作其他平台宿主验收。


<a id="evidence-state-management-3-状态与窗口服务契约收敛2026-11-13"></a>

#### 状态与窗口服务契约收敛（2026-11-13）

移除没有生产实现的 StateMachine trait，保留 Transition 与 StoreWriter::apply；领域无需统一继承一种 reducer。设置发布前的容量检查由 SettingsService 在实例锁内完成，保存顺序、saved=false、失败后重试及 I/O 后内部故障关闭策略保持不变。SettingsError 保留 StoreError、领域错误码及更新同步失败原因，只有原生桥接转换为兼容错误码。

WindowService 明确承担窗口作用域和原子提交；独立 window_session 模块维护分类、计数、预览及操作规则，复用 background 的校验函数，不读取其配置内部字段。仍保留一个 StoreWriter<WindowSnapshot>，避免拆开后引入多个版本或生命周期。窗口发布失败时，候选 session、票据计数、目录身份和待决守卫均保持旧值，可重试同一次操作；这一纯内存契约不复用设置 I/O 后的关闭策略。

Session::apply 返回 Receipt::Applied / Superseded / Ticket。Ticket 在 Rust 内为数值新类型，serde 与桥接统一使用规范十进制字符串；尚未启动的观察操作在兼容快照中仍编码为空票据。WindowState.sessionResult 返回带 kind 的回执 JSON，新 TS 传输使用该入口；旧 session 入口保留原空串 / superseded / 数字字符串，仅作为桥接兼容层。设置文件、窗口 StoreId、导航守卫及关闭语义保持兼容。验证结果见测试方案本轮记录；此前原生和成本数据不视为本轮复验。

</details>

<a id="evidence-routing-4"></a>

<details>
<summary>docs/routing.md · 迁入记录 5（冻结证据）</summary>


<a id="evidence-routing-4-8-实施计划"></a>

#### 8. 实施计划

| 批次 / 项目任务 | 前置 | 交付和验收 | 状态 |
| --- | --- | --- | --- |
| RT1 / 04.10 | 已获授权 | 建立独立 usher-router；matchit 可替换适配、目录 schema、生成契约、解析格式化、纯历史转换与导航结果；依赖许可证和目标编译验证 | 已完成，见第 9 节 |
| RT2 / 04.11 | RT1、SM1 至 SM3 | window_service / bridge、唯一窗口 Store、目录注册、并发票据与守卫；验证拒绝不改历史、旧回执、窗口隔离、目录版本及关闭 | 已完成，见第 10 节 |
| RT3 / 04.12 | RT2；与 SM4 联合验收 | ui/router、ui/pages 集中目录、注册生成及 RouterView；迁移首页、设置、菜单、快捷键、面包屑与历史按钮；清除 App 路由分支和旧可写 navigation；全共享状态验收 | 已完成代码迁移和无头验收；SM4 macOS 原生验收见测试方案第 14 节 |
| RT4 / 04.13 | RT3、目标宿主可用 | 深链接、原生返回 / NavStack 适配、HMR 与目录变更、各平台宿主及发布回归；明确未支持能力与成本 | 部分完成：代码、无头与 macOS 开发宿主通过；macOS 发布包 URL 与完整 VM reload 验收通过，其他宿主和剩余发布场景待办，见第 11 至 13 节 |

RT1 不依赖状态管理实现；RT2 复用 SM1 至 SM3；RT3 是 SM4 导航迁移的唯一交付，不再做另一版导航 Store。RT4 不阻塞桌面现有两页迁移。账户、消息与更新专用页面在对应功能批准时注册，不为演示路由提前实现业务。


<a id="evidence-routing-4-81-现有页面整改清单"></a>

#### 8.1 现有页面整改清单

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


<a id="evidence-routing-4-82-mock命令与测试门禁"></a>

#### 8.2 Mock、命令与测试门禁

测试依据、目录和场景见 [测试方案](testing.md)。RT1 随核心交付目录 / 历史 fixtures、matchit 与参考匹配器的同契约测试；RT2 交付可控守卫、乱序回执及唯一 Store 的应用集成；RT3 交付 RouterView mock 测试和现有页面真实桥接旅程。mock 组件通过不能替代 E2E-01 至 E2E-03 等真实链路验收。

已实现独立入口 `python3 scripts/test.py router`、`python3 scripts/test.py routes-check`，以及 `integration`、`router-ui`、`ui` 和 `e2e --target headless`。macos target 已提供隔离目录下的原生窗口 / HMR 验收，需解锁的 macOS 图形会话；具体执行命令、fixture 加载及非零失败约束以测试方案第 7 节为准。测试目录、命令包装器与 CI 发现机制必须随实现交付，不能只在文档列出命令。

验收包括：重复 ID、参数歧义、静态优先匹配、编码、query 多值、resolve/format 往返、push/replace/go、前进分支截断、导航去重、取消/异常/重定向环、异步乱序、卸载清理、NotFound、窗口隔离、页面注册遗漏与生成产物过期。单测使用同一外部契约切换匹配器；NSH 用例验证真实挂载和按钮语义；原生手势不能用纯函数测试替代。

实施后在仓库根目录运行 `cargo test --locked -p usher-router`、`cargo test --locked --workspace`、`nsh check` 和对应路由 / Shell / 设置用例，生成器检查命令随实现写入 README 与 CI。已执行结果见测试方案第 13 节；不以无头通过代替原生平台验收。



<a id="evidence-routing-4-9-rt1-实施记录"></a>

#### 9. RT1 实施记录

2026-11-12 实现 `crates/usher-router/`：不可变 Catalog / Router、可注入 MatcherFactory / RouteMatcher、Location / Target、纯 History 转换及稳定导航结果。核心不依赖 usher-state、NSH、文件系统或业务页面；单独的 usher-routes CLI 负责文件读写。首版可移植模板仅支持绝对路径、整段参数和最后的 catch-all，拒绝正则、隐式可选片段、参数前后缀及重复参数名。保留尾斜杠；命名参数被静态路由遮蔽时返回 invalid-target，不静默切换页面。

路径仅解码一次，编码的分隔符、反斜杠、控制字符及点段拒绝；query 重复值保留顺序，键排序，参数约束与默认值来自目录。普通参数不能包含分隔符；catch-all 按路径段编码。合法未知路径返回配置的 NotFound，未知名称与无效参数保留旧历史。push / replace / go 返回新历史，entryKey 和 revision 检查溢出，不原地更改输入。

固定使用 [matchit 0.9.2](https://docs.rs/crate/matchit/0.9.2)，许可证 MIT AND BSD-3-Clause、MSRV 1.66、无运行依赖；默认关闭 matcher-matchit feature。仅测试使用 [proptest 1.11.0](https://docs.rs/crate/proptest/1.11.0)，MIT OR Apache-2.0、MSRV 1.85，关闭默认 features 并仅启用 std。新增锁定包为 matchit、proptest、rand_xorshift 与 unarray；没有 UI 框架依赖。

参考匹配器和目录 / 场景 fixtures 留在 crate 的 tests/，两后端运行相同解析、拒绝、目录与历史契约；属性测试数量 128、固定 seed 20261112 来自 cases.json，失败缩减由 proptest 保留。通用夹具身份校验抽取到 usher-test-support 的 fixture 模块，路由和既有状态 fixtures 共同复用，业务数据和解析器仍留在各自测试模块。

`python3 scripts/test.py router` 分别执行无默认 feature 和 matchit；`routes-check` 当前验证合成目录的生成契约及 CLI 过期 / 缺失报告拒绝，RT3 已追加实际业务目录的新鲜度检查，生成检查单独计为 checks，不冒充测试用例。类型生成只产出 RouteId、PageKey、RouteParams、RouteQuery；JSON 目录仍为唯一人工来源。CI 的独立核心作业已接入两命令及 Clippy。Windows MSVC、Linux GNU、iOS 真机和模拟器目标已通过纯 crate cargo check；这不是对应原生应用验收。完整包体积与发布构建成本仍待 RT4 / SM6。



<a id="evidence-routing-4-10-rt2--rt3-与现有页面整改记录"></a>

#### 10. RT2 / RT3 与现有页面整改记录

为继续 SM4，先完成其导航依赖 RT2 / RT3。src/window_service.rs 组合纯 Router 与 usher-state，用唯一写入器提交 WindowSnapshot；历史和 session 在同一不可变快照中发布。守卫尚未完成时保留当前页面，Rust ticket 与历史 revision 拒绝迟到回执，session 变化不会误废弃导航；重定向只提交最终目的地，保留 replace 语义并检测环和目录配置的上限。

WindowState 使用引擎 Local 内的 Windows 服务表装配；窗口 ID 由可信 TS 传输适配器捕获 currentWindow，业务门面不暴露窗口选择，动作绑定 Rust StoreId；同目录重复注册复用服务，不同目录内容拒绝且不改变当前历史。订阅复用 SM3 的有界回调与 Stop 清理。VM 关闭主动关闭服务，普通页面仅借用查询 / 导航能力。宿主同 VM 窗口映射、真实源码 HMR 和关闭重开已在 macOS 通过；原子目录升级 / HMR、深链接及原生返回仍待 RT4。

| 原入口 | 当前入口与职责 | 已通过的验证 |
| --- | --- | --- |
| App.tsx 条件页面与计数 signal | ui/pages/home/Page.tsx、settings/Page.tsx；App 仅组合 Frame / RouterView；计数在 Rust session | e2e 实际 App 往返保留计数与分类；ui 29 项 |
| ui/navigation/index.ts 可写历史 | 已删除；ui/router/index.ts 门面 → WindowState → WindowService → usher-router 规则 | router 9 / 14、integration 窗口 8、router-ui 8 项 |
| 手工页面目录与元数据 | ui/pages/routes.json 单一来源，生成 RouteId / PageKey / 参数类型；index.ts 穷尽注册 | routes-check 3 项 + 真实目录检查、nsh check、双语 titleKey 集成 |
| Frame、Rail、TitleBar、SecondaryMenu、Navigation | 全部使用 ui/router；菜单 / 面包屑从目录派生，快捷键和按钮提交同一历史 | 原有 window-shell / tooltips 与 e2e 菜单、快捷键、前进分支 |
| 未知目标回主页 | 合法未知路径渲染 not-found/Page.tsx，无效名称返回 invalid_target 保留旧页 | router 两后端同契约、RouterProbe 真实 NotFound / back |
| 页面直接持有共享反馈与预览 | appearance 读取窗口 session，settings / diagnostics 独立只读投影；动作带 ticket | state-ui 20、窗口集成、WindowLifecycle 同 VM 重挂载 |

RouterView 接收装配入口注入的类型化 PageRegistry，按 entryKey 挂载页面并提供渲染失败重试 / 返回；App 初始化另有错误出口。独立错误注入测试保留真实 RouterView 和历史服务，只替换页面组件，验证首次失败重试不改变 entryKey，以及持续失败页面可以返回既有历史。源码扫描已无旧 navigation import 或第二份 history signal。有关共享状态、测试数据和原生限制分别见状态方案第 12 节、测试方案第 13 节。


<a id="evidence-routing-4-11-rt4-实施与验收2026-11-13"></a>

#### 11. RT4 实施与验收（2026-11-13）

公开深链接使用用户确认的 `usher://app/…`。scheme 由 nsh.json 声明，允许的 host 由路由配置提供；传输层验证地址后进入同一导航守卫，拒绝用户信息、端口、片段和非许可来源。未就绪时仅保留最新地址，卸载后释放监听。

目录更新通过显式 attach 完成：完整验证候选目录，重新解析每个历史 URL，保留 entry key、游标和窗口会话，一次发布新快照。失效地址或收紧的参数规则转为 NotFound；同 identity 不同内容及已淘汰 identity 均拒绝。新绑定携带目录版本，旧绑定与迟到守卫不能提交到新目录。旧 register 与裸 StoreId 桥接继续兼容。

原生返回使用 NSH 0.5.0 公开 BackHandler，同步消费非根返回请求，异步守卫通过后提交 Rust 历史；根节点交还宿主，浮层优先由 NSH 关闭。当前页面不引入第二份 NavStack 历史。NavPage 的 popOnBack=false 可阻止原生提前出栈，但同时禁用交互式滑动返回，因此本批不承诺 iOS 交互式返回手势。

实现使用现有依赖，没有新增第三方库。`usher-router::migrate_history` 隔离匹配后端；应用服务在 `usher-state` 一次发布目录迁移结果，测试继续复用 `usher-test-support` 的可控 future 工具。旧设置桥接及持久化文件格式没有变化。

主窗口接收 UrlScheme 的启动及后续 open 事件；只有聚焦窗口消费系统返回，无聚焦窗口时由主窗口处理。NSH owner 释放 effect，适配器主动移除事件监听并屏蔽迟到事件。根节点返回 false 交还平台；NSH 测试驱动在已注册 BackPress 时可能仍返回成功，因此根节点验收同时检查历史未改变，并以单元契约验证返回 false。

公开 scheme/host 于本批经用户明确确认；配置为 `nsh.json.schemes` 和 `ui/config/routing.json.hosts`。修改目录后须提供新 identity；回退旧内容也应赋予新的 identity，避免旧 HMR 闭包重新附着。目录迁移保留 URL，不按旧 route ID 重定向；新路径可正常导航，旧路径作为 NotFound 保留。RouterView 在 pageKey 改变时重新挂载组件。

验证：Rust 工作区 229 项、路由独立后端 10 / 16 项、应用集成 36 项、router-ui 11 项、无头 E2E 4 项；macOS 4 项（窗口隔离、目录版本 HMR、关闭重开、真实返回及浮层/守卫）。原生 HMR 日志 `target/test-reports/native-run-kb0wdknr`；测试数据根已清理。完整命令与结果见测试方案第 17 节。

macOS 发布包系统 scheme 注册、外部启动与唤醒已在第 12 节补齐。完整 VM reload 已在第 13 节补齐；尚未验收：跨进程持久状态恢复、Windows/Linux/Android/iOS 宿主与发布回归。iOS 交互式滑动返回未实现。上述范围保留 RT4 部分完成状态，不能用无头 UrlScheme 事件注入替代系统注册验证。SM6 上次构建成本报告属于此前快照，本批未重新测量包体积。


<a id="evidence-routing-4-12-macos-发布包深链接验收扩展"></a>

#### 12. macOS 发布包深链接验收扩展

按用户授权继续验证真实 release `.app` 的 Info.plist URL 声明、LaunchServices 注册、外部 URL 冷启动和已隐藏应用的同进程唤醒。使用 `nsh package --target macos --arch arm64`，保留真实产品 App、路由桥接及官方发布运行器，仅在隔离源码副本添加只读路由观察器，并分配唯一测试应用 ID、关闭测试副本更新源。

发布运行器没有 data-dir 参数；不更改 HOME，不修改 NSH 源码或发布二进制。脚本通过系统目录 API 为唯一测试 ID 建立专属目录链接，指向临时 data/cache/saved-state 根；已有路径（包括悬空链接）一律拒绝，不会覆盖。若系统默认处理程序已指向本轮测试包，直接用 `open <URL>` 验证默认分派；已有其他默认处理程序时使用 `open -a <隔离测试包> <URL>`，避免改写或调用用户已安装的应用。LaunchServices 独立查询确认注册。退出后注销测试包并移除专属链接；如果不能结束测试进程则保留隔离目录，验收失败。

新增命令 `python3 scripts/test.py packaged-routing`，要求解锁的 macOS 图形会话，已加入手工开启的原生 CI 作业；注册、冷启动、唤醒分为三项 JUnit 验收。2026-11-13 解锁会话下完整 3 项通过；报告为 `target/test-reports/packaged-routing-jhaxier9/report.json`。此验收针对本机 ad-hoc 签名 release 构建，不代表 Developer ID 公证、Gatekeeper 下载链路或升级验收。

后续按用户指定顺序继续分批：RT4 的完整 VM reload → 可用目标平台宿主与发布回归；随后 SM6 的完整应用跨平台构建 → 宿主验收 → 远端 CI → 发布运行和实际升级旅程。每批按实际设备、工具链和发布条件记录证据与未完成项。

脚本在构建前及启动前检查 CGSession，锁屏或无前台会话直接失败；清理允许 AppKit 已删除空 saved-state 链接，但拒绝移除被替换的路径。冷 URL 启动与正常前台应用的运行中唤醒作为独立场景。此次默认 handler 由系统解析到隔离测试包；裸 URL 冷启动进入设置页，运行中的隐藏应用收到裸 URL 后恢复并激活，显式应用 URL 打开也保持相同 PID 且 active=true。原 handler 为 null，注销后恢复 null；所有临时根和目录链接均已移除。

仅完整通过的解锁运行计作验收；此前锁屏及清理诊断记录不计入通过数量。Apple 的 [NSApplication.activate 文档](https://developer.apple.com/documentation/appkit/nsapplication/activate%28%29) 说明普通激活请求不保证成功，本项目未新增强制抢焦点行为，也未修改 NSH。


<a id="evidence-routing-4-13-完整-vm-reload-与本机平台续验"></a>

#### 13. 完整 VM reload 与本机平台续验

`python3 scripts/test.py native-reload` 已完整通过 1 项真实 macOS 验收。测试使用官方宿主的 Cmd+R 路径重建 VM，未使用组件重挂载替代。`tests/e2e/fixtures/NativeReload.tsx` 装配真实设置、窗口和路由服务；重载前保存 dark 偏好、设置窗口计数 / 分类 / 预览并挂起守卫，通过隔离 Storage 保存旧 scope / ticket 作为回调检查点。重载后新 StoreId、实时订阅和 dark 偏好恢复，历史回到根节点，计数 / 分类 / 预览 / pending 重置，旧回调不能提交。报告为 `target/test-reports/native-reload.json`、`native-vm-reload-0.xml`。VM reload 不等同于整个进程退出重启或应用升级。

官方原生 runner 给每个 open 创建独立 data/cache 目录；本用例不读写真实用户设置，不改 HOME。独立套件进入手工 native_state CI；无头 E2E 明确列出两个可运行文件，macOS 专用 reload 文件不进入无头发现。

本机 iOS 检查发现当前 Xcode 的 simctl 包装器要求 CoreSimulator 1174.21，已安装 1174.9.2；包装器触发首次启动组件安装并停住，`xcodebuild -checkFirstLaunchStatus` 返回 69。已终止本任务的安装查询，未安装组件、创建设备或修改现有模拟器。直接读取已安装 simctl 的可用设备清单成功，但这不算宿主验收。待 Xcode 环境就绪后再创建专用模拟器运行；本轮没有连接 Android 设备、Windows/Linux 宿主或远端 runner，用户确认先推进本机可验证部分。iOS 交互式滑动返回仍未实现，不以环境修复替代功能实现。


<a id="evidence-routing-4-路由结构与错误契约整改2026-11-13"></a>

#### 路由结构与错误契约整改（2026-11-13）

目录构建按目录元数据、ID 索引、路径 / 参数约束、祖先关系分别校验。Router 预建 HashMap ID 索引和祖先链，resolve.rs 独立负责目标格式化、匹配结果检查、参数与 query 校验、Location 构造；目录顺序不影响解析。命名导航仍在生成路径后经过 matcher，以拒绝被静态路由抢占的目标。真实后端故障使用 BackendUnavailable；无效索引、参数集合或无法匹配自身命名路径使用 InvalidMatch。

应用窗口适配名为 WindowService，会话业务规则位于 window_session。错误保留来源，到 bridge 转为 code；非法目标与后端 / 生命周期失败分别映射。窗口提交失败保留原快照、票据计数与守卫元数据，恢复后可重试；不关闭整个窗口。History 只支持界面投影序列化，不是可反序列化的持久化格式。

生成器同时输出 RouteId / PageKey / 参数类型及 routeMetadata 字面量，routes.json 仍是唯一编辑来源；TS 以 satisfies 校验页面、标题文案和父级类型，无双重类型断言。生成格式变化不改变目录内容，现有 identity 保持不变。门面分离守卫求值、重定向循环和票据清理；无票据时只有明确的 router_invalid_target 映射 invalid_target，服务关闭、容量或后端失败均为 failed。

</details>

<a id="evidence-update-5"></a>

<details>
<summary>docs/update.md · 迁入记录 6（冻结证据）</summary>


<a id="evidence-update-5-11-验证记录与待验证项"></a>

#### 11. 验证记录与待验证项

第 1 批在本机 `nsh 0.5.0`（`nsh-core` 固定为 0.5.0）上验证，方法是读 `nsh-updater`、`nsh-engine`、`nsh-macos-app` 0.5.0 的源码，并在临时目录的项目副本和临时 `HOME` 中用一次性密钥实际打包、签名、运行；没有改动仓库文件，临时文件已删除。

| 编号 | 问题 | 结论 | 状态 |
| --- | --- | --- | --- |
| V1 | 更新源字段 | `updater.github`（`owner/repo`）或 `updater.url`（HTTPS）二选一；`publicKey` 是清单密钥，用 `url` 时必填，用 `github` 时设置后拒绝未签名清单；`packages.roots` 至少一个根公钥。`github` 的地址固定为 `releases/latest/download/latest.json`。打包后写入应用内的 `assets/nsh-updater.json` | 已验证 |
| V2 | 运行时能否切换更新源 | **不能。** 配置在打包时写入应用，`manifest_url()` 只读它。更新源因此是构建属性，单一更新源（D7、5.3） | 已验证 |
| V3 | 应用能否自己加载 `.nshpkg` | **能。** 0.5.0 的运行器 `main()` 调用 `nsh_engine::updates::launch`（此前 06.04 的否定结论基于 0.4.0，已过时）。实测完整周期：`next.nshpkg` 在下次启动被运行并改名 `trying.nshpkg`；进程被杀后下次启动日志为 `the package started last never ran 10s without a fatal error; it is deleted and the app goes back`，包被删除并回到内置代码；正常运行超过 10 秒则晋升为 `current.nshpkg`，并创建 `trust/`。数据目录为 `~/Library/Application Support/<app id>/updates/` | 已验证 |
| V4 | NSH 0.5.0 是否具备本文依赖的接口 | 具备：`nsh` 命令行含 `keygen`、`certify`、`publish`、`update-manifest`、`verify`、`revoke`、`package`；`nsh-updater`、`nsh-package`、`nsh-engine` 源码均在本机 | 已验证 |
| V5 | 整包替换后无 Apple 证书的 `.app` 能否启动 | macOS arm64 标准变体已实测真实归档下载、目录替换及 NSH 自行重启，新进程版本 / 构建号正确且设置保留。应用为 ad hoc 签名，替换前后 `codesign --verify --deep --strict` 通过。身份 / 签名错误、目录不可写时旧版未变且可重新启动。本地续验确认人工 quarantine 标记下 Gatekeeper 拒绝；两次 rename 之间杀进程会使安装路径缺失，保留旧包可从外部恢复 | 正常替换通过；崩溃自动恢复存在缺口；真实分发待验 |
| V6 | 发布命令流程 | keygen / certify / publish / verify 已实跑；批次 2 演练和本地模拟已执行 update-manifest。批次 6 又以真实 tar.gz 生成 latest.json / latest.json.sig，由 NSH 检查、校验并安装。正式密钥和 GitHub 线上分发不在本地验收范围 | 本地工具链已验证 |
| V7 | 与 Sparkle 设想是否冲突 | 不冲突，也不需要 Sparkle：整包替换由 `nsh-updater` 的 `install` 完成。06.04 的 Sparkle 设想作废 | 已验证 |


<a id="evidence-update-5-剩余验收与已确认边界"></a>

#### 剩余验收与已确认边界

| 编号 | 问题 | 影响 |
| --- | --- | --- |
| V5 | Gatekeeper 与替换中断 | 本机 `spctl --assess` 拒绝人工 quarantine 标记的有效 ad hoc 包；未改变系统信任或打开该包。真实浏览器下载 / 首次打开流程仍待验。rename 间隙的强制终止已复现：外部恢复旧包后可重开，但无自动恢复。 |
| V8 | 安装位置与权限 | 临时目录与当前账户可写的 `/Applications` 专属目录升级通过；检查前父目录只读时返回 ManualUpgrade、不下载，下载后改为只读则 Disk、恢复权限后可重试。未模拟管理员所有权、ACL 或提权；这些账户条件仍待验。 |
| V9 | AppKit 变体（`macos-appkit`）与标准变体的整包是否可以用同一份更新清单 | **当前发布实现**：清单只列 `standard` 变体的 `.tar.gz`（`update-manifest` 按 `<os>-<arch>` 只收一个），`appkit` 变体只作为手动下载的 zip。真实 AppKit 旧版更新到 standard 已通过，运行能力由有 AppKit 控件变为无；该体验是否可接受仍待确认，不能把技术可行性当作产品决定 |
| V10 | 仓库私有时更新器能否读到更新 | **不能**，见 4.4。历史匿名访问曾返回 404；当前可见性须在正式发布前复核，见 4.4 |
| V12 | 进度环用什么画 | **已实施并在隔离 macOS 宿主实测**：公共 `Canvas` API 绘制圆环，未知总量时用原生 spin 动画，reduceMotion 时停止旋转；进度按钮禁用且保持不透明。详情区使用 `Progress` 的 trackStyle / indicatorStyle 设置高度与填充。无需新增原生组件；headless 另验状态、禁用和未知总量。 |
| V13 | 清单架构标签与归档内可执行文件不一致 | 已以签名有效、清单标为 arm64 的 x86_64 惰性夹具复现：NSH 0.5.0 会替换应用，未拒绝架构不符。测试在替换后、重启前截停，不运行该文件，恢复旧版后可重开。发布前架构校验与安装器防御仍需补齐，签名正确不等于架构正确。 |

已验证：`downloadUpdate` 与 `downloadPackage` 的 `onProgress` 回调给出 `{ received, total? }`，`total` 在服务端不给长度且清单没有大小时为空（空则退化为不确定旋转）；**没有取消接口**，下载是一个不可中断的异步任务，且同一时刻只允许一个更新操作（再次调用会被拒绝）；校验在下载 Promise 内部完成，**没有独立的校验阶段回调**，所以界面无法区分"下载中"与"校验中"。


</details>

<a id="evidence-update-6"></a>

<details>
<summary>docs/update.md · 迁入记录 7（冻结证据）</summary>


<a id="evidence-update-6-13-实施批次"></a>

#### 13. 实施批次

通用状态管理的前置关系见 [状态管理实施计划](#evidence-state-management-3-9-分批实施计划)：本方案批次 3 的纯业务改造可独立进行；批次 4 的共享状态桥接以 SM1 至 SM3 为前置，批次 5 的界面投影以 SM4 为前置，第二领域复用与效果单执行者在 SM5 验收。此依赖不改变下表批次 3、4、5 的串行顺序；当前按 A 方案交付批次 3 至 5；批次 6 已完成 macOS arm64 标准变体的本地真实整包替换，正式发布及其余验收仍待完成。

| 批次 | 内容 | 状态 |
| --- | --- | --- |
| 1 | 验证 V1 至 V7，回写本文 | 已完成大部分；V6 本地工具链已补验，V5 Gatekeeper 和 V8、V9 剩余项留待后续 |
| 2 | 信任与发布（以首次发布 `0.1.0` 为目标，D13）：离线根密钥（由维护者生成）、`nsh.json` 的 `updater` 配置、外壳输入门禁、门禁只接受正式版标签、流水线产出 `.nshpkg` 与清单 | **演练已在 GitHub 上通过，真实发布待做**。按批次 2 记录，密钥已生成、公钥已写入 `nsh.json`，三个 Secret 曾完成配置；发布前仍须核对。2026-11-12 在 GitHub 上手动运行 Release 演练：`gate`、`verify`、两个变体的 `build-macos`、`rehearse` 全部通过，`Sign and publish` 按设计跳过；`rehearse` 的三种损坏（换归档、去掉签名、与外壳结论不符）都被拒绝。**仍未验证，只有真实发布才会暴露**：三个真实 Secret 是否配对，`release` 环境、草稿转正式、`--latest`、构建来源证明。**代码包路径（`nsh publish`、`nsh verify`、换根被拒）没有在 CI 上执行过**，因为没有标签，`shell_changed=true`，只在本机跑过。更新源匿名可读性须在正式发布前按 4.4 复核 |
| 3 | 双路径状态机、删除旧签名工具、应用服务按 A 方案初始化、独立日志 crate | **已实施，本机 Rust 验证通过**。无取消 / 无交接；日志通过观察接口适配应用状态。 |
| 4 | `UpdateState` 桥接、`[updates].auto_check`、DTO、稳定错误码、更新日志 | **已实施**。真实桥接用例纳入 state-ui / CI；批次 5 已验证本地源代码包下载与再启动；批次 6 已补本地真实整包替换验收。 |
| 5 | 执行控制器、定时检查、`ui/features/updates/`、布局契约、Rail 状态、文案与测试 | **已实施**。真实桥接配合可控原生 I/O 的界面旅程、执行生命周期测试与隔离 macOS 视觉检查通过；本地真实下载模拟结果见本轮验收记录。 |
| 6 | 文档与验收：同步 architecture、scaffolding、deploy、README，真机验收 | **部分完成**。macOS arm64 标准变体的本地真实归档替换、重启、设置保留与失败恢复已验；本地权限回退、AppKit 转 standard 和边界探测已补验，但确认了替换中断无自动恢复、错误架构未拒绝两个安装器缺口。正式分发、真实 Gatekeeper 首次打开、管理员权限组合及 V9 产品决定仍待完成。Apple 签名 / 公证按 D6 暂缓；其他平台不在当前范围。 |

验证限制：开发运行无可更新的外壳版本；正式更新源要求 HTTPS，本机回环 HTTP 是 NSH 允许的模拟例外。本轮已用隔离打包应用验证真实代码包检查 / 下载 / 再启动，本地真实整包替换也已另行验收；仍不能替代正式 GitHub Release、真实密钥与 Gatekeeper 验收。流水线演练已运行，正式发布仍未执行，第一次正式发布必须使用正式版标签。


<a id="evidence-update-6-批次-6真实整包归档验收范围"></a>

#### 批次 6：真实整包归档验收范围

本轮先验收 macOS arm64 的标准变体整包路径，随后扩展本地边界。独立 `shell-update` 测试命令复用模拟器的一次性密钥、本机源和数据隔离。构建同一唯一测试 ID 的旧 / 新 `.app`，用真实新版生成 tar.gz 和签名清单；生产 App 仅在临时源码副本注入测试驱动，调用与界面相同的 check / download / install 动作，不替换控制器、更新桥接或 NSH 安装器。

验收证据必须同时包含：下载的是签名清单列出的真实归档；替换后应用文件清单与新版相符；旧进程退出且 NSH 自行启动的新进程上报正确 shellInfo 版本 / 构建号；已保存设置字节与新进程读到的偏好一致；安装暂存目录 / 旧目录清理。测试宿主拥有重启后的进程，确认全数退出才清理命名空间；无法证明退出时保留隔离树并报失败。

负向场景包括归档哈希不符、归档内应用身份不符、应用代码签名无效、下载完成后安装目录不可写。失败时核对旧应用文件未变、旧版可重新启动；可恢复的下载 / 权限失败再走正常动作完成升级。故障只注入本轮临时应用、归档及父目录，不改真实安装、系统信任或 NSH 源码。首轮五场景未覆盖替换中断、隔离标记或 AppKit 转换；本地续验范围与结果见下文。Apple 公证与正式 GitHub 分发不在本轮执行范围。


<a id="evidence-update-6-批次-6-本地收尾范围2026-11-13"></a>

#### 批次 6 本地收尾范围（2026-11-13）

本轮不执行正式发布，不操作真实密钥、Release 或仓库可见性。先修正文档中的旧门禁、版本和验收描述，再扩展本地真实归档测试：

- 在当前账户可写的 `/Applications` 下创建唯一临时安装目录验证真实升级，并在退出测试进程后清理；检查前安装父目录不可写时，实际 `canInstall` 应投影为手动升级且不下载归档；与下载后权限变化的 Disk 失败分开验收。
- 构建真实 AppKit 旧版，更新到清单唯一的 standard 整包，核对新进程、文件和偏好。技术可行性不代替 V9 的产品取舍确认。
- 在仅测试进程加载的系统调用拦截夹具中，精确停在旧应用移走后、第二次 rename 前，杀死该测试进程，记录磁盘状态并验证从保留旧包手动恢复。若缺少自动恢复，必须标明缺口，不能把手动恢复算成崩溃安全。
- 以本地有效签名但架构与清单不符的归档检验安装器边界；在替换完成后截停并终止测试进程，不让系统尝试运行错误架构夹具；本地 ad hoc 包另做 quarantine 标记 / Gatekeeper 评估。两者仅记录实际行为，不代替可信发布签名与真实浏览器分发。

故障夹具仅作用于唯一测试 ID、临时应用与父目录，不改 NSH 源码、系统信任设置、真实安装和用户数据。无法在当前账户下覆盖的管理员所有权 / ACL 条件与真实分发列为外部待验。


<a id="evidence-update-6-批次-6-首轮五场景结果2026-11-13"></a>

#### 批次 6 首轮五场景结果（2026-11-13）

在本机 macOS arm64 解锁桌面执行 `CARGO_NET_OFFLINE=true python3 scripts/test.py shell-update`，最终 **5 项通过、0 失败、0 跳过**。完整报告为 `target/test-reports/shell-update-yft7wbty/report.json`，统一入口报告为 `target/test-reports/shell-update.json`，JUnit 为 `target/test-reports/shell-archive-macos.xml`。

| 场景 | 实际结果 |
| --- | --- |
| success | 真实归档下载后替换，NSH 启动 0.2.0 / build 3；安装文件与新版一致，旧版专有资源移除，设置保留 |
| archive-tampered | SHA-256 不符归入 Rejected；旧版未变且可重开，恢复正确归档后升级成功 |
| wrong-identity | 清单身份正确但归档内身份不符，原生拒绝且产品归入 Rejected；旧版未变且可重开 |
| invalid-signature | 清单与归档哈希有效，但内部代码签名损坏，拒绝安装；旧版未变且可重开 |
| install-denied | 下载完成后使安装父目录不可写，失败归入 Disk；旧版可重开，恢复权限后正常升级 |

每项均确认不下载代码包、未确认不下载整包、已保存设置字节不变、安装暂存 / 备份目录无残留。最终清理复核 `cleanup-audit.json` 检查了 19 个唯一隔离路径，测试进程、系统链接与临时源码 / 应用 / 密钥 / 数据均无残留。

首轮诊断报告 `shell-update-nywzbldb` 暴露应用身份错误误分类和 JUnit 来源标识不匹配；第二轮 `shell-update-zpm_bsmy` 因哈希诊断文本断言和清理权限错误失败，保留树已在核对进程退出后回收，证据为 cleanup-recovery.json。两轮均不计入最终 5 项通过。修复后重新构建并完整执行全部场景，没有以单项补跑拼接成功结果。

state-ui 39 项、脚本 135 项、nsh check、actionlint 和差异格式检查通过。命令与测试已加入 CI 配置，本轮未运行远端 CI / 正式发布。批次 6 仍为部分完成，剩余边界见上方验收范围与 V5 / V8 / V9。


<a id="evidence-update-6-批次-6-本地收尾结果2026-11-13"></a>

#### 批次 6 本地收尾结果（2026-11-13）

在本机 macOS 27.2.1 / arm64 解锁桌面，经统一入口 `CARGO_NET_OFFLINE=true python3 scripts/test.py shell-update` 重新构建三个真实应用并完整运行，**11 项检查通过、0 失败、0 跳过**。其中 **8 项为功能验收，3 项为边界探测**；后者确认了限制，不能视作产品保护已实现。最终报告为 `target/test-reports/shell-update-w6rsgapu/report.json`，含独立的 functional_cases_passed / boundary_probes_passed / known_limitations；统一报告与 JUnit 均通过。

| 新增场景 | 实测结果与范围 |
| --- | --- |
| install-readonly | 检查前使夹具父目录只读，真实 canInstall 投影为 ManualUpgrade；不下载归档、不改应用和设置 |
| appkit-to-standard | 真正的 AppKit 0.1.0 更新到 standard 0.2.0 / build 3；新旧进程的 AppKit 能力分别为 true / false，文件与新版一致，设置保留 |
| applications-writable | 在当前账户可写的 `/Applications` 下创建本轮唯一临时目录，真实替换、NSH 重启和设置保留通过；退出后删除专属目录。未修改 /Applications 自身权限 |
| interrupted-swap（边界） | 第一轮 rename 后强制终止，原应用路径缺失，旧包备份字节完整；从外部恢复后旧版可重开。**没有自动恢复能力** |
| wrong-architecture（边界） | 清单标为 arm64、内部为有效 ad hoc 签名的 x86_64 惰性可执行文件，安装器仍接受并替换；在重启前截停，恢复旧版可重开。**没有归档内部架构拒绝能力**，未运行错误架构夹具 |
| gatekeeper-quarantine（边界） | 有效 ad hoc 包添加人工 quarantine 标记后，spctl 返回 3 / rejected；未打开该包或改变系统信任。**真实浏览器分发仍待验** |

原有 success、archive-tampered、wrong-identity、invalid-signature、install-denied 也在同一轮完整通过。每个场景均完成隔离清理；独立 `cleanup-audit.json` 复核 **48 个唯一路径，残留 0，测试进程 0**，包含 /Applications 专属目录、系统链接、临时源码 / 应用 / 一次性密钥 / 数据。

脚本单测 **138 项**、`state-ui` **45 项**（含菜单栏交互保持与弹窗内加载按钮专项回归）及 `nsh check` 通过。本轮没有修改 Rust 业务规则，不将历史工作区测试记作新执行结果；未执行远端 CI 或正式发布。

诊断轮不计入最终通过：`shell-update-tm8jmblj` 在构建阶段主动中断以调整错误架构夹具；`shell-update-2to7n7fm` 的重命名拦截递归使测试应用启动失败，且未启动的 quarantine 包不能按已注册应用注销。修正后增加构建前的两个截停点与无关重命名自检，并仅注销实际启动过的测试包；保留树在确认无进程、链接归属正确后清理，见该轮 cleanup-recovery.json。其 boundary-diagnostic 子目录仅为诊断证据，最终结果采用上述全新身份、重新构建、全部 11 项完整运行。

**批次 6 仍为部分完成。** 后续工作分为：修复或明确处理 rename 间隙恢复与错误架构校验；确认 AppKit 转 standard 的产品取舍；在相应账户条件下验证管理员所有权 / ACL；正式发布阶段再核对真实密钥、匿名更新源、Release latest、真实下载 / 首次打开。Apple 签名 / 公证按 D6 暂缓，其他平台不在当前更新范围。本轮没有发布、改标签、改仓库可见性或使用正式私钥。


<a id="evidence-update-6-批次-6-后续待办"></a>

#### 批次 6 后续待办

2026-11-13 用户决定：以下事项先记录，后续再处理；本轮不继续修复或执行正式发布。已有本地验收结果保留，批次 6 仍为部分完成。

| 编号 | 待办 | 后续处理与验收要求 | 状态 |
| --- | --- | --- | --- |
| U6-01 | 整包替换中断后缺少自动恢复（V5） | 确定安装器或外部恢复方案；在两次 rename 之间终止进程后，验证应用可恢复启动、设置保留及备份清理，不能以手动恢复代替自动恢复验收 | 待后续处理 |
| U6-02 | 签名有效但内部架构不符的归档未被拒绝（V13） | 补发布产物架构核对与安装前防御；以错误架构归档验证替换前拒绝、旧应用不变且仍可启动 | 待后续处理 |
| U6-03 | Gatekeeper 与真实下载首次打开（V5） | 在真实浏览器分发条件下验收隔离标记、首次打开及更新后启动；记录 ad hoc 分发的实际体验。Apple 签名 / 公证继续按 D6 暂缓 | 待后续验收 |
| U6-04 | 管理员所有权与 ACL 权限组合（V8） | 在对应账户和安装条件下验证 canInstall、手动升级入口及安装失败时旧版可用性 | 待后续验收 |
| U6-05 | AppKit 更新为 standard 的产品取舍（V9） | 先确认是否接受原生控件能力变化，再决定保持当前清单或调整变体分发；已有技术转换验收不替代该决定 | 待产品确认 |
| U6-06 | 正式发布与线上更新链路 | 后续获准发布时再核对真实密钥配对、匿名更新源、Release latest、真实下载和两条更新路径 | 待后续授权与验收 |

复现证据统一见 `target/test-reports/shell-update-w6rsgapu/report.json` 及上方本地收尾结果。后续处理时按编号回写实现、验证结果与状态。


<a id="evidence-update-6-批次-5-本轮验收2026-11-13"></a>

#### 批次 5 本轮验收（2026-11-13）

- 工作区 216 项、state-ui 38 项、UI 29 项、无头旅程 4 项及脚本 120 项通过，Clippy / nsh check 通过；覆盖清单见 [测试方案](#evidence-testing-2-更新批次-5控制器与产品界面2026-11-13)。
- 隔离 macOS 宿主实测确定 / 不确定 Canvas 环、Popover、就绪图标、失败反馈和线性 Progress 填充；下载中仅热更新设置组件后状态与执行次数保留。此项的原生更新调用为可控适配器，完整 VM 重建不在可恢复范围。
- `scripts/update-sim/sim.sh run --all`：12 项通过、0 失败、0 pending，报告 `target/test-reports/update-sim-d5xkzs26/report.json`。新增 package-download 使用生产控制器和一次性签名密钥，从回环源下载包后再启动，最终存在 current.nshpkg / trust，不存在 next / trying / .part。整包场景仅请求清单与签名，不下载占位归档。
- 首轮中断下载暴露模拟服务器缺少 socket 导入，已修正；单项复验的首次构建遇 crates.io DNS 失败（报告 update-sim-chwwm0xa），未计作通过。随后以 CARGO_NET_OFFLINE=true 复用缓存完整构建并执行该场景，通过且无残留，报告 `target/test-reports/update-sim-fozvm6x9/report.json`。
- 每场景均验证隔离清理；整轮临时源码、应用、密钥和数据根已删除，三个专属系统命名空间链接均无残留。未改变 HOME 或真实应用设置。


<a id="evidence-update-6-14-变更记录"></a>

#### 14. 变更记录

| 日期 | 变更 |
| --- | --- |
| 2026-11-13 | 批次 6 本地收尾：清理门禁、版本与验收旧描述；11 项完整检查通过（8 功能、3 边界），明确 rename 中断恢复、内部架构校验和 ad hoc Gatekeeper 限制；正式发布未执行。 |
| 2026-11-13 | 批次 6 本地真实整包验收：新增 shell-update 五场景、安装后逐文件 / 进程 / 设置 / 清理证据，修正归档内身份拒绝误分类；正式分发与 Gatekeeper 等边界仍待验。 |
| 2026-11-13 | 批次 5：接入单执行者控制器、30 秒 / 6 小时 / 2 次配置、更新设置、菜单栏进度与浮层；共享版本和检查元数据，按候选 canInstall 降级。真实桥接界面、卸载 / 订阅 / 空领取竞态与隔离 macOS 检查通过，本地模拟扩至 12 个有效场景。 |
| 2026-11-13 | 批次 3 / 4 按服务初始化 A 方案实施；设置作用域创建空服务，一次性 Configure；用户确认 auto_check 默认开启。独立日志 crate、真实 UpdateState 桥接与设置同步已接入；工作区 214 项、state-ui 22 项通过，Clippy 和 nsh check 通过。批次 5 的实际执行器 / UI 与真实升级仍待办。 |
| 2026-11-12 | 对接通用状态管理设计：通用层独立为 `usher-state`，更新业务状态机保留在 `usher-update`；明确应用作用域、单执行者和 SM1 至 SM5 的实施前置关系，仅更新文档。 |
| 2026-11-12 | 建立本文：记录 D1 至 D12、第 3 至 12 节方案与 V1 至 V7 待验证项。 |
| 2026-11-12 | 新增第 12 节本地更新模拟（D14）：命令、工作方式、11 个场景与限制。4 个场景已在本机通过，7 个标为 `pending`，等批次 4、5 的检查逻辑。 |
| 2026-11-13 | 决定执行者协议选项 B（不是服务初始化的 A / B 选择）：删除取消与执行者交接（`update_service` 中的 `Cancel`、`Settlement`、`Recovery`、`revoke`、`settle`、`reconcile` 等），保留单执行者领取；脱敏范围定为只保留凭据类；并行会话已结束，批次 3 恢复。 |
| 2026-11-13 | 批次 3 状态机设计补充：失败分五类并各自对应面板处理与日志级别；删除 `Cancel` 事件（NSH 无取消接口，V12）；新增更新日志设计（状态机产生记录、桥接写出、只做 `$HOME` 替换为 `~`）；新增 8.6 日志 crate 拆分要求。发现并行会话已在 `Machine` 外加了 `src/update_service/`，批次 3 与日志拆分等其结束后实施。 |
| 2026-11-12 | 批次 2 演练在 GitHub 上通过；批次 3 开始，决定串行推进，删除 `min_version`，"外壳过旧"由代码包与外壳的主次版本是否相同表达。 |
| 2026-11-12 | 批次 2 实施：门禁改为只接受正式版并要求 `updater` 完整；新增外壳输入清单与检查、版本线约束、清单检查器、`make-update-files.sh`；`release.yml` 的 `publish` 与 `rehearse` 改用 NSH 原生命令；`rehearsal-sign.sh` 改为同一链路加损坏用例；删除 `collect-release-files.sh`。发现仓库私有，新增 4.4 与 V10。 |
| 2026-11-12 | 确定第一次发布为 `0.1.0`（D13），新增 4.5 说明首次发布的限制与验证办法。 |
| 2026-11-12 | 取消预览通道，只保留正式版（D7、D8）：删除预览清单与滚动 Release、通道状态与按构建生成两份配置的设计，门禁只接受正式版标签。 |
| 2026-11-12 | 第 1 批验证回写：通道改为构建属性（V2），设置页不再提供通道切换，`[updates]` 只含 `auto_check`；代码包路径可行（V3，推翻 06.04 的否定结论）；Sparkle 设想作废（V7）；新增 V8、V9；4.3 补充两个通道对应两种构建，预览用户升到正式版后仍留在预览通道。 |
| 2026-11-13 | SM6 本机续验：更新模拟采用唯一测试 ID / 临时数据根，禁止 HOME 重定向；补齐 16 项隔离清理单测，脚本全量 120 项通过，真实启动模拟 4 项通过 / 7 项 pending；普通结束与 SIGTERM 均核对无隔离残留。 |

</details>

<a id="evidence-notify-7"></a>

<details>
<summary>docs/notify.md · 迁入记录 8（冻结证据）</summary>


<a id="evidence-notify-7-11-分批实施与完成条件"></a>

#### 11. 分批实施与完成条件

| 批次 | 范围 | 完成条件 | 当前状态 |
| --- | --- | --- | --- |
| N0 | 组件调研、Slint 对照与完整设计 | 本文明确边界、契约、生命周期及验收 | 设计完成；未运行原生通知实验 |
| N1 | `usher-notify`、共享配置及核心契约 | 独立编译；预算、去重、替换、动作与失败原子性测试通过 | 待实施 |
| N2 | 窗口服务、桥接、TS 门面 | Rust/TS 双入口、版本订阅、窗口隔离与清理契约通过 | 待实施 |
| N3 | NotifyHost、主题、国际化与展示适配 | 真实 NSH 入口验证计时、堆叠、焦点、主题和 HMR；发现组件缺口先提交处理方案 | 待实施 |
| N4 | 业务接入与回归 | 先接诊断导出及适用的显式设置操作；更新方案同步后再接更新；避免重复反馈 | 待实施 |
| N5 | Inbox 与 06.08 展示端 | 历史/已读/持久化/对账独立验收；实时接收另按 06.08 验收 | 后续批次 |
| N6 | 系统通知 | 按平台完成权限、投递、激活、降级与关闭排空验证 | 后续批次 |

N1 至 N4 构成首个可用的应用内通知交付。实施时同步架构、状态归属、测试方案、分批清单及中英文 README；未实现的 Inbox 和系统通知不写为现有能力。


<a id="evidence-notify-7-12-实施前待核验项"></a>

#### 12. 实施前待核验项

- 本地 Toast 类型声明已核实，但 `ToastStack` 在实际宿主的焦点暂停、模态层级和安全区组合仍需 N3 实验；能力不足时按交互规范处理。
- 确定行为配置的默认时长、容量、预算与位置，并通过默认主题提供通知 token；本轮不为这些参数写入硬编码。
- HMR 对展示 Host 的释放/重建顺序须实测，确保不会出现两个有效时钟 owner；首批明确接受重挂载后重新计时。
- 更新接入前明确 notice 到目标窗口的映射及满载时的 effect 完成策略，不能隐式投递到最后聚焦窗口。
- 系统通知 API、依赖成本及各平台能力在 N6 单独调研，当前不作已支持承诺。

</details>

<a id="evidence-log-8"></a>

<details>
<summary>docs/log.md · 迁入记录 9（冻结证据）</summary>


<a id="evidence-log-8-8-实施批次与迁移清单"></a>

#### 8. 实施批次与迁移清单

| 批次 | 状态 | 交付 |
| --- | --- | --- |
| LG1 普通门面 | 已实施 | 自有 Level / Fields / LogKind / Submit，移除公开底层辅助 API |
| LG2 私有后端 | 已实施 | 模块按职责拆分，LogWriter / ArchiveProvider，保留文件语义和配置 |
| LG3 应用适配 | 已实施 | 健康观察者、设置事务和归档桥接迁移；旧设置兼容 |
| LG4 全部日志调用 | 已实施 | 启动、等级变更、TSX 短事件、更新记录 / detail、安装前刷新、panic / exit |
| LG5 分层验证 | 已实施 | 模块单测、公共契约 mock / 真实文件对照、应用集成、桥接及 UI 命令进入现有 CI |
| LG6 类型控制与页面 | 已实施 | 三个实际开关，两个不可用占位，分区换序与诊断页重排；无真实终端 / 内存采集 |

迁移采用现有 `usher-logging`，未新增第二套日志系统。`usher-update` 的领域 Level / Record 不迁入 logger，避免把 I/O 依赖带进纯规则。`src/diagnostic_state.rs` 与 `diagnostic_bridge.rs` 留在应用，测试辅助 Capture 留在日志 crate 的 tests/support，不进入运行时依赖或通用测试工具 crate。


</details>

<a id="evidence-log-9"></a>

<details>
<summary>docs/log.md · 迁入记录 10（冻结证据）</summary>


<a id="evidence-log-9-10-本次验收"></a>

#### 10. 本次验收

本机 macOS arm64 已通过 20 项日志单测、13 项公共契约、241 项工作区测试、44 项应用集成、43 项 state-ui、30 项 UI 与 5 项原生验收。原生宽窄屏和归档区域截图已核对；完整结果见 [测试方案第 26 节](#evidence-testing-2-26-日志门面类型控制与诊断页2026-11-13) 与 [批次 05](#evidence-scaffolding-12-批次-05--诊断日志--部分完成)。当前未验收 Windows、Linux、Android、iOS 的宿主输出 / 分享、真实盘满和进程强杀；它们继续作为平台缺口保留。

</details>

<a id="evidence-deploy-10"></a>

<details>
<summary>docs/deploy.md · 迁入记录 11（冻结证据）</summary>


<a id="evidence-deploy-10-发布前核对首次发布-010"></a>

#### 发布前核对（首次发布 `0.1.0`）

`0.1.0` 是真实发布，标签不可移动，所以下面每一项都要在推标签之前确认：

1. `nsh.json` 的 `updater.publicKey`、`updater.packages.roots` 与 `~/.usher-nsh-update-signing/` 里的 `manifest.pub`、`root.pub` 一致（门禁只检查它们存在，不检查是不是对的那一把）。
2. 三个 Secret 已配置，且 `NSH_RELEASE_CERT` 对应的根就是 `root.pub`，证书未过期。可在本机用 `nsh verify --root root.pub <某个本地签出的 .nshpkg>` 确认这套密钥有效。
3. 仓库对匿名用户可见，否则更新器读不到 `latest.json`（见“已知限制”）。
4. 手动跑一次演练，摘要写着“会被接受”，`rehearse` 最后一行是 `Rehearsal signing passed.`。
5. `CHANGELOG.md` 有 `## 0.1.0` 小节，`nsh.json`、`Cargo.toml` 版本一致，`build` 比任何已发布的高。
6. 发布后 `0.1.0` 验证不了更新（没有旧版本可更新）。要验证更新必须再发一个更高的正式版，见 [更新方案 4.5](update.md#45-首次发布d13)。


</details>

<a id="evidence-deploy-11"></a>

<details>
<summary>docs/deploy.md · 迁入记录 12（冻结证据）</summary>


<a id="evidence-deploy-11-已知限制"></a>

#### 已知限制

- 批次 3 至 5 已接入双路径状态机、实际 NSH 检查 / 下载 / 重启调用与产品 UI；兼容代码包要求与外壳主次版本相同。批次 6 已验证隔离本地源的真实整包替换、NSH 自动重启及设置保留，不能替代真实 Release 下载和 Gatekeeper 验收。
- **更新器匿名读取 `releases/latest/download/latest.json`，仓库必须是公开的，或者更新源另行托管。** 2026-11-12 曾记录匿名访问返回 404；该历史结果不能确定当前可见性，发布前须重新确认匿名资源可读。私有仓库的用户无法读取更新，这与更新器的设计无关，是 GitHub 对匿名下载的限制。
- 发布依赖的 GitHub 动作都固定到提交哈希，升级它们要手动改工作流里的值。
- **NSH 命令行的版本只写在 `Cargo.toml` 里**（`nsh-core` 的精确版本），[install-nsh.sh](../scripts/ci/install-nsh.sh) 读它来决定装哪个版本，所以升级 NSH 只改 `Cargo.toml`，工作流里没有第二份版本号，也没有需要同步的校验值。安装时会核对压缩包与 NSH 同目录发布的 `.sha256`，并确认装好的 `nsh` 报告的版本就是要求的版本。它防得住下载损坏和装错版本，**防不住发布者把压缩包和 `.sha256` 一起替换**；要防这个，得把校验值固定在本仓库里，代价是每次升级 NSH 都要同步改一次。
- Ubuntu 作业固定在 `ubuntu-24.04`，不用 `ubuntu-latest`，因为后者正在迁移到 Ubuntu 26，工作流不该在我们没改动时换了系统。
- 历史 CI 中的格式和 NSH 版本不一致问题已修复。2026-11-12 GitHub Release 演练已通过，见更新方案批次 2；演练跳过正式发布，不能证明真实 Secrets 配对、release 环境、草稿转正式、latest 或构建来源证明。本轮不触发远端流水线。


<a id="evidence-deploy-11-更新逻辑批次-3-至-5-的发布边界"></a>

#### 更新逻辑批次 3 至 5 的发布边界

应用采用 A 方案初始化更新服务；Rust 桥接与独立 usher-logging 均属于外壳输入。本轮包含 Rust 与桥接变更，不能作为仅 TSX 代码包发布；版本线门禁继续按更新方案 D9 执行。客户端控制器和产品 UI 已接入；本机桥接测试使用可控原生 I/O，另有本地源签名代码包下载 / 再启动模拟。这两类测试不证明真实 Release；批次 6 另通过 shell-update 命令验证了 macOS arm64 标准变体真实归档的本地替换与失败恢复。该测试使用一次性密钥和无下载隔离标记的 ad hoc 应用，不证明 Apple 公证、Gatekeeper 或线上分发。本轮未修改密钥、发布工作流，也未执行真实发布。



<a id="evidence-deploy-11-批次-6-本地边界结论"></a>

#### 批次 6 本地边界结论

本地验收已覆盖 AppKit 转 standard 和当前账户可写的 `/Applications` 专属安装目录。NSH 0.5.0 仍有两项已复现的安装边界：两次 rename 之间强制终止后需要从外部恢复旧包；有效签名的错误架构归档不会被安装器拒绝。不能把正常替换成功或签名校验通过当成这两项保护已实现。人工 quarantine 标记的 ad hoc 包也被本机 Gatekeeper 拒绝；这不是 Apple 公证验收，也不替代真实浏览器分发。详情与最终报告见更新方案第 13 节。

本轮未操作真实密钥、Release、标签或仓库可见性，正式发布继续保留待办。

</details>

<a id="evidence-scaffolding-12"></a>

<details>
<summary>scaffolding.md · 迁入记录 13（冻结证据）</summary>


<a id="evidence-scaffolding-12-usher-nsh-分批实施清单"></a>

#### Usher NSH 分批实施清单

本文记录分批实施任务与验收状态，模块职责和依赖边界见[架构文档](../architecture.md)。每个子任务有独立编号，跨平台能力按已验证的平台记录。进度依据当前仓库和已有本机验收，更新于 2026-11-12。

**已完成**表示当前条目所列范围已交付。**部分完成**表示已有实现或验证，后续工作列在同一行。**待办**表示尚无可交付的实现或验证结果。


<a id="evidence-scaffolding-12-批次-01--五端启动与开发基线--部分完成"></a>

#### 批次 01 · 五端启动与开发基线 · 部分完成

对应[系统结构](../architecture.md#1-系统结构)和[平台适配与生命周期](../architecture.md#6-平台适配与生命周期)。

| 编号 | 状态 | 任务与验收 |
| --- | --- | --- |
| 01.01 | 已完成 | 固定 NSH v0.5.0 开发基线（已从 v0.4.0 升级：Rust 依赖、CLI 与类型声明同步，窗口 API 改用 `currentWindow().set`，无头与 macOS 真机用例均已重跑）。macOS 已运行 TSX Hello World 和 Rust 混合宿主，保留按钮及桥接示例。见 [工程说明](../README.md)、[应用入口](../App.tsx)和 [Rust 注册入口](../src/lib.rs)。 |
| 01.02 | 已完成 | macOS 本地打包分为 `standard` 与 `appkit`，默认标准包。发布优化统一在 [Cargo 配置](../.cargo/config.toml)，直接运行 `nsh package --target macos` 也已验证读取。见 [打包脚本](../scripts/package-macos.sh)。 |
| 01.03 | 待办 | Windows、Linux 各自在原生 runner 启动、验证 Rust 桥接和按钮，生成可安装产物并确认 release profile 生效。 |
| 01.04 | 待办 | Android 和 iOS 分别在模拟器验证启动、交互、桥接与打包；发布前在真机验收签名安装。 |
| 01.05 | 部分完成 | macOS 已有 `nsh run` 开发入口，TSX 源码 HMR 保留窗口状态及清理旧投影已由 03.11 在隔离目录验收。Rust 或宿主变更重建后恢复偏好、各目标和自维护宿主的开发服务器连接仍待验收；托盘的 HMR 生命周期继续由 02.04 跟踪。 |


<a id="evidence-scaffolding-12-批次-02--原生宿主与系统托盘--部分完成"></a>

#### 批次 02 · 原生宿主与系统托盘 · 部分完成

对应[平台适配与生命周期](../architecture.md#6-平台适配与生命周期)。

| 编号 | 状态 | 任务与验收 |
| --- | --- | --- |
| 02.01 | 已完成 | macOS 全窗使用 AppKit `NSVisualEffectView` 背景材质，标准与增强变体共享实现。已在 macOS 窗口实测玻璃效果，代码已接入实色回退。见 [窗口材质](../platforms/macos/window_material.rs)和[框架布局](../ui/layout/Frame.tsx)。 |
| 02.02 | 待办 | 实现共享托盘菜单模型与生命周期。显示或隐藏窗口、进入设置、检查更新和退出走统一语义动作。覆盖本地化更新、关闭到托盘、托盘失效后的窗口恢复与统一退出。 |
| 02.03 | 待办 | 接入桌面三端托盘。macOS 验收左键恢复、右键或 Control 单击打开原生菜单及 Dock 唤起；Windows 验收 Explorer 重启后恢复；Linux 验收 KSNI/D-Bus 和无托盘降级。 |
| 02.04 | 待办 | 验收菜单取消和隐藏状态、重复退出、清理原生句柄、HMR 后托盘保持单实例。检查更新入口的真实结果随批次 06 验收。 |
| 02.05 | 待办 | 验证 Windows MSIX 包身份。验证 Android 和 iOS 原生返回、前后台恢复及应用内菜单和商店入口。 |


<a id="evidence-scaffolding-12-批次-03--状态设置主题与国际化--部分完成"></a>

#### 批次 03 · 状态、设置、主题与国际化 · 部分完成

对应[状态与设置桥接](../architecture.md#2-状态与设置桥接)、[主题与背景资源](../architecture.md#4-主题与背景资源)、[国际化](../architecture.md#5-国际化)和[本地设置](../architecture.md#本地设置)。

| 编号 | 状态 | 任务与验收 |
| --- | --- | --- |
| 03.01 | 已完成 | 首个 Rust Settings Store 通过类型化命令返回带 revision 的快照，TSX 外观投影由页面共用。主页与设置页切换时共享外观，主页计数保留。见 [设置仓库](../src/settings.rs)、[NSH 桥接](../src/settings_bridge.rs)和 [TSX 投影](../ui/state/appearance.ts)。 |
| 03.02 | 部分完成 | 已按数据目录共享原生设置仓库，当前设置视图通过版本订阅及显式读取获取快照。[通用状态管理方案](state-management.md)已记录独立 `usher-state` crate、可替换后端、NSH 适配及全部跨页面共享数据迁移计划；03.08 核心、03.09 设置接入与 03.10 JS 订阅已完成；其余迁移由 03.11 至 03.13 跟踪。导航、分类与共享预览也迁移到新模块，作用域仍属当前窗口。 |
| 03.03 | 部分完成 | 已用带版本的 TOML 保存外观、语言、侧栏和状态栏偏好，支持原子替换、旧值迁移、损坏备份、未来 schema 保护和失败重试。待加入合并写入 Worker、退出与移动后台限时刷盘，以及其他平台路径、托盘、更新和日志偏好。 |
| 03.04 | 已完成 | 支持 Light、Dark、System、独立强调色和样式主题。所有主题使用同一种 JSON，由 Rust 注册表合并 `extends` 链并替换 `var()` 引用，界面只按 token 名读取，不含按主题 ID 的分支。内置主题在构建时从 [主题目录](../ui/theme)发现，包含 Default、Apple 玻璃、NSH Gallery、Primer、Geist、Slate 和 Radix Sand。见 [主题注册表](../src/style_themes.rs)、[token 投影](../ui/theme/index.ts)和[样式用例](../tests/ui/style-themes.test.tsx)。 |
| 03.05 | 已完成 | 支持本地 JSON 主题发现、继承任一已注册主题、扫描诊断（含基础主题缺失、成环和引用无效）、编辑或复制和选择恢复。IBM Carbon 已作为开发版外部文件在 macOS 验证浅色与深色。当前主题定义保存在 `themes/` 的 JSON，TOML 保存选择并兼容迁移旧内嵌主题。两者分别写入，由设置 Store 管理提交顺序和失败重试。见[主题加载器](../src/style_themes.rs)和[本地主题说明](../README.zh.md#样式主题)。 |
| 03.06 | 部分完成 | TSX 已支持 `zh-Hans`、`en`、系统语言匹配与回退，切换后页面即时更新。待补 TSX/Rust 共用的参数契约、原生菜单和托盘文案、独立格式地区、日期时间与复数规则，以及长文案和系统字号验收。见[文案目录](../i18n/messages)和[翻译入口](../ui/i18n/index.ts)。 |
| 03.07 | 部分完成 | 接入 NSH 官方 Gallery UI 作为可选主题 `builtin.nsh-gallery`：快照固定到上游提交，映射见 [sources/nsh-gallery.json](../ui/theme/sources/nsh-gallery.json)，用 [同步脚本](../scripts/sync-gallery-theme.py)更新，署名见 [NOTICE](../NOTICE)。已带入颜色、圆角、阴影和控件度量（按钮 28、输入框 32、聚焦光晕 3、按压 97%、悬停叠色 5%），每项度量记录上游来源。主题不再只换颜色：度量由 `cssVars.shared` 提供，`ui/theme/look.ts` 统一读取，视图不含写死的尺寸。Slate、Primer、Radix Sand 按各自官方源码给出尺寸，Geist 未公开尺寸故沿用根主题。Rust 单测覆盖度量校验、共享 token、强调色重算和各体系尺寸差异，NSH 用例 `control-metrics` 在布局层断言同一按钮在不同主题下的高度。后续已补：按钮、输入框、卡片的表面质感（`finish-*` token：填充、渐变、描边、多层阴影）和触屏尺寸表（`cssVars.touch`，按 `platform.desktop` 选用），聚焦环改为由注册表推导的 `focus-ring`。2026-11-12 已在 macOS 真机对照官网 Gallery 的按钮和输入框，质感接近；Gallery 的 `finishes` 渐变起止同色，视觉差异来自内高光阴影和描边。未做：Gallery 的组件结构（标签条、分段控件、分体按钮、菜单、面板）。聚焦环在屏幕上的实际对比度未达 3:1，见 [07.05](#evidence-scaffolding-12-批次-07--测试体系与性能基线--部分完成)。 |


<a id="evidence-scaffolding-12-通用状态管理实施子批次"></a>

#### 通用状态管理实施子批次

方案复审及修订见 [复审记录](#evidence-state-management-3-11-方案复审记录)：增加提交失败一致性、桥接合并投递、释放权限和后端接口约束。SM4 的导航迁移由下述 RT1 至 RT3 交付，不能再建第二份历史状态。

详细交付和验收条件以 [通用状态管理方案第 9 节](#evidence-state-management-3-9-分批实施计划) 为准。2026-11-12 已按授权完成 SM1 至 SM3、UT1 的状态部分及 UT2 的设置 / 桥接部分；RT1 至 RT3 与 UT1 路由部分已交付；SM4 已补齐 macOS 原生多窗口 / HMR 验收。

| 编号 | 状态 | 任务与验收 |
| --- | --- | --- |
| 03.08 | 已完成 | SM1：已建立独立 `crates/usher-state/`，提供后端无关的纯转换、快照、读写分离及订阅接口；工厂注入可选 watch 后端，使用相同消费者和契约套件验证替换，不依赖 NSH 与业务 I/O。`python3 scripts/test.py state` 两组合分别通过 20 / 35 项（含编译失败文档测试）；三处临时变异均被捕获；依赖树、工作区回归及 Clippy 通过。证据见状态方案第 10 节。 |
| 03.09 | 已完成 | SM2：`app_state` 注册表和 `SettingsService` 将设置接入 usher-state；目录初始化及 I/O 不持有全局表锁，实例串行提交，Rust 只读快照和订阅独立。桥接 24 个函数签名与 DTO 不变，原 schema / 文件写入规则不变；同 revision 保存重试发布新 sequence，异常关闭后拒绝写入。应用 62 单测 + 11 集成、工作区 172 项和外观无头 2 项通过，详见 [测试记录](#evidence-testing-2-11-sm2-设置应用集成实施记录)。JS 订阅已由 SM3 交付，跨窗口 UI 验收继续由 SM4 跟踪。 |
| 03.10 | 已完成 | SM3：引擎局部有界订阅投递、Stop 清理与会话关闭；TS 传输、版本投影、NSH 响应式适配器和只读借用门面，现有设置入口接入。复用 usher-test-support；Rust 工作区 179 项、TS 投影 11 项、真实桥接 6 项通过，state-ui / ui 命令与 CI 已接入。NSH 0.5.0 无头证据见 [测试记录](#evidence-testing-2-12-sm3-订阅桥接实施记录)；真实多窗口、原生 HMR 与远端 CI 未验收。 |
| 03.11 | 已完成 | SM4：全量现有共享状态已迁移，原每 VM 单一窗口 Store 修正为宿主窗口注册 + Rust StoreId。隔离源码 / 数据目录下的 macOS 同 VM 多窗口偏好同步与会话隔离、真实源码 HMR 保留数据及旧绑定失效、关闭重开共 3 项验收通过；Rust 工作区 207 项。当前完成范围为 macOS 开发宿主，其他平台 / 发布由 SM6 跟踪，见 [原生证据](#evidence-testing-2-14-sm4-原生窗口与-hmr-验收)。 |
| 03.12 | 已完成 | SM5：应用 update_service 适配原更新 Machine，通过 usher-state 发布；单执行租约、效果领取、停止 / 恢复屏障及旧回执拒绝，原 API 和设置格式兼容。更新集成 15 项（10 项各运行两个后端）、应用单测新增 2 项，原更新 38 项及工作区 224 项通过；消息域契约见状态方案第 14 节。NSH 产品桥接 / 实际更新仍归更新批次 4 / 5，本批不启用下载。 |
| 03.13 | 部分完成 | SM6：共享来源审计、后端替换、工作区最新 229 项、全部无头 / UI 与原生 macOS 回归通过；四库七目标编译、应用 Apple 四目标编译通过；隔离发布构建相对 06ed67e 增加 261,882 字节（1.10%）。CI 配置及报告已同步；本轮复验四库七目标、应用 Apple 四目标，并补齐 macOS ad-hoc 发布运行 / VM reload。完整应用跨平台检查缺 C 工具链 / Windows SDK，iOS 缺就绪的 Xcode 组件；其他宿主、远端 CI、分发和实际升级仍待验收。更新模拟已改为唯一测试 ID / 每例临时数据根；新增 16 项隔离清理单测，脚本全量 120 项通过，4 项真实升级启动模拟通过、7 项 pending；普通结束与 SIGTERM 均确认隔离无残留。详见 [验收审计](#evidence-state-audit-1)。 |


<a id="evidence-scaffolding-12-批次-04--导航窗口-shell-与-appkit-控件--部分完成"></a>

#### 批次 04 · 导航、窗口 Shell 与 AppKit 控件 · 部分完成

对应[窗口与导航](../architecture.md#3-窗口与导航)和[平台适配与生命周期](../architecture.md#6-平台适配与生命周期)。

| 编号 | 状态 | 任务与验收 |
| --- | --- | --- |
| 04.01 | 已完成 | 桌面 Shell 首期实现自绘标题栏、侧栏停靠与抽屉、主页与设置类型化路由、前进后退和快捷键、可点击面包屑及窄窗折叠。窗口框架后来迁入 `ui/layout/`，见 04.07。见[路由](../ui/router/index.ts)、[框架布局](../ui/layout/Frame.tsx)和 [Shell 用例](../tests/ui/window-shell.test.tsx)。 |
| 04.02 | 已完成 | 底部状态栏显示当前应用版本与设置保存状态，开关即时生效并落盘，隐藏后释放布局空间。见[底部状态栏](../ui/layout/components/StatusBar.tsx)和 [Shell 用例](../tests/ui/window-shell.test.tsx)。 |
| 04.03 | 已完成 | macOS 开发宿主默认编入 AppKit 设置控件，支持的系统可在同一窗口切换控件族和样式，两套控件复用 Rust 设置。标准包及不可用环境回退到 NSH 控件。见 [AppKit 控件](../platforms/macos/controls.rs)和[切换用例](../tests/ui/appkit.test.tsx)。 |
| 04.04 | 待办 | 补路由参数、可用条件、动态标题、更新页面和原生动作接入。验收移动导航栈、系统返回优先关闭浮层、安全区和系统字号。 |
| 04.05 | 待办 | 补 AppKit 切换时的草稿保留、焦点恢复、浮层释放与失败回退验收。更新任务的状态栏投影随批次 06 接入。 |
| 04.06 | 部分完成 | 设置页改为分类导航：宽窗口左侧是分类列表、右侧是当前分类内容，窄窗口（页面宽度低于 560pt）列表换成一个下拉框，通用、主题、诊断三个分类。分类选择保存在外观 Store 中，离开设置页再回来保持不变，窗口缩放时分类不丢。NSH 控件族使用框架 `Tab` / `Tabs` 和 `Select`，选中态由框架指示层配合主题底色、边框和阴影绘制；液态玻璃（AppKit）版本宽屏用新增的原生组件 `usher.settings.sidebar`（`NSButton` 工具栏样式的纵向列表），窄屏用已有的 `NSPopUpButton`。各分类内容改为分组卡片（小标题加圆角卡片，行内左侧名称与说明、右侧控件，行间分隔线），行内控件沿用各控件族自己的组件。见 [设置页](../ui/features/settings/SettingsPage.tsx)、[分类导航](../ui/features/settings/SettingsNavigation.tsx)和 [布局用例](../tests/ui/settings-layout.test.tsx)；用例在无头宿主与 `--target macos` 都通过。待验收：分组卡片布局在真实宿主中的观感（含 AppKit 控件族与窄窗口折行）、真机上原生列表的外观与选中态、键盘导航与焦点顺序、系统字号放大后的折行。 |
| 04.07 | 部分完成 | 新建 `ui/layout/` 管理框架层布局：通栏标题栏（菜单开关、前进后退、面包屑、右端浅色/深色切换，设置入口在窄菜单底部和二级菜单）、始终显示的窄菜单（底部为更新、设置、用户头像按钮）、可显示或隐藏的二级菜单、内容区（二级菜单隐藏时带动画向左延伸）和与窗口等宽的底部状态栏。整条标题栏可拖动（拖动行为需在真机确认），工具栏图标和按钮尺寸取主题 `size-toolbar-icon`、`size-toolbar-button`，图标与菜单文字使用正文色而非弱化色。二级菜单是带圆角的独立面板，底色和选中色来自主题 `menu-surface`、`menu-selected`（取值按参考图实测）。尺寸与动画时长来自主题 `layout-*`、`motion-layout` 度量，Rust 注册表新增 `layout-` 族校验。更新与头像按钮只定义了接口（[updates.ts](../ui/layout/contracts/updates.ts)、[account.ts](../ui/layout/contracts/account.ts)），功能接入前保持禁用。见[框架布局](../ui/layout/Frame.tsx)和[布局用例](../tests/ui/layout.test.tsx)；`nsh check`、`cargo test --locked --workspace` 和全部界面用例在无头宿主通过，布局、窗口 Shell、设置页和 AppKit 用例也在 `--target macos` 通过，并看过 macOS 宿主截图。待验收：展开与收起二级菜单时的动画观感、全屏下的标题栏、窄窗抽屉、AppKit 控件族下的窄菜单、更新与头像功能接入。 |
| 04.08 | 部分完成 | 通用设置新增用户背景图片选择、移除与 0–100% 不透明度，默认 30%，使用框架 `Picker.pickImages` 和 `Slider`。`ui/layout/components/WindowBackdrop.tsx` 统一装配毛玻璃、主题叠色与图片层，图片位于所有交互区域之下，铺满窗口且不拦截点击。Rust Store 保存元数据与透明度，`src/background.rs` 将图片复制到本地，设置提交成功后清理旧资源；保存失败重试复用候选 ID，无数据目录时保留会话内图片。尺寸与导入限制由 `ui/config/background.json` 共享。已通过 `nsh check`、格式检查、Clippy、Rust 工作区 114 项测试、[背景设置用例](../tests/ui/background.test.tsx)（3 项：用户选择、取消、实时透明度预览、跨页面保持、非法图片、读取失败回退与移除）、[背景布局用例](../tests/ui/background-layout.test.tsx)在无头与 macOS 宿主运行，并查看 macOS 30%/100% 图片层截图；相关外观、布局、设置分类和主题度量回归通过。待验收：系统选择器人工选图、应用重启后的原生显示、Windows/Linux/Android/iOS 的真实宿主行为。 |
| 04.09 | 部分完成 | 通用设置新增内容区域背景透明度，默认 0%，100% 时移除次级菜单栏（含抽屉）、右侧内容面板和设置分组卡片的底色，露出窗口背景图片；只降低填充色 alpha，保留文字、控件与阴影。共享配置定义范围与步长，Rust Store 校验并独立持久化，兼容旧设置，拖动实时预览、释放后保存。`nsh check`、格式检查、Clippy、Rust 工作区 115 项测试及背景、背景布局、设置分类共 10 项无头用例通过；macOS 背景交互用例通过，已查看浅色 0%/42%/100% 及深色 100% 的真实窗口截图。窄窗抽屉通过无头用例验证。待验收：其他平台真实宿主显示。 |

2026-11-12 标题栏与窄菜单提示：八个图标按钮通过 [ButtonTooltip.tsx](../ui/layout/components/ButtonTooltip.tsx) 直接挂载 NSH `Tooltip`，包含禁用的前进后退、更新和账户按钮；面包屑不添加提示。文案复用中英文消息目录，随语言、侧栏和明暗切换动作更新。标题栏优先向下、窄菜单优先向右，框架根浮层负责安全区域内的翻转、平移和层级，公开 `maxWidth` 样式限制宽度并允许长译文换行，样式与边距复用现有主题 token。`nsh check`、Rust 工作区 116 项测试及 [Tooltip 用例](../tests/ui/tooltips.test.tsx) 3 项在无头与 macOS 宿主通过，布局 5 项与窗口 Shell 2 项无头回归通过；已查看 macOS 浅色英文和深色中文截图，覆盖右上角、窄菜单底部及 380 × 500 点窄窗口，未见提示越界或被内容面板遮挡。其他平台真实宿主待验收。

2026-11-12 固定菜单底部图标调整：更新、设置和默认头像统一读取 `size-rail-utility-icon`（18 点），顶部主页保留 `size-rail-icon`（24 点）。全部窄菜单图标通过现有 AppKit 适配器绘制系统 SF Symbols，配置集中在 `ui/config/navigation-icons.json`；默认头像使用线框 `person`，设置继续按导航状态切换线框／实心。`nsh check` 和布局 5 项用例通过，已查看 macOS 浅色主页、浅色与深色设置页截图。

2026-11-12 导航图标与 Select 反馈调整：主页和设置从 `ui/config/navigation-icons.json` 读取成对 SF Symbols，选中使用 `house.fill` / `gearshape.fill`，未选中使用对应线框，窄菜单、次级菜单与标题栏设置按钮共享。设置分类下拉框按 [NSH Select 官方接口](https://nsh.dev/elements/select/)使用 `itemClass` 的 `focus:` 状态，鼠标与键盘高亮由框架管理；`selectLook` 从当前主题读取 `item-hover`，兼容带空格的 CSS 颜色函数。`nsh check`、布局 5 项与设置分类 6 项无头用例通过；macOS 实窗检查覆盖主页／设置与前进后退、NSH Gallery 明暗模式的鼠标移项和方向键高亮、回车选择，已查看截图。其他宿主沿用字符图标回退，原生视觉效果待验收。

2026-11-11 窄菜单与状态栏调整：窄菜单图标使用独立主题度量 `size-rail-icon`，默认 24 点；浅色菜单选中底色按用户参考图取样为 `#eaeaeb`，集中在 `menu-selected`。状态栏通过框架 `content-center` 居中文本行，配合 `items-center` 保持行内对齐。`nsh check`、Rust 工作区 115 项、布局 5 项和窗口 Shell 2 项无头用例通过。已查看 macOS 主页与设置选中态、明暗模式及中英文状态栏截图；其他平台实际观感待验收。

2026-11-11 设置卡片与分类导航调整：通用、主题和诊断分组卡片通过 `groupLook` 复用内容背景透明度，只调整底色 alpha。分类导航按 [NSH 官方 Tabs 文档](https://nsh.dev/elements/tabs/)改用 `Tab` / `Tabs`，选中底色、边框和阴影交给单一原生指示层，键盘焦点通过 `focusVisibleStyle` 呈现；框架内部代码未修改。新增设置页键盘用例，覆盖方向键、Home/End、循环选中及 Tab 一步进出列表。`nsh check`、背景 4 项、设置分类 6 项和背景布局 1 项无头用例通过，设置分类 6 项 macOS 用例通过；已查看 macOS 明暗分类切换和卡片 0%/部分/100% 透明度截图，连续切换后未见旧项黑框。其他平台真实宿主待验收。

2026-11-11 布局目录整理：`ui/layout/` 根目录保留 `Frame.tsx` 装配入口，视图、控制逻辑和接口分别归入 `components/`、`controllers/`、`contracts/`，共迁移 11 个文件并同步导入与文档。`nsh check` 及无头布局、窗口 Shell、背景布局共 8 项用例通过。

2026-11-11 外观调整：Default 与 NSH Gallery 的浅色毛玻璃窗口使用中性浅灰叠色，内容面板为白色；NSH Gallery 的 `panel` 映射到 `card`，窗口叠色与 Shell 阴影继承根主题。次级菜单使用主题 `shell-shadow`，停靠时允许边缘阴影溢出，收起时裁剪。macOS 布局与设置分类用例各 5 项通过，并查看浅色、深色与图片叠加截图。

2026-11-11 窗口尺寸调整：macOS 默认启动外框及最小外框均为 800 × 600 点，由[原生窗口适配](../platforms/macos/window_geometry.rs)按窗口实例设置，TSX 重挂载不重置尺寸。已通过 `nsh check`、`cargo test --locked --workspace`（71 项）及 macOS 宿主 Shell 用例（2 项）；拖动窗口边缘的最小尺寸限制和 HMR 尺寸保留尚未手工验收。


<a id="evidence-scaffolding-12-独立路由实施子批次"></a>

#### 独立路由实施子批次

详细设计与选型见 [路由方案](routing.md)。RT1 至 RT3 已交付代码和无头验证，现有页面已迁移；SM4 macOS 原生验收已完成，RT4 的目录升级、深链接及 macOS 原生返回已实施并验证；本机 macOS ad-hoc 发布包深链接与完整 VM reload 已通过，其他平台和分发 / 升级仍待验收。

| 编号 | 状态 | 任务与验收 |
| --- | --- | --- |
| 04.10 | 已完成 | RT1：独立 usher-router、可替换 matchit 适配、路由目录与生成契约、纯解析及历史转换测试；无 NSH 和业务页面依赖。 |
| 04.11 | 已完成 | RT2：WindowService / WindowState 桥接将历史与 session 放入唯一窗口 Store；目录注册、守卫取消 / 失败 / 重定向、乱序票据和关闭通过 8 项应用集成测试，复用 usher-test-support。 |
| 04.12 | 已完成 | RT3：ui/pages 集中目录 / 页面注册与生成检查，ui/router 门面和 RouterView；首页、设置、菜单、快捷键、面包屑及历史已整改，旧 navigation 删除。门面 mock 4 项、RouterView 错误注入 1 项、真实桥接 3 项、实际 App 旅程 2 项及原有 UI 29 项通过；SM4 原生验收已由 03.11 完成，详见[整改记录](#evidence-routing-4-10-rt2--rt3-与现有页面整改记录)。 |
| 04.13 | 部分完成 | RT4：usher://app/ 深链接、原生返回与异步守卫、原子目录迁移 / HMR 和旧绑定隔离已实施；229 项 Rust、11 项 router-ui、4 项无头与 4 项隔离 macOS 验收通过。公开 NavStack 能力已检查，当前不启用 iOS 交互式返回。macOS 发布包系统注册、外部冷启动、隐藏恢复和同进程前台唤醒完整 3 项通过，临时数据与系统注册已清理。完整 Cmd+R VM reload 1 项通过。iOS 模拟器因 Xcode 首次启动 / CoreSimulator 版本不匹配阻塞，未修改设备；其他宿主与分发回归待办，外部设备/runner 暂缺，见 [路由记录](routing.md)。 |


<a id="evidence-scaffolding-12-批次-05--诊断日志--部分完成"></a>

#### 批次 05 · 诊断日志 · 部分完成

对应[诊断日志](../architecture.md#8-诊断日志)，详细设计与迁移批次见 [docs/log.md](log.md)。独立日志门面、可替换适配器和全部既有调用迁移已完成；诊断页接通应用文件、控制台和结构化诊断开关，终端 / 内存禁用并标记尚未接入，更新分区移至诊断之前。规则、公共契约、应用集成与原生验收按项目分层组织，实际结果见[测试方案第 26 节](#evidence-testing-2-26-日志门面类型控制与诊断页2026-11-13)。下表保留未完成的平台与异常场景，不把本次重构等同于全部平台验收。

| 编号 | 状态 | 任务与验收 |
| --- | --- | --- |
| 05.01 | 部分完成 | 已实现 `ERROR` 至 `TRACE` 及 `OFF` 的过滤器，等级写入 `[diagnostics]` 并随设置恢复，修改后即时作用于运行中的写入器。TSX 只能上报短标识的模块名和事件名，不能带自由文本。单测已验证等级默认 `INFO`、写入后重启恢复，默认值不写入文件。设置页用例已通过。已在真实 macOS 宿主验证：启动写入 `diagnostics_started`，重启后恢复保存的等级，切换等级写入变更记录。随后发现切到 `OFF` 时该记录被过滤器吃掉，已改为变更记录不受过滤器限制并带 `from`/`to`，这一版只有单测验证，尚未在真实宿主重看。待验收：改后的变更记录在真实宿主的内容、文件权限为 `0600`、HMR 后写入器唯一。 |
| 05.02 | 部分完成 | 已实现有界队列、后台单线程写入、按事件本地日期分桶、8 MiB 分片、14 天保留与磁盘配额，目录不可写时记录健康状态而不影响应用。警告和错误在队列满时等待入队，低级别日志丢弃并计数。保留清理在每天首次写入时运行，不只在启动时。单测已通过，覆盖跨午夜、等级切换、分片、清理、不可写目录和队列满。退出时通过 `atexit` 冲刷队列，单测验证排队的记录在冲刷后已落盘。待验收：磁盘满、真实应用 Quit 后日志尾部完整。 |
| 05.03 | 部分完成 | 已实现按日期范围导出 `.tar.gz`：先写同目录临时文件再改名发布，取消或失败不留半成品；跳过符号链接；单文件 8 MiB、总计 64 MiB 上限，超限时优先保留最新日志；导出目录只留最近 5 个归档。导出逻辑的单测已通过（日期范围、符号链接、截断、取消与失败清理）。新增应用集成验证关闭输出后仍能导出两类历史日志。目前导出到应用数据目录下的 `exports/` 并打开该目录，未接系统保存对话框；归档数量上限（保留最近 5 个，刚写入的永远保留，无关文件不动）和归档命名唯一有序已有单测。待验收：真实应用点导出后文件夹打开且归档内容正确、桌面保存对话框、Android 与 iOS 系统导出入口。 |
| 05.04 | 已完成 | Logger / Level / Fields / LogKind 稳定门面，runtime 装配与 LogWriter / ArchiveProvider 后端接口；底层辅助函数私有，健康、设置、更新及退出调用迁移。20 项模块单测、13 项公共契约通过，覆盖四类型 × 六阈值 × 六提交等级、32 种开关配置、脱敏、动态配置与故障，真实文件与 capture 对照。 |
| 05.05 | 已完成 | 诊断页按输出 / 详细程度 / 专项诊断 / 归档分组，更新与诊断换序，三个开关持久化，两个不可用采集项禁用。真实桥接、30 项完整 UI 回归通过；隔离 macOS 宿主确认宽窄屏、等级选择和归档区显示，5 项原生套件通过。终端 / 内存真实采集不在本次范围。 |

2026-11-13 界面反馈：移除设置标题下的实时保存状态；日志开关保存时保持已接入项的外观，避免其它开关跟随灰显，仍保留防重复提交。真实 macOS 保存前 / 等待中截图内容区像素一致，专项 2 项通过；回归记录见 [测试方案第 27 节](#evidence-testing-2-27-设置标题状态与日志开关闪烁修正2026-11-13)。

**与同级 Usher 的有意差异**：等级可运行时修改，Usher 的 `fern` 只能重启生效；单一写入线程取代多个 sink 和两套互相转发的日志库；崩溃记录只写一条脱敏的 `ERROR`，不替换宿主的 panic 行为也不主动请求关闭；日志文件权限为 `0600`；预算用尽时丢弃最旧而不是跳过其后的文件。


<a id="evidence-scaffolding-12-批次-06--检查更新渠道升级与版本说明--部分完成"></a>

#### 批次 06 · 检查更新、渠道升级与版本说明 · 部分完成

2026-11-13 更新批次 5：A 初始化下的控制器、定时检查、更新设置和菜单栏浮层已接入；216 项工作区测试、38 项 state-ui、29 项 UI、4 项无头旅程和 120 项脚本测试通过，Clippy 与 nsh check 通过。隔离 macOS 检查了进度环、浮层及组件热更新期间的下载状态；完整 VM 销毁仍须重启应用。真实代码包模拟与剩余整包 / 发布验收见更新方案第 13 节和测试方案。

2026-11-13 更新界面整改：自动代码包下载显示进度环，就绪图标采用主题配置的系统下载图标（后续按反馈改为 14pt 的 square.and.arrow.down，浮层关闭入口移至右上角 ×）；设置页和浮层的说明换行、操作排列已在隔离 macOS 宿主核对。state-ui 40 项、UI 29 项和工作区 216 项通过，记录见测试方案第 23 节。

2026-11-13 更新入口显隐：无更新隐藏，检查中使用无中心图标的灰色旋转细环，有更新显示系统下载图标及“有可用更新”提示；无候选检查失败不显示警示入口。state-ui 41 项、UI 29 项、nsh check 和隔离原生显隐 / 动画核对通过，见测试方案第 24 节。

2026-11-13 检查失败反馈补充：后台失败静默；手动失败保留左侧更新警示入口，弹层内重试失败仍保留原因和重试，关闭弹层不隐藏入口，确认已是最新后隐藏。state-ui 42 项（含更新界面 8 项）和 nsh check 通过，见测试方案第 25 节。 同日按反馈恢复实心警示色下载按钮，保留显隐与重试逻辑，20 项更新专项测试及 nsh check 通过。

2026-11-13 更新操作补充：检查按钮内显示加载圆环并保持 disabled，浮层下载与检查按钮同排；布局与交互要求集中到 docs/interaction-design.md。nsh check、state-ui 44 项、UI 31 项及隔离 macOS 原生专项 1 项通过，截图已核对，见测试方案第 28 节。


对应[更新子系统](../architecture.md#7-更新子系统)和[发布信任边界](../architecture.md#发布信任边界)。更新逻辑独立成内部 crate [usher-update](../crates/usher-update)，没有 NSH 依赖，改动后只重编这个 crate。

**本批次以 [应用更新方案](update.md) 为准**：NSH 原生信任、单一正式版通道、代码包 / 整包两条路径。批次 3 至 5 已按 A 方案实施，控制器、更新设置分区与菜单栏浮层已接入。

| 编号 | 状态 | 任务与验收 |
| --- | --- | --- |
| 06.01 | 部分完成 | 已有 `nsh.json` 版本和构建号、状态栏版本显示及 [CHANGELOG 文件](../CHANGELOG.md)。待统一运行构建身份与关于弹窗、包元数据及 Release 附件，补版本说明生成、升级公告、首次升级提示和版本一致性门禁。 |
| 06.02 | 部分完成 | 双路径状态机、失败预算、静默包下载策略和 A 初始化服务已实现；UpdateState 桥接、默认开启的 auto_check 持久化、单执行租约 / 领取与稳定错误码已交付。没有取消或执行者交接；独立 usher-logging 已接入更新记录。批次 5 已接入 TSX 单执行者、定时检查、设置分区、进度环和面板；批次 6 已验证 macOS arm64 标准变体的本地真实整包替换、自动重启与设置保留；原生托盘入口和正式发布验收仍待办。 |
| 06.03 | 部分完成 | 信任边界已改为 NSH 原生清单、代码包签名和运行时信任库；旧 signature.rs、trusted_keys.txt、usher-update-tool 已删除。发布脚本已有演练，真实发布与密钥轮换分发验收仍按更新方案 / deploy 推进。 |
| 06.04 | 部分完成 | 已用 NSH 自带工具链实测签名包：`nsh keygen`、`certify`、`publish`、`verify`、`revoke` 都可用，真实应用打出的 `.nshpkg` 约 262 KB。验证了根密钥正确时通过、换根拒绝、改动一个字节拒绝、发布密钥被吊销后拒绝、轮换到新发布密钥后的新构建仍通过。**2026-11-12 更正**：早期认为应用无法自行加载代码包的结论基于 NSH 0.4.0，在 0.5.0 上已不成立。0.5.0 的 `nsh_macos_app::main()` 调用 `nsh_engine::updates::launch`，应用能自己加载并试运行 `.nshpkg`，失败时回滚到内置代码；已用一次性密钥签的真实包在打包出的应用上实测完整周期，详见 [应用更新方案](#evidence-update-5-11-验证记录与待验证项) V3。macOS 整包层不再接 Sparkle：NSH 的 `nsh-updater` 自己替换 `.app`，Sparkle 设想作废。 |
| 06.05 | 部分完成 | macOS arm64 真实归档替换、NSH 自动重启、设置保留和原有失败恢复已验收；本地续验补充 AppKit 转 standard、当前账户可写的 /Applications 专属目录及检查前权限回退。已复现 NSH 0.5.0 的 rename 间隙缺少自动恢复、错误架构未拒绝；人工 quarantine 标记的 ad hoc 包被 Gatekeeper 拒绝。正式分发、真实首次打开、管理员所有权 / ACL 组合及 V9 产品决定仍待完成。Apple 公证按 D6 暂缓，其他平台不在当前范围；详见更新方案批次 6。 |
| 06.06 | 部分完成 | 发布流水线已按 NSH 0.5.0 单一正式版通道实施，门禁拒绝预发布标签；standard / appkit 构建、版本核对、临时密钥演练、签名清单与先草稿后发布流程见 [部署与发布](deploy.md)。2026-11-12 GitHub 演练通过，详情见更新方案批次 2。真实 Secret 配对、release 环境、草稿转正式、latest 与构建来源证明仍待正式发布验收，本轮不执行。Apple 签名 / 公证按 D6 暂缓；Windows / Linux 与 Intel Mac 不属于当前发布范围。NSH 版本由 Cargo.toml 统一提供；lefthook 与 CI 门禁已接入，历史测试数量不代表当前完整验收。 |
| 06.07 | 待办 | 增加更新公告，直接从仓库新版本对应的 changelog 获取内容，按更新通道与目标版本展示，复用已有发布记录作为单一内容来源。接入发现新版本及首次升级后的公告入口，验收版本匹配、已读去重、内容缺失、获取失败与重试。 |
| 06.08 | 待办 | 建立全局消息通知系统，用于实时接收服务端下发的公告与通知，提供跨页面共享的消息状态与统一展示入口，支持已读、关闭和关联动作。消息接收与界面展示通过统一契约衔接；更新公告内容仍按 06.07 直接获取仓库新版本的 changelog。验收实时送达、断线重连与漏收补取、重复消息去重、已读状态持久化、跨页面展示及连接与订阅清理。 |

2026-11-13 更新批次 6：macOS arm64 标准变体的真实整包归档五场景完整通过，含安装后 NSH 自动重启、设置保留和失败恢复；应用身份错误误报磁盘的问题已修复。state-ui 39 项、脚本 135 项通过，nsh check / actionlint 通过，隔离路径与进程复核无残留。正式分发及 Gatekeeper 等剩余边界见更新方案第 13 节。

2026-11-13 批次 6 续验：统一 shell-update 完整运行 11 项通过（8 功能、3 边界），脚本 138 项、state-ui 45 项及 nsh check 通过；48 个隔离路径与测试进程复核无残留。最终证据为 shell-update-w6rsgapu。rename 间隙恢复、内部架构校验两项缺口已确认，人工 quarantine 下 Gatekeeper 拒绝 ad hoc 包；技术可行性不替代 AppKit 转 standard 的产品决定，正式发布未执行。

2026-11-13 后续安排：用户决定将更新遗留问题先记录、后续再处理。替换中断恢复、内部架构校验、真实 Gatekeeper 分发、管理员 / ACL 权限、AppKit 变体取舍及正式发布分别登记为 [更新后续待办 U6-01 至 U6-06](#evidence-update-6-批次-6-后续待办)。本轮不继续修复或发布，批次 6 保持部分完成。


<a id="evidence-scaffolding-12-批次-07--测试体系与性能基线--部分完成"></a>

#### 批次 07 · 测试体系与性能基线 · 部分完成

验证要求见[开发与贡献规范](../CONTRIBUTING.md#验证与交付)。测试随对应功能交付，本批负责补齐跨层覆盖、平台验收和性能作业。

新增设计见 [测试方案](testing.md)，已调研 Rust Book、Tokio、Vue Router、Proptest 与 Playwright 的 GitHub 官方资料。UT1 状态和路由部分已完成；UT2 应用 / 桥接与 TE 无头部分已交付，原生验收和剩余覆盖继续按下表跟踪。

| 编号 | 状态 | 任务与验收 |
| --- | --- | --- |
| 07.01 | 部分完成 | 已有 Rust 设置、迁移和主题单测，以及外观、主题、导航、AppKit 的 [NSH UI 用例](../tests/ui)。新增领域随实现补 reducer、更新策略、日志过滤和按日分桶单测。 |
| 07.02 | 部分完成 | 已交付设置、窗口 / 路由、更新应用集成及隔离 / 离线夹具，覆盖动作到快照、设置落盘、更新执行交接；CI 已接入。五端原生烟测、实际更新 / 导出产物旅程及签名验收仍待对应批次。 |
| 07.03 | 待办 | 在 `benches/` 建立微基准，测量派发、快照投影、列表转换和日志格式化，固定输入并保存结果。 |
| 07.04 | 待办 | 在 `tests/performance/` 建立发布宿主性能作业，测冷启动、交互、帧时间、CPU、峰值内存、托盘长驻与恢复、移动前后台及资源释放。在同规格 runner 上保存原始样本、P50/P95 和平台基线，再定义回归门禁。 |
| 07.05 | 待办 | 聚焦环在屏幕上的实际对比度。注册表按主题自己的前景色与背景色推导 `focus-ring`（[derive_focus_ring](../src/style_themes.rs)），Rust 单测 `every_bundled_focus_ring_reaches_three_to_one_on_every_surface` 保证该 token 对 background、card、panel 三种表面不低于 3:1，推导时另外参考 sidebar 与 menu-surface。这只保证 token，不保证屏幕。2026-11-12 在 macOS 真机（NSH 0.5.0）对 Default、NSH Gallery、Primer、Slate 的浅色和深色做 Tab 前后截图差分，取变化像素作为环，实测对比度：按 Default、Gallery、Primer、Slate 的顺序，浅色 3.86、3.85、2.70、4.23，深色 2.76、2.61、2.13、2.83。深色四项和 Primer 浅色低于 3:1；Apple Glass、Geist、Radix Sand 未截图。已知现象：屏幕上的环比 token 暗，三通道按近似同一比例缩小：Default 浅色 0.92，Default 深色 0.90，Primer 深色约 0.90，Gallery 深色约 0.86，各样本不一致，不是固定倍数。例：Default 深色 token `#707070`，屏幕 `#656565`。变暗在浅色底上会抬高对比度，在深色底上会压低对比度，所以浅色大多达标而深色全部不达标。来源未查明，可能是颜色空间转换，也可能是 outline 的混合方式。不确定之处：对比度的参照色取的是 Tab 前同一位置的像素，outline 落在控件边缘时它可能是控件自己的边色而不是页面表面（Primer 浅色取到 `#ccd3db`，深色取到 `#2b2f35`，都不在已检查的表面里），所以 Primer 两项的数值可能偏低，需要用页面和卡片的真实颜色复核。另有一处观察：各控件聚焦偏移不统一，列表项使用负偏移，按钮不是。待办：1. 用已知 `outlineColor` 的纯色方块在浅色、深色底上截图，建立设定色到屏幕色的映射，确认压暗的原因。2. 按映射反推推导公式，并把验收改为对显示色断言：截图差分用例固定在 `tests/ui/`，覆盖七个内置主题的浅色和深色，参照色取页面、卡片、面板的真实像素。3. 统一聚焦偏移。4. 在 Windows 和 Linux 宿主复核同一映射，不假设与 macOS 相同。 |
| 07.06 | 已完成 | UT1：状态两后端契约保留，RT1 新增独立路由参考 / matchit 同契约、固定 seed 属性、生成器拒绝及历史溢出测试。router 无默认 feature 9 项、matchit 14 项通过，state 20 / 35 项通过；共用测试支持 6 项，脚本 96 项通过。命令与 CI 已接入，远端 CI 未运行。 |
| 07.07 | 部分完成 | UT2：设置 11 项、窗口 / 路由 8 项、更新 15 项应用集成，TS 投影、真实订阅 / 路由桥接、可控守卫门面、生成新鲜度检查及命令 / CI 已交付。RouterView 重试 / 返回错误注入已通过；统一跨语言 DTO fixture 目录仍待补齐，真实桥接另行验证生产 DTO。 |
| 07.08 | 部分完成 | TE1：无头及 macOS 原生隔离数据目录、真实窗口 / HMR、清理、JUnit / JSON 已交付；e2e 支持 headless 和 macos。原生命令已配置手动专用图形 runner 作业，远端未运行；跨进程重启和其他平台仍待验收。 |
| 07.09 | 部分完成 | TE2：2 项实际 App 无头旅程、桥接 / UI 回归与 3 项 macOS 原生窗口 / HMR 验收已交付。真实鼠标 / 键盘全产品旅程、长导出文件任务、跨进程重启及发布产物仍待对应批次，详见测试方案第 14 节。 |
| 07.10 | 已完成 | 通用测试支持：独立 usher-test-support 无外部及业务依赖，仅开发依赖使用；复用 WakeCounter / poll / ready 与夹具身份校验。消费方为状态、路由及应用测试；业务 mock / fixtures 留在所属模块。当前工具 6 项、工作区 224 项通过，远端 CI 未运行。原抽取记录见 [第 10 节](#evidence-testing-2-10-通用测试支持抽取实施记录)。 |


<a id="evidence-scaffolding-12-状态--路由审查整改2026-11-13"></a>

#### 状态 / 路由审查整改（2026-11-13）

代码已完成：删除闲置状态机 trait 和通用发布预检；设置服务自行检查容量并保留持久化故障契约；WindowService 与 window_session 明确职责，回执 / 票据 / 服务错误类型化；路由目录索引、祖先缓存和分阶段解析；生成强类型路由元数据；TS 守卫流程拆分与基础设施错误分类修正。旧 session 桥接仍兼容。新增回归覆盖故障重试、目录 / 守卫原子性、票据精度、桥接双协议及导航错误分类。

本批验证结果记录在 docs/testing.md 的“状态与路由审查整改”节；不扩展 RT4 其他平台、SM6 原生 / 发布成本验收范围。


<a id="evidence-scaffolding-12-异步按钮交互整改2026-11-13"></a>

#### 异步按钮交互整改（2026-11-13）

文件选择与读写、主题扫描与保存、诊断导出和更新入口接入按钮内部加载图标与加载期间禁用；主题、明暗模式、语言及导航等即时切换不显示 loading。加载图标预留空间，保持按钮文案、尺寸与控件实现稳定，最左侧窄菜单栏保持现状。按钮事件保留 Promise，长任务继续投影现有业务状态。本次尺寸与反馈范围修正通过 nsh check、UI 40 项及 state-ui 45 项；原生取消流程已补齐：测试宿主使用 AppKit 公共动作取消真实面板，连续两次验证取消、关闭与状态恢复，完整原生 7 项及无头按钮专项 7 项通过，0 失败 / 超时 / 跳过，并接入 e2e-macos。按钮尺寸与即时主题切换的原生验证通过；具体证据及前轮结果见 docs/interaction-design.md 和 docs/testing.md 第 29 节。


<a id="evidence-scaffolding-12-nsh-gallery-组件主题审计2026-11-13"></a>

#### NSH Gallery 组件主题审计（2026-11-13）

已核对生产界面使用的 NSH 组件，补齐部件表面、配色与尺寸映射，详见 [主题审计](#evidence-theme-audit-0)。本项不代表官网动效或跨平台像素一致验收完成；验证结果以审计文档为准。

2026-11-13 动效补齐：Switch 原生弹簧、Slider 悬停、Progress 扫动、Popover / Tooltip 进出及 Sheet 抽屉已接入；Select 进入动画已接入，退出保留框架默认行为。验证：`nsh check`、241 项 Rust、44 项 UI 和 2 项隔离 macOS 原生验收通过。原生中间帧与减少动效验收纳入 `e2e-macos`，详见 [主题动效审计](#evidence-theme-audit-0-动效补齐2026-11-13)。

</details>
