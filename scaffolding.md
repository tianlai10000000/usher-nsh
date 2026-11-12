# Usher NSH 分批实施清单

本文记录分批实施任务与验收状态，模块职责和依赖边界见[架构文档](architecture.md)。每个子任务有独立编号，跨平台能力按已验证的平台记录。进度依据当前仓库和已有本机验收，更新于 2026-11-12。

**已完成**表示当前条目所列范围已交付。**部分完成**表示已有实现或验证，后续工作列在同一行。**待办**表示尚无可交付的实现或验证结果。

## 批次 01 · 五端启动与开发基线 · 部分完成

对应[系统结构](architecture.md#1-系统结构)和[平台适配与生命周期](architecture.md#6-平台适配与生命周期)。

| 编号 | 状态 | 任务与验收 |
| --- | --- | --- |
| 01.01 | 已完成 | 固定 NSH v0.5.0 开发基线（已从 v0.4.0 升级：Rust 依赖、CLI 与类型声明同步，窗口 API 改用 `currentWindow().set`，无头与 macOS 真机用例均已重跑）。macOS 已运行 TSX Hello World 和 Rust 混合宿主，保留按钮及桥接示例。见 [工程说明](README.md)、[应用入口](App.tsx)和 [Rust 注册入口](src/lib.rs)。 |
| 01.02 | 已完成 | macOS 本地打包分为 `standard` 与 `appkit`，默认标准包。发布优化统一在 [Cargo 配置](.cargo/config.toml)，直接运行 `nsh package --target macos` 也已验证读取。见 [打包脚本](scripts/package-macos.sh)。 |
| 01.03 | 待办 | Windows、Linux 各自在原生 runner 启动、验证 Rust 桥接和按钮，生成可安装产物并确认 release profile 生效。 |
| 01.04 | 待办 | Android 和 iOS 分别在模拟器验证启动、交互、桥接与打包；发布前在真机验收签名安装。 |
| 01.05 | 部分完成 | macOS 已有 `nsh run` 开发入口。待补 TSX 热替换保留状态、Rust 或宿主变更重建后恢复偏好的项目验收，验证各目标和自维护宿主的开发服务器连接。Store 与托盘的 HMR 生命周期分别在 03.02 和 02.04 验收。 |

## 批次 02 · 原生宿主与系统托盘 · 部分完成

对应[平台适配与生命周期](architecture.md#6-平台适配与生命周期)。

| 编号 | 状态 | 任务与验收 |
| --- | --- | --- |
| 02.01 | 已完成 | macOS 全窗使用 AppKit `NSVisualEffectView` 背景材质，标准与增强变体共享实现。已在 macOS 窗口实测玻璃效果，代码已接入实色回退。见 [窗口材质](platforms/macos/window_material.rs)和[框架布局](ui/layout/Frame.tsx)。 |
| 02.02 | 待办 | 实现共享托盘菜单模型与生命周期。显示或隐藏窗口、进入设置、检查更新和退出走统一语义动作。覆盖本地化更新、关闭到托盘、托盘失效后的窗口恢复与统一退出。 |
| 02.03 | 待办 | 接入桌面三端托盘。macOS 验收左键恢复、右键或 Control 单击打开原生菜单及 Dock 唤起；Windows 验收 Explorer 重启后恢复；Linux 验收 KSNI/D-Bus 和无托盘降级。 |
| 02.04 | 待办 | 验收菜单取消和隐藏状态、重复退出、清理原生句柄、HMR 后托盘保持单实例。检查更新入口的真实结果随批次 06 验收。 |
| 02.05 | 待办 | 验证 Windows MSIX 包身份。验证 Android 和 iOS 原生返回、前后台恢复及应用内菜单和商店入口。 |

## 批次 03 · 状态、设置、主题与国际化 · 部分完成

对应[状态与设置桥接](architecture.md#2-状态与设置桥接)、[主题与背景资源](architecture.md#4-主题与背景资源)、[国际化](architecture.md#5-国际化)和[本地设置](architecture.md#本地设置)。

| 编号 | 状态 | 任务与验收 |
| --- | --- | --- |
| 03.01 | 已完成 | 首个 Rust Settings Store 通过类型化命令返回带 revision 的快照，TSX 外观投影由页面共用。主页与设置页切换时共享外观，主页计数保留。见 [设置仓库](src/settings.rs)、[NSH 桥接](src/settings_bridge.rs)和 [TSX 投影](ui/state/appearance.ts)。 |
| 03.02 | 部分完成 | 已按数据目录共享原生设置仓库，当前视图通过读取和命令回执获取快照。待拆分 settings、update、session 域及纯 reducer 与后台效果，加入有序派发、按域订阅、引擎线程回调、迟到响应处理、跨窗口同步与 HMR 清理验收。 |
| 03.03 | 部分完成 | 已用带版本的 TOML 保存外观、语言、侧栏和状态栏偏好，支持原子替换、旧值迁移、损坏备份、未来 schema 保护和失败重试。待加入合并写入 Worker、退出与移动后台限时刷盘，以及其他平台路径、托盘、更新和日志偏好。 |
| 03.04 | 已完成 | 支持 Light、Dark、System、独立强调色和样式主题。所有主题使用同一种 JSON，由 Rust 注册表合并 `extends` 链并替换 `var()` 引用，界面只按 token 名读取，不含按主题 ID 的分支。内置主题在构建时从 [主题目录](ui/theme/)发现，包含 Default、Apple 玻璃、NSH Gallery、Primer、Geist、Slate 和 Radix Sand。见 [主题注册表](src/style_themes.rs)、[token 投影](ui/theme/index.ts)和[样式用例](tests/ui/style-themes.test.tsx)。 |
| 03.05 | 已完成 | 支持本地 JSON 主题发现、继承任一已注册主题、扫描诊断（含基础主题缺失、成环和引用无效）、编辑或复制和选择恢复。IBM Carbon 已作为开发版外部文件在 macOS 验证浅色与深色。当前主题定义保存在 `themes/` 的 JSON，TOML 保存选择并兼容迁移旧内嵌主题。两者分别写入，由设置 Store 管理提交顺序和失败重试。见[主题加载器](src/style_themes.rs)和[本地主题说明](README.zh.md#样式主题)。 |
| 03.06 | 部分完成 | TSX 已支持 `zh-Hans`、`en`、系统语言匹配与回退，切换后页面即时更新。待补 TSX/Rust 共用的参数契约、原生菜单和托盘文案、独立格式地区、日期时间与复数规则，以及长文案和系统字号验收。见[文案目录](i18n/messages/)和[翻译入口](ui/i18n/index.ts)。 |
| 03.07 | 部分完成 | 接入 NSH 官方 Gallery UI 作为可选主题 `builtin.nsh-gallery`：快照固定到上游提交，映射见 [sources/nsh-gallery.json](ui/theme/sources/nsh-gallery.json)，用 [同步脚本](scripts/sync-gallery-theme.py)更新，署名见 [NOTICE](NOTICE)。已带入颜色、圆角、阴影和控件度量（按钮 28、输入框 32、聚焦光晕 3、按压 97%、悬停叠色 5%），每项度量记录上游来源。主题不再只换颜色：度量由 `cssVars.shared` 提供，`ui/theme/look.ts` 统一读取，视图不含写死的尺寸。Slate、Primer、Radix Sand 按各自官方源码给出尺寸，Geist 未公开尺寸故沿用根主题。Rust 单测覆盖度量校验、共享 token、强调色重算和各体系尺寸差异，NSH 用例 `control-metrics` 在布局层断言同一按钮在不同主题下的高度。后续已补：按钮、输入框、卡片的表面质感（`finish-*` token：填充、渐变、描边、多层阴影）和触屏尺寸表（`cssVars.touch`，按 `platform.desktop` 选用），聚焦环改为由注册表推导的 `focus-ring`。2026-11-12 已在 macOS 真机对照官网 Gallery 的按钮和输入框，质感接近；Gallery 的 `finishes` 渐变起止同色，视觉差异来自内高光阴影和描边。未做：Gallery 的组件结构（标签条、分段控件、分体按钮、菜单、面板）。聚焦环在屏幕上的实际对比度未达 3:1，见 [07.05](#批次-07--测试体系与性能基线--部分完成)。 |

## 批次 04 · 导航、窗口 Shell 与 AppKit 控件 · 部分完成

对应[窗口与导航](architecture.md#3-窗口与导航)和[平台适配与生命周期](architecture.md#6-平台适配与生命周期)。

| 编号 | 状态 | 任务与验收 |
| --- | --- | --- |
| 04.01 | 已完成 | 桌面 Shell 首期实现自绘标题栏、侧栏停靠与抽屉、主页与设置类型化路由、前进后退和快捷键、可点击面包屑及窄窗折叠。窗口框架后来迁入 `ui/layout/`，见 04.07。见[路由](ui/navigation/index.ts)、[框架布局](ui/layout/Frame.tsx)和 [Shell 用例](tests/ui/window-shell.test.tsx)。 |
| 04.02 | 已完成 | 底部状态栏显示当前应用版本与设置保存状态，开关即时生效并落盘，隐藏后释放布局空间。见[底部状态栏](ui/layout/components/StatusBar.tsx)和 [Shell 用例](tests/ui/window-shell.test.tsx)。 |
| 04.03 | 已完成 | macOS 开发宿主默认编入 AppKit 设置控件，支持的系统可在同一窗口切换控件族和样式，两套控件复用 Rust 设置。标准包及不可用环境回退到 NSH 控件。见 [AppKit 控件](platforms/macos/controls.rs)和[切换用例](tests/ui/appkit.test.tsx)。 |
| 04.04 | 待办 | 补路由参数、可用条件、动态标题、更新页面和原生动作接入。验收移动导航栈、系统返回优先关闭浮层、安全区和系统字号。 |
| 04.05 | 待办 | 补 AppKit 切换时的草稿保留、焦点恢复、浮层释放与失败回退验收。更新任务的状态栏投影随批次 06 接入。 |
| 04.06 | 部分完成 | 设置页改为分类导航：宽窗口左侧是分类列表、右侧是当前分类内容，窄窗口（页面宽度低于 560pt）列表换成一个下拉框，通用、主题、诊断三个分类。分类选择保存在外观 Store 中，离开设置页再回来保持不变，窗口缩放时分类不丢。NSH 控件族使用框架 `Tab` / `Tabs` 和 `Select`，选中态由框架指示层配合主题底色、边框和阴影绘制；液态玻璃（AppKit）版本宽屏用新增的原生组件 `usher.settings.sidebar`（`NSButton` 工具栏样式的纵向列表），窄屏用已有的 `NSPopUpButton`。各分类内容改为分组卡片（小标题加圆角卡片，行内左侧名称与说明、右侧控件，行间分隔线），行内控件沿用各控件族自己的组件。见 [设置页](ui/features/settings/SettingsPage.tsx)、[分类导航](ui/features/settings/SettingsNavigation.tsx)和 [布局用例](tests/ui/settings-layout.test.tsx)；用例在无头宿主与 `--target macos` 都通过。待验收：分组卡片布局在真实宿主中的观感（含 AppKit 控件族与窄窗口折行）、真机上原生列表的外观与选中态、键盘导航与焦点顺序、系统字号放大后的折行。 |
| 04.07 | 部分完成 | 新建 `ui/layout/` 管理框架层布局：通栏标题栏（菜单开关、前进后退、面包屑、右端浅色/深色切换，设置入口在窄菜单底部和二级菜单）、始终显示的窄菜单（底部为更新、设置、用户头像按钮）、可显示或隐藏的二级菜单、内容区（二级菜单隐藏时带动画向左延伸）和与窗口等宽的底部状态栏。整条标题栏可拖动（拖动行为需在真机确认），工具栏图标和按钮尺寸取主题 `size-toolbar-icon`、`size-toolbar-button`，图标与菜单文字使用正文色而非弱化色。二级菜单是带圆角的独立面板，底色和选中色来自主题 `menu-surface`、`menu-selected`（取值按参考图实测）。尺寸与动画时长来自主题 `layout-*`、`motion-layout` 度量，Rust 注册表新增 `layout-` 族校验。更新与头像按钮只定义了接口（[updates.ts](ui/layout/contracts/updates.ts)、[account.ts](ui/layout/contracts/account.ts)），功能接入前保持禁用。见[框架布局](ui/layout/Frame.tsx)和[布局用例](tests/ui/layout.test.tsx)；`nsh check`、`cargo test --locked --workspace` 和全部界面用例在无头宿主通过，布局、窗口 Shell、设置页和 AppKit 用例也在 `--target macos` 通过，并看过 macOS 宿主截图。待验收：展开与收起二级菜单时的动画观感、全屏下的标题栏、窄窗抽屉、AppKit 控件族下的窄菜单、更新与头像功能接入。 |
| 04.08 | 部分完成 | 通用设置新增用户背景图片选择、移除与 0–100% 不透明度，默认 30%，使用框架 `Picker.pickImages` 和 `Slider`。`ui/layout/components/WindowBackdrop.tsx` 统一装配毛玻璃、主题叠色与图片层，图片位于所有交互区域之下，铺满窗口且不拦截点击。Rust Store 保存元数据与透明度，`src/background.rs` 将图片复制到本地，设置提交成功后清理旧资源；保存失败重试复用候选 ID，无数据目录时保留会话内图片。尺寸与导入限制由 `ui/config/background.json` 共享。已通过 `nsh check`、格式检查、Clippy、Rust 工作区 114 项测试、[背景设置用例](tests/ui/background.test.tsx)（3 项：用户选择、取消、实时透明度预览、跨页面保持、非法图片、读取失败回退与移除）、[背景布局用例](tests/ui/background-layout.test.tsx)在无头与 macOS 宿主运行，并查看 macOS 30%/100% 图片层截图；相关外观、布局、设置分类和主题度量回归通过。待验收：系统选择器人工选图、应用重启后的原生显示、Windows/Linux/Android/iOS 的真实宿主行为。 |
| 04.09 | 部分完成 | 通用设置新增内容区域背景透明度，默认 0%，100% 时移除次级菜单栏（含抽屉）、右侧内容面板和设置分组卡片的底色，露出窗口背景图片；只降低填充色 alpha，保留文字、控件与阴影。共享配置定义范围与步长，Rust Store 校验并独立持久化，兼容旧设置，拖动实时预览、释放后保存。`nsh check`、格式检查、Clippy、Rust 工作区 115 项测试及背景、背景布局、设置分类共 10 项无头用例通过；macOS 背景交互用例通过，已查看浅色 0%/42%/100% 及深色 100% 的真实窗口截图。窄窗抽屉通过无头用例验证。待验收：其他平台真实宿主显示。 |

2026-11-12 标题栏与窄菜单提示：八个图标按钮通过 [ButtonTooltip.tsx](ui/layout/components/ButtonTooltip.tsx) 直接挂载 NSH `Tooltip`，包含禁用的前进后退、更新和账户按钮；面包屑不添加提示。文案复用中英文消息目录，随语言、侧栏和明暗切换动作更新。标题栏优先向下、窄菜单优先向右，框架根浮层负责安全区域内的翻转、平移和层级，公开 `maxWidth` 样式限制宽度并允许长译文换行，样式与边距复用现有主题 token。`nsh check`、Rust 工作区 116 项测试及 [Tooltip 用例](tests/ui/tooltips.test.tsx) 3 项在无头与 macOS 宿主通过，布局 5 项与窗口 Shell 2 项无头回归通过；已查看 macOS 浅色英文和深色中文截图，覆盖右上角、窄菜单底部及 380 × 500 点窄窗口，未见提示越界或被内容面板遮挡。其他平台真实宿主待验收。

2026-11-12 固定菜单底部图标调整：更新、设置和默认头像统一读取 `size-rail-utility-icon`（18 点），顶部主页保留 `size-rail-icon`（24 点）。全部窄菜单图标通过现有 AppKit 适配器绘制系统 SF Symbols，配置集中在 `ui/config/navigation-icons.json`；默认头像使用线框 `person`，设置继续按导航状态切换线框／实心。`nsh check` 和布局 5 项用例通过，已查看 macOS 浅色主页、浅色与深色设置页截图。

2026-11-12 导航图标与 Select 反馈调整：主页和设置从 `ui/config/navigation-icons.json` 读取成对 SF Symbols，选中使用 `house.fill` / `gearshape.fill`，未选中使用对应线框，窄菜单、次级菜单与标题栏设置按钮共享。设置分类下拉框按 [NSH Select 官方接口](https://nsh.dev/elements/select/)使用 `itemClass` 的 `focus:` 状态，鼠标与键盘高亮由框架管理；`selectLook` 从当前主题读取 `item-hover`，兼容带空格的 CSS 颜色函数。`nsh check`、布局 5 项与设置分类 6 项无头用例通过；macOS 实窗检查覆盖主页／设置与前进后退、NSH Gallery 明暗模式的鼠标移项和方向键高亮、回车选择，已查看截图。其他宿主沿用字符图标回退，原生视觉效果待验收。

2026-11-11 窄菜单与状态栏调整：窄菜单图标使用独立主题度量 `size-rail-icon`，默认 24 点；浅色菜单选中底色按用户参考图取样为 `#eaeaeb`，集中在 `menu-selected`。状态栏通过框架 `content-center` 居中文本行，配合 `items-center` 保持行内对齐。`nsh check`、Rust 工作区 115 项、布局 5 项和窗口 Shell 2 项无头用例通过。已查看 macOS 主页与设置选中态、明暗模式及中英文状态栏截图；其他平台实际观感待验收。

2026-11-11 设置卡片与分类导航调整：通用、主题和诊断分组卡片通过 `groupLook` 复用内容背景透明度，只调整底色 alpha。分类导航按 [NSH 官方 Tabs 文档](https://nsh.dev/elements/tabs/)改用 `Tab` / `Tabs`，选中底色、边框和阴影交给单一原生指示层，键盘焦点通过 `focusVisibleStyle` 呈现；框架内部代码未修改。新增设置页键盘用例，覆盖方向键、Home/End、循环选中及 Tab 一步进出列表。`nsh check`、背景 4 项、设置分类 6 项和背景布局 1 项无头用例通过，设置分类 6 项 macOS 用例通过；已查看 macOS 明暗分类切换和卡片 0%/部分/100% 透明度截图，连续切换后未见旧项黑框。其他平台真实宿主待验收。

2026-11-11 布局目录整理：`ui/layout/` 根目录保留 `Frame.tsx` 装配入口，视图、控制逻辑和接口分别归入 `components/`、`controllers/`、`contracts/`，共迁移 11 个文件并同步导入与文档。`nsh check` 及无头布局、窗口 Shell、背景布局共 8 项用例通过。

2026-11-11 外观调整：Default 与 NSH Gallery 的浅色毛玻璃窗口使用中性浅灰叠色，内容面板为白色；NSH Gallery 的 `panel` 映射到 `card`，窗口叠色与 Shell 阴影继承根主题。次级菜单使用主题 `shell-shadow`，停靠时允许边缘阴影溢出，收起时裁剪。macOS 布局与设置分类用例各 5 项通过，并查看浅色、深色与图片叠加截图。

2026-11-11 窗口尺寸调整：macOS 默认启动外框及最小外框均为 800 × 600 点，由[原生窗口适配](platforms/macos/window_geometry.rs)按窗口实例设置，TSX 重挂载不重置尺寸。已通过 `nsh check`、`cargo test --locked --workspace`（71 项）及 macOS 宿主 Shell 用例（2 项）；拖动窗口边缘的最小尺寸限制和 HMR 尺寸保留尚未手工验收。

## 批次 05 · 诊断日志 · 部分完成

对应[诊断日志](architecture.md#8-诊断日志)。实现见 [诊断模块](src/diagnostics.rs)、[设置页](ui/features/settings/DiagnosticsSettings.tsx)和 [NSH 桥接](src/settings_bridge.rs)。`cargo check`、`cargo test --lib`（35 项）和 `nsh check` 已在本机通过。设置页用例 [diagnostics.test.tsx](tests/ui/diagnostics.test.tsx) 在无头宿主通过（级别切换、无数据目录时的导出报错、中文文案）；应用内的实际界面尚未看过。

| 编号 | 状态 | 任务与验收 |
| --- | --- | --- |
| 05.01 | 部分完成 | 已实现 `ERROR` 至 `TRACE` 及 `OFF` 的过滤器，等级写入 `[diagnostics]` 并随设置恢复，修改后即时作用于运行中的写入器。TSX 只能上报短标识的模块名和事件名，不能带自由文本。单测已验证等级默认 `INFO`、写入后重启恢复，默认值不写入文件。设置页用例已通过。已在真实 macOS 宿主验证：启动写入 `diagnostics_started`，重启后恢复保存的等级，切换等级写入变更记录。随后发现切到 `OFF` 时该记录被过滤器吃掉，已改为变更记录不受过滤器限制并带 `from`/`to`，这一版只有单测验证，尚未在真实宿主重看。待验收：改后的变更记录在真实宿主的内容、文件权限为 `0600`、HMR 后写入器唯一。 |
| 05.02 | 部分完成 | 已实现有界队列、后台单线程写入、按事件本地日期分桶、8 MiB 分片、14 天保留与磁盘配额，目录不可写时记录健康状态而不影响应用。警告和错误在队列满时等待入队，低级别日志丢弃并计数。保留清理在每天首次写入时运行，不只在启动时。单测已通过，覆盖跨午夜、等级切换、分片、清理、不可写目录和队列满。退出时通过 `atexit` 冲刷队列，单测验证排队的记录在冲刷后已落盘。待验收：磁盘满、真实应用 Quit 后日志尾部完整。 |
| 05.03 | 部分完成 | 已实现按日期范围导出 `.tar.gz`：先写同目录临时文件再改名发布，取消或失败不留半成品；跳过符号链接；单文件 8 MiB、总计 64 MiB 上限，超限时优先保留最新日志；导出目录只留最近 5 个归档。导出逻辑的单测已通过（日期范围、符号链接、截断、取消与失败清理）。目前导出到应用数据目录下的 `exports/` 并打开该目录，未接系统保存对话框；归档数量上限（保留最近 5 个，刚写入的永远保留，无关文件不动）和归档命名唯一有序已有单测。待验收：真实应用点导出后文件夹打开且归档内容正确、桌面保存对话框、Android 与 iOS 系统导出入口。 |

**与同级 Usher 的有意差异**：等级可运行时修改，Usher 的 `fern` 只能重启生效；单一写入线程取代多个 sink 和两套互相转发的日志库；崩溃记录只写一条脱敏的 `ERROR`，不替换宿主的 panic 行为也不主动请求关闭；日志文件权限为 `0600`；预算用尽时丢弃最旧而不是跳过其后的文件。

## 批次 06 · 检查更新、渠道升级与版本说明 · 部分完成

对应[更新子系统](architecture.md#7-更新子系统)和[发布信任边界](architecture.md#发布信任边界)。更新逻辑独立成内部 crate [usher-update](crates/usher-update/)，没有 NSH 依赖，改动后只重编这个 crate。

| 编号 | 状态 | 任务与验收 |
| --- | --- | --- |
| 06.01 | 部分完成 | 已有 `nsh.json` 版本和构建号、状态栏版本显示及 [CHANGELOG 文件](CHANGELOG.md)。待统一运行构建身份与关于弹窗、包元数据及 Release 附件，补版本说明生成、升级公告、首次升级提示和版本一致性门禁。 |
| 06.02 | 部分完成 | 更新状态机已实现并有单测（[state.rs](crates/usher-update/src/state.rs)）：单执行者、重复请求并入进行中的检查、迟到回执按票据丢弃、稳定版不收预发布也不降级、通道切换丢弃旧回执、候选锁定、下载验证安装按序推进、取消后可重试、按能力区分应用内更新、手动升级、系统管理和不适用、后台失败不打扰。待接入：桥接命令、设置页的检查按钮与状态、应用菜单和托盘入口、迟到回执在真实网络下的验证。 |
| 06.03 | 部分完成 | 发布签名清单已实现并有单测（[signature.rs](crates/usher-update/src/signature.rs)）：可信公钥集与密钥 ID、备用密钥签发、清单携带密钥吊销信息、不能吊销全部密钥、通道与版本绑定、`min-version`、精确资产匹配、流式哈希、畸形输入全部拒绝。另有发布侧命令行 [usher-update-tool](crates/usher-update-tool/)：`keygen`、`sign`、`verify`、`pubkeys`，22 个单测覆盖私钥权限与不覆盖、拒绝写入仓库、签名后自验、篡改与未知密钥、通道与吊销、符号链接、CI 环境变量取密钥，并用三处故意改坏的变异确认这些测试真的会失败；真实二进制的完整流程也跑过。已创建首把正式发布密钥（密钥 ID `c319436980d8b1c4`，公钥 SHA-256 指纹 `c319436980d8b1c40fd2f01ff3b2af0150d7e44c32936419f04981e33fb5fb01`），公钥已写入 `trusted_keys.txt`；私钥只在本机 `~/.usher-nsh-update-signing/`（0600），不在仓库中，尚无离线备份。另有一把恢复密钥（密钥 ID `05c3fc64cf159880`，公钥 SHA-256 指纹 `05c3fc64cf159880f348713c7ee518693ae71bdc1549b999072b0352461f943b`，在 `trusted_keys.txt` 中带 `recovery:` 前缀），私钥目前在本机 `~/.usher-nsh-update-backup/`，**需要尽快移到离线存储并从本机删除**，否则它不构成真正的备用。密钥分发布与恢复两种角色：只有恢复密钥签发的清单才能吊销密钥，发布密钥签发的带吊销的清单被拒绝，防止偷到发布密钥的人吊销恢复密钥；恢复密钥不能吊销自己，保证始终有一把密钥可信。这些规则有单测，并用真实的两把密钥演练了四种情形（发布密钥正常签发、发布密钥试图吊销恢复密钥被拒、恢复密钥吊销泄露的发布密钥、恢复密钥吊销自己被拒）。内置列表为空会让构建的单测失败，避免发布一个什么都不信任的更新器。泄露处置流程见[发布密钥泄露处置](docs/deploy.md#发布密钥泄露处置)。待办：吊销集合的持久化与应用，以及安装交接前用最新吊销集合重新验证已下载的包。待办：CI 工作流里调用 `sign`、从 GitHub Release 取清单的网络层、按构建变体选包、Windows MSIX、Linux Flatpak、Android Play、iOS App Store 适配。 |
| 06.04 | 部分完成 | 已用 NSH 自带工具链实测签名包：`nsh keygen`、`certify`、`publish`、`verify`、`revoke` 都可用，真实应用打出的 `.nshpkg` 约 262 KB。验证了根密钥正确时通过、换根拒绝、改动一个字节拒绝、发布密钥被吊销后拒绝、轮换到新发布密钥后的新构建仍通过。**未通过**：生成的发布运行器 `main.rs` 直接调用 `nsh_macos_app::main()`，只从应用包内 `Resources/nsh/bundle.nshc` 读取内置字节码，没有读取 `.nshpkg` 的入口，`load_package` 只在引擎库里；我们的 Rust 模块也拿不到运行中的 `AppView`，所以应用目前无法自己加载新的签名包。可行的出口只有 `NSH_CHUNK_DIR` 指向的 `import()` 模块目录，它只读源码，且不经过 `TrustStore`，签名要由我们自己的代码先验证再落盘。该层在拿到可行方案前保持待办，整包层不受影响。macOS 整包层待接 Sparkle 2 与签名 appcast。 |
| 06.05 | 待办 | 用各正式渠道的签名旧版实际升到新版，验证设置保留、版本一致性、防降级及失败恢复。若自管下载，覆盖离线、中断、签名错误、包架构不匹配、重复触发和中断恢复。 |
| 06.06 | 部分完成 | GitHub 发布流水线已写好并在本机逐段验证：[ci.yml](.github/workflows/ci.yml)（格式、Clippy、Rust 测试、更新 crate 独立测试、`nsh check` 与界面测试、发布脚本测试、workflow 与 shell 脚本 lint）和 [release.yml](.github/workflows/release.yml)（门禁、复测、打包 `standard` 与 `appkit` 变体、核对包内版本、签名清单、构建来源证明、先草稿后发布）。门禁脚本 [release_gate.py](scripts/release_gate.py) 有 28 个单测并做过变异验证。`actionlint` 与 `shellcheck` 通过，NSH 命令行的安装脚本用固定哈希校验并测过错误哈希、格式错误和缺变量三种失败，`nsh package` 与归档步骤在本机跑通且解压后签名仍有效。**没有 Apple 开发者证书**：Apple 签名、公证、钥匙串步骤已作为占位写好，开关 `MACOS_SIGNING_NOTARIZATION_ENABLED` 默认关闭，未签名发布在日志、摘要和发布说明里都有明确警示；钥匙串安装、`codesign` 与清理用自签名证书演练过，公证与 `stapler` 未演练。`release.yml` 只有两种启动方式：推送新的 `v*` 标签是发布，手动运行是演练，没有分支或定时触发；门禁脚本拒绝其他一切事件，有单测。演练走同一条构建链路（门禁、复测、两个变体的打包与版本核对、收集文件），再由没有任何 Secret、没有写权限、不绑定 `release` 环境的 `rehearse` 作业用临时密钥签名，并按客户端规则验证、故意弄坏三份拷贝确认被拒；本机把这条链路完整跑过一遍，并确认验证器被替换成“全部通过”的假工具时演练会失败。演练里只有会让构建失去意义的问题（版本不一致、构建号无效）会失败，更新日志缺失、构建号不够高改为警告并在摘要里标明“标签会被拒绝”。`publish` 只在发布时运行。`ci.yml` 只检查不发布。生产构建只产出 macOS Apple Silicon 的 `standard`、`appkit` 两个变体，更新包由 `standard` 复制；NSH 的 macOS 运行时只有 arm64 且 `nsh package` 没有架构选项，所以无法构建 Intel Mac 版本。**从未在 GitHub 上真实运行过**，演练也证明不了真实更新密钥是否配置正确、发布步骤本身和 Apple 签名。待验收：首次推送后手动跑一次演练、建 `release` 环境与 `UPDATE_SIGNING_KEY`、Windows 与 Linux。详见 [部署与发布](docs/deploy.md)。 |

## 批次 07 · 测试体系与性能基线 · 部分完成

验证要求见[开发与贡献规范](CONTRIBUTING.md#验证与交付)。测试随对应功能交付，本批负责补齐跨层覆盖、平台验收和性能作业。

| 编号 | 状态 | 任务与验收 |
| --- | --- | --- |
| 07.01 | 部分完成 | 已有 Rust 设置、迁移和主题单测，以及外观、主题、导航、AppKit 的 [NSH UI 用例](tests/ui/)。新增领域随实现补 reducer、更新策略、日志过滤和按日分桶单测。 |
| 07.02 | 待办 | 补独立 Rust 集成测试和离线夹具，覆盖动作到快照及落盘、跨宿主桥接、更新和导出。建立合并检查、五端原生烟测及签名产物验收的 CI 矩阵。 |
| 07.03 | 待办 | 在 `benches/` 建立微基准，测量派发、快照投影、列表转换和日志格式化，固定输入并保存结果。 |
| 07.04 | 待办 | 在 `tests/performance/` 建立发布宿主性能作业，测冷启动、交互、帧时间、CPU、峰值内存、托盘长驻与恢复、移动前后台及资源释放。在同规格 runner 上保存原始样本、P50/P95 和平台基线，再定义回归门禁。 |
| 07.05 | 待办 | 聚焦环在屏幕上的实际对比度。注册表按主题自己的前景色与背景色推导 `focus-ring`（[derive_focus_ring](src/style_themes.rs)），Rust 单测 `every_bundled_focus_ring_reaches_three_to_one_on_every_surface` 保证该 token 对 background、card、panel 三种表面不低于 3:1，推导时另外参考 sidebar 与 menu-surface。这只保证 token，不保证屏幕。2026-11-12 在 macOS 真机（NSH 0.5.0）对 Default、NSH Gallery、Primer、Slate 的浅色和深色做 Tab 前后截图差分，取变化像素作为环，实测对比度：按 Default、Gallery、Primer、Slate 的顺序，浅色 3.86、3.85、2.70、4.23，深色 2.76、2.61、2.13、2.83。深色四项和 Primer 浅色低于 3:1；Apple Glass、Geist、Radix Sand 未截图。已知现象：屏幕上的环比 token 暗，三通道按近似同一比例缩小：Default 浅色 0.92，Default 深色 0.90，Primer 深色约 0.90，Gallery 深色约 0.86，各样本不一致，不是固定倍数。例：Default 深色 token `#707070`，屏幕 `#656565`。变暗在浅色底上会抬高对比度，在深色底上会压低对比度，所以浅色大多达标而深色全部不达标。来源未查明，可能是颜色空间转换，也可能是 outline 的混合方式。不确定之处：对比度的参照色取的是 Tab 前同一位置的像素，outline 落在控件边缘时它可能是控件自己的边色而不是页面表面（Primer 浅色取到 `#ccd3db`，深色取到 `#2b2f35`，都不在已检查的表面里），所以 Primer 两项的数值可能偏低，需要用页面和卡片的真实颜色复核。另有一处观察：各控件聚焦偏移不统一，列表项使用负偏移，按钮不是。待办：1. 用已知 `outlineColor` 的纯色方块在浅色、深色底上截图，建立设定色到屏幕色的映射，确认压暗的原因。2. 按映射反推推导公式，并把验收改为对显示色断言：截图差分用例固定在 `tests/ui/`，覆盖七个内置主题的浅色和深色，参照色取页面、卡片、面板的真实像素。3. 统一聚焦偏移。4. 在 Windows 和 Linux 宿主复核同一映射，不假设与 macOS 相同。 |
