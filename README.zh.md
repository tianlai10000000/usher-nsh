# Usher NSH

[English](README.md) · [架构文档](architecture.md) · [开发与贡献规范](CONTRIBUTING.md)

Usher NSH 是基于 Longbridge Native Shell（NSH）、Rust 和 TSX 构建的桌面与移动应用基础工程。Rust 管理共享应用数据和系统服务，TSX 负责视图与表现状态。主页目前保留 Hello World 示例。

## 当前能力

- Rust 共享设置、本地 TOML 持久化、迁移、恢复和保存状态反馈。
- Light、Dark、System 明暗模式，可配置强调色、内置色板和本地 JSON 主题。
- 英文、简体中文与跟随系统语言。
- 自绘桌面标题栏、可折叠侧栏、导航历史、面包屑和可隐藏底部状态栏。
- macOS AppKit 窗口材质，以及受支持的增强构建中可切换的原生设置控件族。
- 结构化诊断日志、可配置等级、按日记录、保留清理和归档导出。
- 更新设置、自动检查、代码包下载、整包下载确认、进度和重启操作；共享状态由 Rust 持有，签名校验由 NSH 负责。

macOS arm64 的本地整包替换，以及 standard 0.1.0 升级到公开发布的 0.1.1 已验证；远端代码包验收、系统托盘和其余平台验收按实施清单推进。日志目前导出到应用数据目录的 `exports/` 并打开该文件夹，系统保存对话框与移动端导出接入仍待完成。

## 支持平台

| 目标 | 项目状态 |
| --- | --- |
| macOS | 当前开发平台，已有 AppKit 集成和本地打包验证 |
| Windows、Linux | 计划支持的桌面平台，待完成项目级构建与运行验收 |
| Android、iOS | 计划支持的移动平台，待完成模拟器、真机和打包验收 |

实现进度与验收标准记录在 [迭代批次](docs/todo.md)。

## 环境要求

准备 Rust 1.97 或更高版本（edition 2024）、Xcode 命令行工具和 NSH CLI v0.5.0。项目中的 Rust NSH 依赖固定为同一版本。NSH 发布文件可从上游项目获取。[NSH v0.5.0](https://github.com/longbridge/nsh/releases/tag/v0.5.0)

## 快速开始

在项目根目录运行。

```sh
nsh --version
nsh run --target macos
```

首次运行会编译 Rust 模块和 macOS 开发宿主，按 `Ctrl+C` 停止开发服务器。开发流程支持 TSX 热更新。Rust 和宿主变更需要重新编译，HMR 状态保留与生命周期验收在实施清单中跟踪。

在 **设置 → 外观** 中分别选择明暗模式、样式主题和控件族。macOS 开发构建默认包含 AppKit 设置控件，原生组件已注册时可在 macOS 26 及以上系统使用。

文件选择与读写、主题扫描与保存、诊断导出、更新等需要等待的操作在按钮内部显示加载图标并保持禁用，加载前后按钮尺寸不变。主题、明暗模式、语言和导航等即时切换不显示加载图标。仅含图标的按钮在加载期间替换原图标；最左侧窄菜单栏保持现有交互。

导航支持前进、后退和面包屑。macOS 桌面快捷键为 `⌘+[` 和 `⌘+]`，其他桌面目标为 `Ctrl+[` 和 `Ctrl+]`。窗口由通栏标题栏（菜单开关、前进后退、面包屑和浅色/深色切换）、固定窄菜单（上方主页，底部依次为更新、设置、用户头像按钮）、可显示或隐藏的二级菜单、内容区和可选的通栏底部状态栏组成。隐藏二级菜单时内容区带动画向左延伸，窄窗改用抽屉显示二级菜单。更新按钮展示可用更新、下载进度和重启就绪状态；不支持更新（包括开发运行）或显式下载期间禁用。头像按钮仍禁用。二级菜单停靠和底部状态栏开关会保存到本地。

标题栏和窄菜单的八个图标按钮使用 NSH `Tooltip`，支持鼠标悬停和键盘焦点，禁用按钮也可显示提示。提示文案随所选语言和当前动作更新，优先显示在标题栏按钮下方、窄菜单按钮右侧；NSH 在窗口内容之上绘制浮层，按安全区域自动翻转或平移，较长文案按可用宽度换行。面包屑不显示 Tooltip。

设置页的宽屏分类列表使用框架 `Tab` / `Tabs`，紧凑布局使用 `Select`。选中项显示主题底色与边缘，指示层由 NSH 定位和移动；键盘焦点使用独立光环，方向键及 Home/End 切换分类，Tab 一步进入或离开列表。AppKit 控件族使用原生导航控件。

macOS 窗口的默认启动尺寸和最小外框尺寸均为 800 × 600 点。TSX 热更新保留当前窗口尺寸。

## 本地数据

使用默认 macOS 宿主和应用标识时，数据目录如下。

| 构建 | 目录 |
| --- | --- |
| 开发版 | `~/Library/Application Support/com.tianlai10000000.usher.nsh/development/` |
| 正式版 | `~/Library/Application Support/com.tianlai10000000.usher.nsh/` |

每个目录包含 `settings.toml`、本地主题目录 `themes/`、导入背景图片的 `backgrounds/`、诊断日志 `logs/YYYY-MM-DD/usher.log` 和生成归档的 `exports/`。基础目录由宿主提供，隔离测试可以使用单独的数据根目录。

## 样式主题

所有主题使用同一种 JSON 格式，无论随应用发布还是由用户在本地添加。Rust 注册表（[src/style_themes.rs](src/style_themes.rs)）负责加载主题、与 `extends` 指向的基础主题合并、替换 `var(--name)` 引用，再向界面提供完整的 token 表。界面按名称读取 token，新增主题不需要修改代码。

内置主题在构建时从 [ui/theme/builtin/](ui/theme/builtin) 和 [ui/theme/presets/](ui/theme/presets) 发现。在这两个目录添加文件即可新增主题，`order` 字段决定它在列表中的位置。[default](ui/theme/builtin/default/theme.json) 是根主题，定义全部 token，其他主题只需覆盖差异部分。

添加本地主题时，在设置中打开主题目录，添加 `themes/<名称>.json` 或 `themes/<目录>/theme.json`，再点击 **重新扫描主题文件**。设置页也可创建主题或复制已有主题。修改显示名称时保留稳定 ID，以便恢复已保存的选择。

```json
{
  "schemaVersion": 1,
  "id": "user.warm-frost",
  "name": "Warm Frost",
  "names": { "zh-Hans": "暖灰磨砂" },
  "extends": "default",
  "cssVars": {
    "light": { "sidebar": "#EFEDE9B3", "radius": 12 },
    "dark": { "sidebar": "#242322B3", "radius": 12 }
  }
}
```

| 字段 | 含义 |
| --- | --- |
| `id` | 本地文件使用 `user.<名称>` 或 `custom-<名称>`。内置主题使用普通 ID 或 `builtin.<名称>`。 |
| `names` | 可选，按界面语言给出显示名称。语言没有对应项时使用 `name`。 |
| `extends` | 另一个主题的 ID，省略时为 `default`。继承链止于 `default`，最多八层。 |
| `material` | `solid` 或 `glass`，省略时继承基础主题。 |
| `order` | 在主题列表中的位置。没有该字段的本地主题排在内置主题之后。 |
| `source` | 可选，外部来源主题的出处：名称、地址、许可证和版本。 |
| `cssVars.shared` | 与配色无关的 token：尺寸、间距、字号、圆角、聚焦和动效。 |
| `cssVars.light`、`cssVars.dark` | 颜色及其他随配色变化的 token。配色中的值优先于 `shared`。每种配色最多 192 项。 |

token 取值按名称族校验，族之外按形态校验。颜色为 `#RRGGBB` 或 `#RRGGBBAA`、`color-mix(...)`、`rgba(...)` 等 CSS 函数、阴影列表，或 `var(--token)` 引用。度量族取普通数字：

| 族 | 含义 | 范围 |
| --- | --- | --- |
| `size-*` | 高度、宽度和线宽，单位点 | 0 到 64 |
| `layout-*` | 框架宽度、边距、标题栏高度和宽窗断点，单位点 | 0 到 1200 |
| `space-*` | 内边距和间距，单位点 | 0 到 48 |
| `font-*` | 字号，单位点 | 0 到 32 |
| `radius`、`radius-*` | 圆角，单位点 | 0 到 32 |
| `focus-width`、`focus-offset` | 键盘聚焦光晕的宽度和偏移，单位点。偏移可为负，表示画在内侧。 | 0 到 8，偏移 ±8 |
| `motion-*` | 时长，单位毫秒 | 0 到 1000 |
| `feedback-*` | 比例：按压缩放、悬停叠色强度、禁用透明度 | 0 到 1 |

根主题 [default](ui/theme/builtin/default/theme.json) 列出全部度量。控件通过 [ui/theme/look.ts](ui/theme/look.ts) 读取它们，因此主题只需修改数值，就能改变按钮的高度、内边距、圆角、聚焦光晕、按压下沉和悬停叠色。视图中不再有自己的尺寸和颜色。基础主题不存在、引用成环或取值无效的主题会被跳过并在设置中提示，不影响其他主题。

内置主题包括 [Default](ui/theme/builtin/default/theme.json)、[Apple 液态玻璃](ui/theme/builtin/apple-glass/theme.json)、[NSH Gallery](ui/theme/presets/nsh-gallery.json)、[Primer · GitHub](ui/theme/presets/primer.json)、[Geist · Vercel](ui/theme/presets/geist.json)、[Slate · shadcn/ui](ui/theme/presets/slate.json) 和 [Radix Colors · Sand](ui/theme/presets/radix-sand.json)。

Default 和 NSH Gallery 在浅色模式下使用中性浅灰色窗口叠色，覆盖在 macOS 毛玻璃背景之上。NSH Gallery 的内容面板使用卡片底色，浅色模式下为白色。停靠的次级菜单通过 `shell-shadow` 绘制边缘阴影；NSH Gallery 从 Default 继承该阴影与窗口叠色。

窄菜单顶部图标使用 `size-rail-icon`（默认 24 点），底部更新、设置和默认头像图标使用 `size-rail-utility-icon`（默认 18 点），与标题栏图标尺寸独立。浅色模式下的默认菜单选中底色为 `menu-selected: #eaeaeb`。

macOS 上的主页和设置使用系统 SF Symbols，选中时为实心，未选中时为线框；窄菜单和次级菜单保持一致。更新和默认头像同样使用系统图标，所有窄菜单图标集中在 [navigation-icons.json](ui/config/navigation-icons.json)，其他宿主沿用已有字符回退。窄窗口的设置分类下拉框通过 Select 原生高亮状态响应鼠标悬停和键盘导航，底色使用当前主题的 `item-hover`。

在 **设置 → 通用 → 窗口背景** 中，可通过系统图片选择器选择图片、移除图片或调节图片不透明度。初始值为 30%，0% 完全透明，100% 完全不透明。图片铺满窗口，位于毛玻璃背景与所有界面元素之间。拖动滑块时实时预览，释放后保存。图片先复制到本地再保存引用，移动原文件不会影响背景；设置文件仅保存图片元数据与透明度。选择器将图片最长边缩至最多 2560 像素，导入限制为 16 MiB 与 16,777,216 像素。限制与默认值通过 [background.json](ui/config/background.json) 在 Rust 与 TSX 之间共享。图片缺失或无法读取时回退为主题背景，可在设置中重新选择或移除。无数据目录的宿主仅在当前会话保留选择。背景合成统一由 [WindowBackdrop.tsx](ui/layout/components/WindowBackdrop.tsx) 管理。

同一区域中的**内容区域背景透明度**统一控制次级菜单栏（含抽屉）、右侧内容面板及设置页各分类的卡片背景。默认 0%，保留主题底色；100% 时面板底色完全透明，可看到下方的自定义背景图片。文字和控件保持原有外观。拖动时实时预览，释放后保存，与图片不透明度分别存储。

### NSH Gallery 主题

[nsh-gallery.json](ui/theme/presets/nsh-gallery.json) 由 NSH Gallery UI（[examples/nsh-gallery-ui](https://github.com/longbridge/nsh/tree/main/examples/nsh-gallery-ui)，Apache-2.0，见 [NOTICE](NOTICE)）的设计 token 生成。该层是 NSH 仓库的内部部分，因此本主题是固定到某次提交的快照，而不是依赖项。上游 token 到本应用角色的映射位于 [ui/theme/sources/nsh-gallery.json](ui/theme/sources/nsh-gallery.json)。需要跟进上游时，在项目根目录执行：

```sh
python3 -I scripts/sync-gallery-theme.py --revision main
cargo test --locked --lib
```

组件级主题接入与逐项验收见 [NSH Gallery 主题审计](docs/todo.md#evidence-theme-audit-0)。开关、滑杆、进度、选中指示器和浮层分别读取部件表面；自定义强调色替换固定强调色渐变。已接入开关原生弹簧、滑杆悬停、进度扫动和浮层进出动画，支持减少动态效果。Select 的关闭保留当前框架行为，其余边界见审计文档。


脚本只重写主题文件和固定的版本号。提交前请检查差异。该主题提供 Gallery 的颜色、圆角和阴影，以及 Gallery 的控件度量：28 点按钮、32 点输入框、7 点控件圆角、3 点聚焦光晕、97% 按压下沉和 5% 悬停叠色。映射中的每个度量都记录了来源文件和符号。尺寸写在 TypeScript 源码（`look/*.ts`）而不是 token 文件里，因此同步后需要在新版本中核对这些符号。Gallery 自身的组件结构不在此复现：标签条、分体按钮、菜单和面板不属于本应用。

[Slate](ui/theme/presets/slate.json)、[Primer](ui/theme/presets/primer.json) 和 [Radix Colors · Sand](ui/theme/presets/radix-sand.json) 以同样方式给出 shadcn/ui、Primer 和 Radix Themes 的尺寸。[Geist](ui/theme/presets/geist.json) 没有给出，因为 Vercel 未公开组件尺寸，所以它使用根主题的尺寸和自己的颜色。

## 诊断日志

设置页顺序为更新在前、诊断在后。诊断页分为日志输出、详细程度、专项诊断和归档：应用文件日志默认开启，控制台和结构化诊断可独立开关，默认关闭；终端详细日志与内存诊断显示禁用开关并注明“尚未接入”。

所有类型共用 `ERROR`、`WARN`、`INFO`、`DEBUG`、`TRACE` 和 `OFF` 等级，默认 INFO。等级及输出开关即时生效并保存供下次启动恢复，OFF 保留开关偏好。结构化诊断是独立事件流，与 TRACE 等级无关，当前记录诊断配置与更新状态转换。

后台写入器按本地日期记录 JSONL：应用事件写 `usher.log`，结构化事件写 `usher_tracing.log`，各自 8 MiB 分片，共享 14 天 / 256 MiB 保留清理；不会为了配额删除当天目录。凭据脱敏，主目录替换为 `~`。支持导出今天、最近 7 天或最近 14 天的 `.tar.gz`，包括已关闭输出的历史文件；跳过符号链接，单个源文件上限 8 MiB，源数据总上限 64 MiB，导出目录保留最近五个归档。独立门面、后端接口与测试矩阵见[日志设计](docs/log.md)，平台验收缺口见[批次 05](docs/todo.md#logging)。

## macOS 打包

```sh
sh scripts/package-macos.sh standard
sh scripts/package-macos.sh appkit
```

| 变体 | 内容 | 输出目录 |
| --- | --- | --- |
| `standard` | AppKit 窗口材质、共享主题与 NSH 控件 | `.nsh/out/macos-standard/` |
| `appkit` | 标准包能力，以及受支持系统上可选择的 AppKit 设置控件 | `.nsh/out/macos-appkit/` |

省略脚本参数时生成 `standard`。两种变体共用应用标识和正式版设置目录。脚本生成本地 ad hoc 签名产物，正式分发签名与公证仍属于发布阶段工作。

Release 优化参数统一放在 [.cargo/config.toml](.cargo/config.toml)。在项目根目录直接运行 `nsh package --target macos` 也会读取该配置，默认输出到 `.nsh/out/`。其余目标在接入时分别验证配置生效情况。

## 验证

从项目根目录按变更范围运行检查。工作区命令包含更新、状态、路由与测试支持 crate；应用依赖启用 watch 和 matchit 适配器，独立套件另行验证不启用这些 feature 的核心。

```sh
cargo test --locked --workspace
nsh check
nsh test tests/ui/appearance.test.tsx
nsh test tests/ui/style-themes.test.tsx
nsh test tests/ui/background.test.tsx
nsh test tests/ui/background-layout.test.tsx
nsh test tests/ui/window-shell.test.tsx
nsh test tests/ui/tooltips.test.tsx
nsh test tests/ui/diagnostics.test.tsx
```

只修改更新逻辑时可运行。

```sh
cargo test --locked -p usher-update
```

验证通用测试工具、状态与路由核心、应用集成及无头旅程时运行：

```sh
python3 scripts/test.py test-support
python3 scripts/test.py logging
python3 scripts/test.py logging-mock
python3 scripts/test.py state
python3 scripts/test.py router
python3 scripts/test.py routes-check
python3 scripts/test.py update-state
python3 scripts/test.py state-targets
python3 scripts/test.py integration
python3 scripts/test.py state-ui
python3 scripts/test.py router-ui
python3 scripts/test.py e2e --target headless
python3 scripts/test.py ui
```

`test-support`、`logging`、`logging-mock`、`state` 和 `router` 不需要 NSH。日志规则单测就近放置，公共契约通过注入后端捕捉全部类型和等级，并将真实文件适配器与 capture 后端对照。状态和路由以参考及真实后端运行同一消费者契约，路由属性测试使用夹具中的固定种子。`integration` 运行 11 项设置、13 项窗口 / 路由、17 项更新与 3 项诊断测试，使用真实服务和隔离数据。`state-ui` 验证 TS 投影及真实订阅桥接，`router-ui` 验证路由门面及真实页面生命周期。`e2e --target headless` 使用隔离内存会话，运行实际 App 到 Rust 服务的四项旅程。macOS 原生验收命令已验证同 VM 窗口共享持久偏好、窗口会话隔离、真实源码 HMR 及关闭重开身份。

窗口状态由 `WindowService` 原子提交，页面会话规则位于 `src/window_session.rs`。Rust 服务保留类型化错误，桥接输出稳定错误码。生成的路由契约包含字面量元数据，并校验页面与文案类型。旧会话桥接保持兼容。

报告写入 `target/test-reports/`；失败、空测试、跳过或必测组缺失均非零退出。NSH 套件递归发现源文件并验证新生成的 JUnit。同一 checkout 内的 NSH 命令须串行运行，类型检查单独执行 `nsh check`。`routes-check` 验证生成器测试及实际目录生成契约的新鲜度，不修改文件；后者单独计为检查，不额外计为测试。修改 `ui/pages/routes.json` 后执行：

```sh
cargo run --locked -p usher-router --features matcher-matchit --bin usher-routes -- generate ui/pages/routes.json ui/pages/routes.generated.ts
python3 scripts/test.py routes-check
nsh check
```

`state-targets` 编译配置中的七个 Rust 目标，须先安装对应 rust-std；CI 从 tests/config.json 读取目标并安装。它只检查纯库，不启动其他平台宿主。

macOS 原生检查会打开真实窗口，需要解锁的图形会话。状态验收命令复制独立源码，给官方宿主显式指定临时数据 / 缓存根；仅修改副本触发 HMR，检查落盘文件，并在结束后删除临时树：

```sh
python3 scripts/test.py e2e --target macos
```

该命令运行三项生命周期探针、一项原生返回 / 守卫测试、诊断布局 / 控件与保存过程样式测试，以及七项按钮生命周期旅程，使用生产状态 / 路由适配器及 RouterView。按钮套件在隔离宿主中加入仅用于测试的适配器，取消真实 AppKit 图片选择面板，并验证关闭、状态恢复与再次取消；该适配器不进入生产应用。不包含进程重启或所有原生输入路径。报告包括 e2e-macos.json、native-state.xml、native-isolation.json 和逐轮日志。CI 通过手动 native_state 开关在带 usher-native 标签的 self-hosted macOS runner 上运行；需预先配置并保持桌面解锁，远端运行尚未验证。

仅运行隔离的原生按钮旅程：

```sh
python3 scripts/native_button_acceptance.py --junit target/test-reports/native-buttons.xml
```

发布包深链接验收使用 ad-hoc 签名的 release 应用与唯一测试 ID；专属数据 / 缓存命名空间映射到临时目录，仅在隔离源码副本的产品入口加入只读路由观察器。命令验证 LaunchServices 注册、外部 URL 冷启动和同进程唤醒，然后注销测试应用并清理隔离数据：

```sh
python3 scripts/test.py packaged-routing
```

该命令也进入手动 native_state CI 作业，不代表 Developer ID 公证或 Gatekeeper 下载应用链路验收。

macOS VM reload 验收通过真实 Cmd+R 重建 VM，恢复持久偏好、重置窗口历史 / 会话并拒绝旧回调。它使用官方 runner 的隔离数据 / 缓存目录，与 HMR 和整个进程重启分别验收。

```sh
python3 scripts/test.py native-reload
```

本地更新启动模拟为每轮生成唯一测试应用 ID，每个场景使用独立临时数据 / 缓存根，不改 HOME。12 个场景覆盖检查、签名代码包下载与下次启动晋升、断网 / 中断下载、信任失败及启动回滚。需在解锁的 Apple Silicon macOS 宿主运行：

```sh
scripts/update-sim/sim.sh run --all
```

日志和清理证据保留在 `target/test-reports/update-sim-*/`。`serve <场景>` 由脚本管理隔离应用，Ctrl+C 后停止并清理。该命令验证生产控制器检查、签名代码包下载和下次启动晋升，以及预置包的恢复链路；真实整包替换由下方独立的 shell-update 命令验收，见[本地更新模拟方案](docs/update.md#12-本地更新模拟d14)。

真实整包归档验收构建新旧标准变体与旧版 AppKit 应用，通过生产控制器向签名本地源执行检查、下载和安装，核对文件替换、NSH 自动重启、设置保留，以及归档损坏、内部身份 / 签名拒绝和安装权限恢复：

```sh
python3 scripts/test.py shell-update
```

在解锁的 Apple Silicon Mac（macOS 26 或以上，AppKit 场景要求）串行执行。每轮使用唯一应用 ID、临时源码 / 应用 / 数据目录和一次性密钥。报告保留在 `target/test-reports/shell-update-*/`；清理失败时保留隔离树并使测试失败。

命令已加入手动 native_state CI 作业，另覆盖 AppKit 转标准版、`/Applications` 下本轮专属临时目录升级、检查前目录不可写，以及替换中断、架构标签不符和人工 quarantine 标记的本地边界探测。边界探测在各场景报告里单独记录已知缺口；探测通过不等于自动崩溃恢复或 Gatekeeper 放行。Developer ID 公证、真实浏览器分发和正式发布不在命令范围内。已确认 NSH 0.5.0 在安装重命名间隙中断后需要外部恢复，且不会拒绝签名有效但架构不符的归档；人工隔离标记的 ad hoc 包被 Gatekeeper 拒绝。剩余验收见更新方案。

原有原生 UI 检查：

```sh
nsh test tests/ui/window-shell.test.tsx --target macos
nsh test tests/ui/tooltips.test.tsx --target macos
nsh test tests/ui/background-layout.test.tsx --target macos
nsh test tests/ui/appkit.test.tsx --target macos
USHER_MACOS_APPKIT=0 nsh test tests/ui/appkit.test.tsx --target macos
```

无头 UI 检查使用会话设置，磁盘持久化与原生材质需要在真实宿主验证。集成测试、benchmark 和性能测试的后续工作见实施清单。

## 仓库导航

| 位置 | 职责 |
| --- | --- |
| `App.tsx` | 应用挂载与共享视图装配 |
| `src/` | Rust 设置、主题文件、诊断日志和 NSH 桥接 |
| `crates/usher-update/` | 代码包 / 整包更新纯策略 |
| `crates/usher-logging/` | 独立日志、脱敏、保留清理与导出 |
| `crates/usher-state/`、`crates/usher-router/` | 可替换状态后端与独立路由规则 |
| `crates/usher-test-support/` | 仅开发依赖使用的通用测试工具 |
| `tests/integration/`、`tests/e2e/` | 应用契约与真实无头旅程 |
| `ui/layout/Frame.tsx` | 窗口框架装配入口 |
| `ui/layout/components/` | 标题栏、菜单、内容区、状态栏与背景层 |
| `ui/layout/controllers/`、`ui/layout/contracts/` | 菜单与窗口控制逻辑，以及布局消费的接口 |
| `ui/router/`、`ui/pages/` | 路由门面、RouterView、路由目录与页面注册 |
| `ui/features/`、`ui/state/`、`ui/components/` | 功能视图、表现状态和复用控件 |
| `ui/theme/` | 内置主题文件、Gallery 映射与 token 投影 |
| `i18n/messages/`、`ui/i18n/` | 翻译目录与界面国际化 |
| `platforms/macos/` | AppKit 窗口材质、控件与图标 |
| `tests/ui/` | NSH 界面检查 |
| `scripts/` | 打包与 Gallery 主题同步命令 |
| `.nsh/`、`target/` | 生成的构建与测试产物 |

[项目架构](architecture.md)描述模块职责、依赖边界、状态归属与数据流。[分批实施清单](docs/todo.md)记录实现状态，[开发与贡献规范](CONTRIBUTING.md)定义开发与贡献要求，[交互设计规范](docs/interaction-design.md)定义布局、按钮加载、主题与文案等界面要求。

[状态管理方案](docs/state-management.md)记录 SM1 至 SM3 及 SM4 迁移。设置保持原桥接和持久化格式兼容；导航、分类、操作反馈、共享预览、导出和计数已使用 Rust Store，诊断健康使用独立实时订阅。无头回归和隔离目录下的 macOS 多窗口 / HMR 验收均已通过。SM4 在当前支持的 macOS 开发宿主范围完成；其他平台及发布验收分别跟踪。

[路由方案](docs/routing.md)记录已实施的 RT1 至 RT3 与 RT4 当前进展：独立 `usher-router`、可替换 matchit 匹配、从 `ui/pages/routes.json` 生成类型、集中页面注册及 `ui/router` / RouterView。历史经 `usher-state` 提交，原有可写 `ui/navigation` 已移除。RT4 已接入 `usher://app/…` 深链接、经过异步守卫的原生返回，以及保留历史和会话的目录替换 / HMR。改动 routes.json 内容须更新 identity 并重新生成类型。macOS 开发宿主以及本机 arm64 ad-hoc 发布包的系统 URL 注册、外部冷启动与同进程唤醒已验证；其他平台宿主及分发/公证仍待验收，iOS 交互式滑动返回尚未实现。

[测试方案](docs/testing.md)记录 GitHub 实践依据、夹具和实际结果。UT1 已完成，应用集成、桥接及无头 E2E 已交付。通用 Rust 工具通过开发依赖复用 `usher-test-support`，业务夹具仍归所属模块。单测和 crate 契约就近组织，应用集成与 E2E 集中管理。CI 已接入新命令，远端 GitHub 运行结果尚未验证。

更新采用 A 方案：SettingsService 创建并持有未配置的服务，Configure 一次性提供版本，借用视图不会关闭服务。Rust UpdateState 提供快照、命令及单执行者领取；`[updates].auto_check` 默认开启，默认时省略该段。控制器与“更新”设置分区已接入：启动 30 秒后首次检查，此后每 6 小时检查；兼容代码包静默下载，整包下载需确认，重启始终由用户触发。菜单栏指示器与浮层共用进度、重试和就绪状态。未检查和没有更新时隐藏入口；检查中显示无中心图标的灰色旋转细环，发现更新后显示下载入口。后台检查失败时隐藏入口，错误保留在设置页；手动检查失败时保留警示入口，设置页或已打开的浮层保留原因和重试操作，关闭浮层不会隐藏入口。自动下载代码包时同样显示进度环，不自动打开浮层；就绪界面使用紧凑系统下载图标、可换行说明和明确的重启操作；浮层通过右上角的主题 × 图标按钮关闭。无执行者交接；销毁其 VM 后需要重启应用。`usher-logging` 独立提供日志，并通过观察接口发布应用健康状态。`update-state` 与真实桥接测试使用隔离数据和可控原生 I/O。本地更新模拟另外验证本机更新源的签名代码包下载；独立真实归档测试已验证本地整包替换、重启、设置保留和失败恢复，0.1.1 公开发布及 standard 0.1.0 整包升级已在 macOS arm64 验证；远端代码包分发与 Gatekeeper 验收仍待完成。

SM6 本机回归已通过；RT4 后最新验证包含 229 项 Rust 测试、36 项应用集成、全部 UI / 无头旅程和 4 项隔离原生验收。七目标纯库及 Apple 四目标应用编译通过；完整其他平台应用构建、宿主和远端 CI 尚未完成，因此 SM6 保持部分完成。此前 SM6 隔离发布构建相对 06ed67e 的应用文件总量增加 1.10%（尚未重测 RT4），测量条件和原始证据见上述审计。

## 许可

Cargo 包声明为 `LicenseRef-Proprietary`，并设置了 `publish = false`。第三方依赖遵循各自许可证。
