# 应用更新方案

本文是更新模块的唯一设计依据。更新的行为、目录结构、发布流程或配置发生调整时，**先修改本文，再按本文实施**；代码与本文不一致时，以本文为准，并把差异当作待修复的缺陷或待更新的文档，二者不可并存。

本文区分三类内容：**已决定**的设计、**待验证**的假设、**当前实现**的状态。待验证项在第 11 节登记，验证结果回写到本文后才能据此实施。模块职责与依赖边界的通用规则见 [architecture.md](../architecture.md)，编码规范见 [CONTRIBUTING.md](../CONTRIBUTING.md)，任务状态见 [scaffolding.md](../scaffolding.md) 批次 06，发布流水线见 [deploy.md](deploy.md)。

## 1. 范围

| 在范围内 | 不在范围内 |
| --- | --- |
| 检查更新、下载、校验、待重启、重启或替换安装、失败处理 | 更新公告内容（06.07）、全局消息通知（06.08） |
| 代码包热更新与整包更新两条路径 | 开发期 HMR 热重载，它与用户更新无关 |
| 正式版通道（唯一通道） | 预览、alpha、beta、rc 等预发布通道；Windows、Linux、iOS、Android 的更新适配（当前只发布 macOS Apple Silicon） |
| 菜单栏更新按钮、设置页"更新"分区、更新面板 | 托盘与应用菜单入口（02.02 完成后接入同一语义动作） |

## 2. 决策记录

| 编号 | 决定 | 依据 |
| --- | --- | --- |
| D1 | 信任与清单采用 NSH 原生体系，不再自建签名 | 避免两套密钥和吊销机制并存；NSH 运行时本身会复验 |
| D2 | 两种更新路径合并为一次检查，由检查结果决定走哪条 | 与 NSH 官方流程一致：先找代码包，必要时才退回整包 |
| D3 | 整包路径：用户确认后下载、校验，自动替换并重启，无需手动安装 | 对齐 Slint 版 Usher 的更新体验；手动下载只作兜底 |
| D4 | 代码包路径：自动检查开启时静默下载，仅在待重启时提示 | 代码包小、不替换应用，打扰成本低 |
| D5 | 外壳版本过旧（需要整包）时只提示，不强制更新 | 用户决定 |
| D6 | 暂不启用 Apple 签名与公证，功能保留，拿到证书后启用 | 当前没有 Developer ID；NSH 包签名校验不受影响，始终开启 |
| D7 | **只有正式版一个通道**，不提供预览、alpha、beta、rc 的更新。预发布版本不发布更新，也不被任何客户端接收 | 用户决定（取代此前的稳定与预览双通道方案）；V2 验证更新源写在打包时的 `nsh-updater.json`，本就无法运行时切换 |
| D8 | 清单从 GitHub Release 读取：`updater.github` 指向本仓库，同时配置 `publicKey` | 用户决定；V1 验证 `github` 字段的地址固定为 `releases/latest/download/latest.json`，GitHub 的 latest 不含草稿与预发布 |
| D9 | 外壳兼容分两层：发布门禁按文件差异判断，外壳输入有改动的版本只发整包；**且外壳有改动时必须升主版本或次版本**，客户端只在代码包与外壳的主次版本相同时使用代码包 | 官方只用 `minRuntime`，无法拦截 Rust 与桥接变更。只比较相邻版本会漏掉累积错配：0.1.0 → 0.1.1（改 Rust，只发整包）→ 0.1.2（只改 TSX，发代码包），装着 0.1.0 外壳的用户会收到依赖 0.1.1 外壳的代码包 |
| D10 | 设置页新增"更新"分区：自动检查开关、检查按钮、版本信息；自动检查关闭时不检查、不静默下载 | 用户决定 |
| D11 | 更新模块的界面层（菜单栏按钮除外）独立成一个模块；逻辑层独立成 crate | 解耦，便于独立测试与替换 |
| D12 | 旧的自建签名与发布工具不为兼容历史而保留，用不到即删除 | 用户决定 |
| D13 | 第一次发布的版本是 `0.1.0`，此前的 `0.0.1-alpha.1` 不发布 | 用户决定；D7 之后门禁只接受正式版标签 |
| D14 | 提供本地更新模拟命令与 mock 数据，修改更新流程后可在本机走完整个流程，不依赖 GitHub 与真实密钥 | 用户决定；`nsh-updater` 的 `check_url` 允许 `http://127.0.0.1`、`localhost`、`[::1]`（"a local test"），无需自签名证书 |

## 3. 两种更新

| | 代码包热更新 | 整包更新 |
| --- | --- | --- |
| NSH 接口 | `checkForPackage`、`downloadPackage`、`restartToPackage` | `checkForUpdate`、`downloadUpdate`、`installAndRestart` |
| 内容 | 应用的 TSX、导入的图片与字体 | 整个应用：Rust 代码、原生组件、运行时、`nsh.json`、权限 |
| 生效 | 下次启动，或立即重启进入 | 替换已安装的应用并重启 |
| 校验 | 离线根密钥背书的发布密钥签名，限定本应用，构建号高于运行中与信任库下限，适配运行时 | 清单签名加哈希与大小校验 |
| 失败处理 | NSH 试运行：首帧后 10 秒或正常退出视为健康，否则删除并回滚 | 替换失败时保持旧版本 |
| 权限 | 不超过外壳 | 取决于新外壳 |

代码包**只含 TSX**。以下变更必须发整包：应用自己的 Rust crate、原生组件、桥接接口、权限、`nsh.json`（含 `updater` 配置）、编进二进制的资源。第一次引入 `updater` 配置的版本必然是整包，旧安装只能手动安装一次。

### 路径判断

```text
检查（读取 GitHub 最新正式版的清单）
 ├─ 清单含代码包，外壳满足 minRuntime，且代码包与外壳主次版本相同  → 代码包路径
 ├─ 有新版本，但没有可用的代码包         → 整包路径（只提示，面板写明"需要更新应用"）
 └─ 无新版本                             → 已是最新
```

同时存在两者时，外壳满足要求就走代码包。一次整包更新带上了代码，整包完成后不再单独更新代码包。`minRuntime` 只比较 NSH 运行时版本，不比较应用外壳；外壳兼容靠 D9 的两层：门禁保证外壳变了就升主次版本，客户端用 `shellInfo().version` 与代码包版本的主次版本比较，不同就按整包处理，面板写明"需要更新应用"。**客户端这一半尚未实现，属于批次 4、5。**

## 4. 发布侧

### 4.1 密钥与信任

NSH 原生体系有三类互相独立的材料，其中清单密钥与代码包的信任链是两条线：

| 材料 | 用途 | 保管 |
| --- | --- | --- |
| 清单密钥（`nsh keygen`） | 签署更新清单 `latest.json`；公钥写入 `updater.publicKey` | CI Secret，另有离线备份 |
| 根密钥 | 为发布密钥签发证书，签发吊销列表；公钥写入 `updater.packages.roots` | **离线**，由维护者在离线环境生成，不进入仓库，不进入 CI |
| 发布密钥与证书 | 签署 `.nshpkg`（`nsh publish --key --cert`），证书随包分发，可轮换而无需更新应用；`nsh certify` 限定可签的应用 ID 与有效天数 | CI Secret |

旧体系的发布密钥与恢复密钥在切换后作废，`trusted_keys.txt` 随之删除。Apple 签名与公证与以上密钥无关，沿用 `MACOS_SIGNING_NOTARIZATION_ENABLED`，默认关闭（D6）。

证书有有效期（`nsh certify --days`，默认 365 天）。到期前必须重新签发，否则新发布的代码包会被拒绝；到期时间应记入发布检查清单。

### 4.2 外壳输入与门禁（D9）

门禁脚本对比上一个发布标签：外壳输入文件有改动，则本次发布**只含整包**，不生成 `.nshpkg`；没有改动才同时生成代码包。

外壳输入的范围由配置文件 `scripts/release/shell-inputs.txt` 定义，脚本不写死路径。范围包括：

- `src/`、`platforms/`、`crates/`、`build.rs`、`Cargo.toml`、`Cargo.lock`。
- `ui/theme/` 与 `ui/config/background.json`：它们被 `include_str!` 编进二进制，改动需要重编译。
- `nsh.json`，比较时忽略 `version` 与 `build`，它们每次发布都会变。

**版本线约束（D9）。** 门禁用 `shell_inputs.py changed --version <本次版本> --strict` 找到上一个正式版标签，外壳输入有改动且主次版本没有升高时，真实发布失败，演练只给警告。预发布标签与格式不对的标签不算发布，不会成为比较基准。补丁版本因此只能是 TSX 变更：

| 变更 | 版本 | 产物 |
| --- | --- | --- |
| 只改 TSX、文案、样式之外的非外壳文件 | 补丁版本（0.1.0 → 0.1.1） | 整包与代码包 |
| 改了外壳输入 | 次版本或主版本（0.1.x → 0.2.0） | 只有整包 |

遗漏防护：CI 扫描 Rust 源码里所有 `include_str!` 与 `include_bytes!` 引用的文件，凡不在上述范围内即失败。新增编译期嵌入的资源目录时，必须同时更新该配置文件。

### 4.3 产物与清单（D7、D8）

| 项 | 内容 |
| --- | --- |
| 清单位置 | `releases/latest/download/latest.json` 与同目录的签名。GitHub 的 latest 只指向最新的非草稿、非预发布 Release，所以只有正式版 Release 会成为更新源 |
| `nsh.json` 的 `updater` | `github: "<owner>/<repo>"`、`publicKey`、`packages.roots`。只有这一种配置，不需要按构建生成多份 |
| 每次发布的产物 | 整包归档、`.nshpkg`（外壳输入未变时）、清单及签名 |

- 发布门禁只接受正式版标签（`v<major>.<minor>.<patch>`，无预发布后缀）。带连字符的标签被拒绝，不构建、不发布；`scripts/release_gate.py` 的 `channel_for` 与 `prerelease` 输出随之删除。**当前门禁与 [deploy.md](deploy.md) 仍按旧设计支持预发布标签，这部分在批次 2 改造，改造前以代码实际行为为准。**
- Release 创建时不带 `--prerelease`，保证它会成为 latest。
- 构建号单调递增（`nsh.json` 的 `build`，门禁已要求）。NSH 信任库记录每个应用已接受的最高构建号，据此拒绝降级。
- `.nshpkg` 内的 `nsh.json` 不含 `updater`，更新源始终来自外壳，不随代码包变化。

演练模式沿用临时密钥，不读真实密钥，不写 Release。演练在没有标签时运行，版本可以是预发布，仅用于验证构建链路；真实发布必须是正式版。

### 4.4 更新源的可见性

更新器用匿名 HTTP 读取 `https://github.com/<owner>/<repo>/releases/latest/download/latest.json`，不带任何凭据，也无法带（客户端没有令牌，也不应该有）。**仓库为私有时，该地址对所有用户返回 404，应用永远读不到更新。** 2026-11-12 未登录访问 `tianlai10000000/usher-nsh` 返回 404，仓库当前是私有的。

处理办法三选一，由维护者决定，**在此之前不能真实发布 `0.1.0`**：

| 办法 | 做法 | 代价 |
| --- | --- | --- |
| 仓库改为公开 | 在 GitHub 设置里改可见性 | 源码公开 |
| 单独的公开发布仓库 | 新建一个只放 Release 的公开仓库，`updater.github` 指向它，工作流用令牌把产物发到那里 | 多一个仓库与一个跨仓库令牌，发布作业要改 |
| 自托管更新源 | `updater.url` 指向自己的 HTTPS 地址（须同时配置 `publicKey`，已配置），发布时把文件传上去 | 要有托管，且 D8 的"从 GitHub Release 读取"不再成立 |

门禁目前要求 `updater.github` 等于本仓库，并拒绝 `updater.url`。选第二或第三种办法时，这条规则要随之修改，并同步本文 D8。

### 4.5 首次发布（D13）

- 第一次发布是 `0.1.0`。发布前按 [deploy.md](deploy.md) 的"发一个版本"步骤，把 `nsh.json` 与根 `Cargo.toml` 的版本同改为 `0.1.0`，`build` 加一，并在 `CHANGELOG.md` 写 `## 0.1.0` 小节。版本号现在仍是 `0.0.1-alpha.1`，**在发布前不改**，避免打断平时的开发构建。
- `0.1.0` 是第一个带 `updater` 配置的版本，没有任何旧安装能更新到它，用户只能手动安装一次（第 3 节）。
- 因为只有正式版通道，**`0.1.0` 本身验证不了更新**：端到端验证需要再发一个更高的正式版（例如 `0.1.1`），让已安装的 `0.1.0` 去检查、下载、替换。代码包路径同样需要外壳输入没变的 `0.1.1`；整包路径需要外壳输入有变的 `0.1.1`。所以至少要连续发布两个版本才能把两条路径各验证一遍。
- 在 `0.1.0` 发布前，只能用演练模式和本地的临时更新源验证；演练证明不了真实更新密钥、发布步骤和 Release 的 latest 行为。真实 Release 一旦发布，标签不可移动，所以 `0.1.0` 的发布前检查要比往常更严格：根密钥与证书有效期、`publicKey` 与 `roots` 写入 `nsh.json`、`nsh verify --root` 对产出的 `.nshpkg` 通过。

## 5. 客户端流程

### 5.1 步骤

```text
检查 → 验证清单 → 下载 → 校验 → 待重启 → 重启（代码包）或保存状态后替换并重启（整包） → 首次启动确认
```

- 重启前先保存设置，设置 Store 已有原子写入。
- 每一步失败都清理临时文件并回到可重试状态；整包替换失败时旧版本保持可用。
- 代码包的试运行与回滚由 NSH 负责。应用额外记录失败的版本与次数，用于面板显示；**同一版本连续失败两次后停止自动提示**，只在用户手动检查时重试。这条规则是为了避免"更新失败后反复提示同一个更新"的循环。
- 失败信息保存失败类别与版本，面板翻译显示；整包失败时给出手动下载链接。

### 5.2 触发与下载策略（D4、D10）

| 自动检查 | 检查时机 | 代码包 | 整包 |
| --- | --- | --- | --- |
| 开启 | 启动后延迟一次，之后定期 | 静默下载，待重启时提示 | 只提示，用户点击后才下载 |
| 关闭 | 只在用户点击"检查更新"时 | 手动检查后由用户确认下载 | 同左 |

后台检查失败只记日志，不打扰用户；手动检查的结果在面板中反馈，不使用系统弹窗。从不强制重启，待重启状态保留到用户点击或下次启动。

### 5.3 更新源（D7、D8）

- 更新源是构建属性。NSH 在打包时把 `nsh.json` 的 `updater` 写入应用包内的 `nsh-updater.json`，`checkForUpdate` 与 `checkForPackage` 只读这份配置，运行时不能更改（V2）。
- 应用只有一个更新源：本仓库最新的正式版 Release。设置里没有通道选项，也没有通道偏好。
- 状态机不再有通道概念：不接收预发布版本、不降级，两条规则都由 NSH（`latest` 地址与 `version <= current` 判断）保证，状态机只保留一道防御性校验，拒绝带预发布后缀的候选。
- 构建号只增不减：接受过某个构建的安装不会再接受构建号更低的包（NSH 信任库），这也是防降级的依据。

## 6. 菜单栏按钮

按钮由 `ui/layout/components/Rail.tsx` 渲染，只读取 `ui/layout/contracts/updates.ts` 定义的状态投影并上报按下，不依赖更新模块的内部实现。

### 6.1 视觉形态

参考成熟客户端（ChatGPT 桌面端）的菜单栏更新按钮，按钮只有三种形态，状态靠形态而不是角标区分：

| 形态 | 画法 | 用在 |
| --- | --- | --- |
| 静默 | 与设置按钮同款的普通图标（`arrow.down.circle`，文字色），悬停时有浅灰圆角底 | 已是最新、检查中、代码包静默下载中 |
| 强调 | **实心强调色圆**，圆内是白色（强调前景色）的下载图标；不再用角标小圆点。圆的直径取 `size-avatar`，与下方头像对齐 | 发现新版本、需要更新应用、待重启 |
| 进度 | 灰色底环加一段较深的弧表示进度，弧从 12 点钟方向顺时针增长；环中央是一个小圆角方块（参考图的样式） | 下载中 |

悬停时按钮背景出现浅灰圆角底，提示在按钮右侧弹出，样式沿用 `ButtonTooltip`。强调形态在悬停时只做不透明度变化，不叠加灰底。尺寸、颜色、弧宽与动画时长全部走主题令牌，不在组件里写字面量；新增令牌须同步所有内置主题。总大小未知时弧改为匀速旋转，减少动效时静止显示。

**中央方块不表示"取消"**：NSH 的下载没有取消接口（V12），按钮在进度形态下**不可按**，只作状态展示，详情在面板里。因此中央方块是纯装饰，不承诺可中断；如果之后要支持取消，需要 NSH 先提供接口，再改本节。进度形态不使用禁用的半透明样式，保持原样显示。

### 6.2 状态表

| 状态 | 形态 | 提示文案 | 按下后 |
| --- | --- | --- | --- |
| 已是最新 | 静默 | 检查更新 | 立即检查，面板反馈结果 |
| 检查中 | 静默，图标旋转 | 正在检查更新 | 打开面板 |
| 代码包静默下载中 | 静默，不打扰 | 检查更新 | 打开面板可看进度 |
| 代码包待重启 | 强调（对勾替换下载图标） | 重启以应用更新 | 面板里"立即重启"或"下次启动时" |
| 发现新版本（整包，或手动检查到） | 强调 | 发现新版本 X | 面板里"下载并更新" |
| 下载、校验中 | 进度（有总大小时为确定进度，否则旋转） | 正在下载 X | 不可按；面板可看详情。NSH 无取消接口，校验含在下载内，不单独显示（V12） |
| 整包待重启 | 强调（对勾） | 重启以完成更新 | 保存设置后替换并重启 |
| 需要更新应用（外壳过旧） | 强调 | 需要更新应用 | 面板说明并提供整包更新 |
| 失败 | 强调，改用警示色 | 更新失败 | 面板显示原因、重试、手动下载 |
| 系统管理、不适用 | 禁用 | 更新由系统管理 | 无 |

契约现在只有布尔的 `available()` 与 `request()`，需要扩展为上表的状态投影，仍由布局模块拥有，更新模块实现它。

## 7. 设置页"更新"分区

内容：自动检查开关（D10）、"检查更新"按钮、当前外壳版本、代码包版本与构建号（来自 `shellInfo()` 与 `runningPackage()`）、上次检查时间与结果、整包失败时的手动下载链接。

设置页通过 `settingsSections` 注册该分区，分区内容由更新模块提供。设置页只持有一个分区 ID 与一行挂载，不读取更新模块内部状态。

## 8. 模块与目录

### 8.1 总览

```mermaid
flowchart LR
    Rail[菜单栏按钮<br/>ui/layout] -->|读取状态投影| Contract[UpdateIndicator 契约<br/>ui/layout/contracts]
    Module[更新界面模块<br/>ui/features/updates] -->|实现| Contract
    Module -->|发送事件 读取快照| Bridge[src/update_bridge.rs]
    Bridge --> Crate[crates/usher-update<br/>状态机与策略]
    Module -->|执行检查 下载 重启| NSH[nsh/updater]
    Bridge --> Settings[SettingsStore<br/>updates 偏好]
```

依赖方向：更新界面模块依赖布局契约，**布局不依赖更新模块**；更新 crate 不依赖 NSH、窗口与界面；NSH 的更新接口只由更新界面模块调用。

### 8.2 逻辑层：`crates/usher-update/`

沿用现有 crate，按本文重塑，删除用不到的部分（D12）：

| 部分 | 处置 |
| --- | --- |
| `state.rs`（状态机） | **保留并改造**：增加更新种类（代码包、整包）、失败版本与次数、外壳过旧的结果；删除 `Channel`、`ChannelChanged` 与通道切换逻辑，只保留"拒绝预发布候选"的防御性校验；删除自建签名相关的 `verified` 与 `Unverified`、`min_version` |
| `signature.rs`、`trusted_keys.txt` | **删除**：被 NSH 原生校验取代 |
| `crates/usher-update-tool/` | **删除**：密钥与清单由 `nsh keygen`、`nsh update-manifest` 等命令处理 |
| 依赖 `ed25519-dalek`、`base64`、`sha2` | **删除** |

crate 保持无 I/O、无 NSH 依赖：输入事件，输出状态与效果。网络与文件操作由调用方执行后把结果作为事件送回，沿用现有的票据机制丢弃迟到回执。

#### 状态机设计

**一个状态机，两条路径共用状态与事件，差异放在策略里。** 阶段名不变（`Idle`、`Checking`、`Checked`、`Downloading`、`Verifying`、`ReadyToInstall`、`Installing`、`AwaitingFirstLaunch`、`InstallFailed`），每个候选带 `Kind`（`Package` 或 `Shell`）。代码包路径的"安装"就是"重启进入新包"，整包路径的"安装"是替换并重启。

| 旧 | 新 |
| --- | --- |
| `Outcome::InAppUpdate` | 拆成 `PackageUpdate(Candidate)` 与 `ShellUpdate(Candidate)` |
| `Outcome::ManualUpgrade { reason }` | 保留；原因为 `ShellTooOld`（见下）与 `Environment`（`canInstall` 为 false，给手动下载链接）；删除 `ReleasePolicy`、`Unverified` |
| `Channel`、`Event::ChannelChanged`、`Channel::accepts` | 删除。仅保留"拒绝带预发布后缀的候选"的防御性校验 |
| `Candidate.min_version`、`Candidate.verified` | 删除 |
| `Failure::Signature` | 删除，由下面的五类取代 |
| `Event::Cancel`、`Effect::CancelRunning`、下载中取消 | **删除。** NSH 没有取消接口（V12）：更新操作串行，进行中再次调用返回 `an update operation is already in progress`，`updater.d.ts` 里没有 cancel 或 abort。保留一个调不到的取消事件会让状态机表达做不到的事 |

**失败分类**（用户决定，分得细以便排查；每一类对应面板上的一种处理。共六类，`Busy` 是选项 B 之后新增的）：

| 类别 | 含义 | 重试有用吗 | 面板处理 | 日志级别 |
| --- | --- | --- | --- | --- |
| `Network` | 连不上、超时、连接被中断 | 有用 | 重试 | `WARN` |
| `Busy` | NSH 报 `an update operation is already in progress`：更新操作串行，另一个还没结束（窗口重建撞上进行中的操作时会出现） | 有用，但要等 | 提示"另一个更新操作进行中，请稍后"，不自动重试 | `WARN` |
| `Rejected` | NSH 拒绝了包：签名、应用 ID、构建号不新于已接受的、运行时太旧、哈希或大小不符 | **没用** | 此版本无法安装；同一版本失败后后台不再提示 | `ERROR` |
| `Disk` | 写临时文件或替换应用失败 | 视情况 | 提示检查磁盘与目录权限 | `ERROR` |
| `Cancelled` | 用户在应用侧放弃（例如关闭面板、退出应用时的等待），**不是 NSH 取消** | 无需 | 回到可重试状态，不计入失败次数 | `INFO` |
| `Unknown` | 以上都不是 | 不确定 | 重试，并建议导出日志 | `ERROR` |

`Rejected` 内部无法由状态机再细分，因为 NSH 只返回一条消息。细分原因放在日志里（见"更新日志"），不影响状态机逻辑。

**失败次数的计法**：只有 `Rejected`、`Disk`、`Unknown` 计入某版本的失败次数（它们重试大概率仍失败，达到上限后后台不再提示）。`Network`、`Busy` 是暂时性的，`Cancelled` 不是失败，三者都**不计入**，否则离线或窗口重建两次就会让一个正常的更新被永久压下。

**外壳过旧**（D9 的客户端一半）。检查输入带两个版本：`shell`（`shellInfo().version`，安装的应用外壳）与 `running`（当前运行的版本，运行着代码包时高于外壳）。判断是纯函数：

- 候选版本不高于 `running`：已是最新；
- 候选带代码包，且候选与 `shell` 的主版本、次版本都相同：`PackageUpdate`；
- 其余，即候选没有代码包，或主次版本与外壳不同：`ShellUpdate`（面板写"需要更新应用"）。

**失败记忆**（防止反复提示同一个失败的更新）。状态机记录每个版本的失败次数；某版本失败次数达到上限后，**后台检查不再提示它**，手动检查仍会重试。上限是构造参数，数值放配置（第 9 节），不写在逻辑里。

**不取消、不交接**（用户决定，选项 B）。NSH 不能取消更新操作（V12），所以应用层也不提供取消，也不提供执行者交接：

- `src/update_service/` 中以下内容删除：`Command::Cancel`、`Command::Channel`、`reset_after_stop`、`Model.recovery` 与 `Model.draining`、`Settlement`、`Recovery`、`revoke`、`settle`、`reconcile`、`Execution::Draining`、`Execution::RecoveryRequired`、`Error::RecoveryRequired`。它们存在的全部理由是"撤销旧身份后确认原操作已停止"，而这个确认在 NSH 下做不到。
- 保留单执行者的核心：`acquire`、`claim`、`progress`、`complete`、`Lease`、`Claim`、票据与迟到回执拒绝、`close`。
- 执行者持有租约直到服务关闭。窗口销毁时调用 `close`，`Machine` 不重置；重建后的服务从 `Idle` 开始（`UpdateService::new` 已要求 `phase == Idle`）。**取舍：** 进行中的 NSH 操作撞上窗口重建时，状态机会丢失它，而 NSH 的操作仍在运行，新服务发起检查会收到 `an update operation is already in progress`。这在生产里很少发生（HMR 与多窗口关闭重开主要出现在开发期）。已落盘的 `next.nshpkg` 不受影响，下次启动 NSH 自己处理。

**更新日志**（用户决定：错误详情要记录到日志，日志开启才写）。纯逻辑 crate 不做 I/O，所以状态机只**产生**日志记录，由调用方写出：

- 每次状态变化与每次失败产生 `Effect::Log(Record)`，`Record` 含级别、事件名和结构化字段（版本、种类、失败类别、票据、已失败次数），不含自由文本。字段是类型化的，可以直接用 `cargo test` 断言"某个失败产生某条记录"，不需要真的写日志。
- 桥接（批次 4）执行 `Effect::Log`，调用日志 crate 写出；日志级别关闭时日志 crate 自己丢弃，不需要状态机判断。
- NSH 返回的错误消息是自由文本，由桥接作为 `detail` 字段与 `reason` 一并写入：`reason` 用一张映射表把已知的 NSH 消息归为更细的原因（如 `signature`、`build_not_newer`、`runtime_too_old`），映射不上记 `unclassified`。**`reason` 只进日志，不影响状态机**，所以 NSH 升级后消息文字变化只会让 `reason` 变成 `unclassified`，应用行为不变，排查时看 `detail`。
- 脱敏：用户决定**不做过多过滤，避免排查困难**。只做一项：写日志前把用户主目录前缀（`$HOME`）替换成 `~`。其余字段与消息保留原样。这与现有日志模块的按字段名脱敏（`token`、`secret`、`body`、`content` 等）不同，**拆分日志 crate 时需要重新决定脱敏范围**（见 8.6）。

**下载策略**（D4、D10）。构造参数 `auto_check` 与路径共同决定：开启时代码包直接产生 `StartDownload`，不需要用户操作，整包只产生 `Notify`；关闭时后台检查不发生，手动检查的结果都只产生 `Notify`，由用户确认后才下载。

### 8.3 桥接层：`src/update_bridge.rs`

提供 DTO 转换和事件入口，状态机实例由应用装配在同一状态作用域内唯一持有；把 `auto_check` 偏好委托给 `SettingsStore`，不另存一份。更新源来自构建，不是偏好。业务错误返回稳定错误码，界面翻译。设置新增 `[updates]` 段，只含 `auto_check`，缺省时不写入文件，沿用现有的 schema 迁移与保存规则。

共享状态接入遵循 [通用状态管理方案](state-management.md)：业务状态机继续留在 `usher-update`，应用装配按作用域持有实例，桥接负责动作适配和订阅投递，复用独立 `usher-state` crate 的后端无关接口。watch 仅为可选内部实现，更新模块不得依赖其通道类型；TSX 更新状态通过项目门面和集中的 NSH 响应式适配访问，菜单、面板与设置分区不各自保存状态副本。不得把更新阶段、下载策略或 `nsh/updater` 调用移入通用 crate。多窗口只订阅同一状态，效果由单一执行者领取；沿用界面侧调用 NSH 更新接口的边界。通用 crate 与 SM1 至 SM4 已实施。

#### SM5 兼容接入边界

本轮先实施应用层 `src/update_service/`，适配当前 `Machine`，不改旧领域 API、设置 schema 或落盘格式。更新批次 3 的领域重构、批次 4 的 NSH 桥接 / 自动检查设置及批次 5 的产品界面仍按原顺序交付；本批不启用网络下载、安装或后台定时器。协调器通过 Rust 公共契约验证第二领域共享发布，后续桥接复用同一实例，不再创建第二个 Machine。

- 应用注册表以宿主设置服务的 StoreId 为作用域；同目录的消费方借用同一更新服务，不同临时目录独立。无数据目录的会话由所属装配持有并显式关闭。
- 纯领域转换在副本中计算，快照成功提交后才更换内部模型和效果队列；发布失败不丢任务、不推进票据、不允许执行候选效果。订阅只含状态和队列数量，不广播效果。
- 单一执行租约与单一在途领取均由服务签发，绑定 StoreId、租约代次、效果 ID 和原领域票据。重复领取返回空；普通消费方只能提交用户命令，不能伪造网络回执。
- 撤销执行者立即拒绝旧进度 / 回执；有在途操作时进入 draining，直到原执行器确认 Promise 已结束或操作确已停止才开放新租约。当前 NSH 无取消 API，不能以超时、组件卸载或租约失效代替停止确认。
- 安装交接结果未知时进入 recovery_required，禁止重领、重试和新租约；只有可信宿主持有同一 Claim 并确认未安装或已交接才解除，迟到的旧恢复查询不能解除新操作的屏障。NSH 0.5.0 没有通用在途任务查询 / 取消 API，因此接入层不得自动确认；查询与重启恢复仍由更新批次 4 / 5 实施。
- 本批 mock 执行器留在更新集成测试，使用可控完成顺序、隔离设置目录及 `usher-test-support`；不会访问真实更新源、密钥或应用安装位置。

### 8.4 界面层：`ui/features/updates/`

| 文件 | 职责 |
| --- | --- |
| `controller.ts` | 把状态机的效果翻译成 `nsh/updater` 调用，再把结果作为事件送回桥接；负责定时检查与清理 |
| `indicator.ts` | 把更新状态适配成布局的 `UpdateIndicator` 契约 |
| `UpdatePanel.tsx` | 按钮旁的浮层 |
| `UpdatesSettings.tsx` | 设置页分区 |
| `index.ts` | 对外只导出 `createUpdates`，由 `App.tsx` 创建并传给 `Frame` 与设置页 |

菜单栏按钮的渲染留在 `ui/layout/`。布局组件使用 NSH 组件库的公开接口，遵循 CONTRIBUTING 对布局的限制。文案使用 `i18n/messages/` 的 `update.*` 键，中英文同步。

### 8.5 发布工具

不新建 crate。外壳输入检查与发布门禁写成 `scripts/release/` 下的脚本，由 `release_gate.py` 与工作流调用，测试放在 `scripts/tests/`。

### 8.6 日志 crate（待实施，等批次 3 的并行改动结束）

用户要求把日志模块拆成独立 crate，让程序任何位置都能输出或记录日志。当前实现是 `src/diagnostics.rs`，**不能原样搬出**，原因有两个：

1. **它依赖应用根 crate。** 另一条并行工作在 `Shared` 里加了 `reporting` 字段与对 `crate::diagnostic_state::StatusService` 的依赖，用来把健康状态与丢弃计数发布到状态订阅。独立 crate 不能依赖根 crate，所以必须把它反过来：日志 crate 暴露一个回调或 trait（例如"状态变化观察者"），由应用根注入实现。
2. **它的脱敏规则与"少过滤"的要求冲突。** 现在的 `redact` 与 `redact_text` 按字段名（`token`、`secret`、`body`、`content`、`email` 等）替换值，更新日志里这类字段名会被抹掉。用户要求尽量不做过滤，拆分时保留哪些：**用户决定只保留凭据类**，即字段名含 `token`、`secret`、`authorization`、`bearer`、`password`、`apikey`、`credential` 的值，以及自由文本里的 `Authorization:` 与 `bearer` 令牌；`body`、`content`、`email`、`cookie` 等字段不再替换。另外把用户主目录前缀替换成 `~`。
3. **TSX 仍只能上报短标识。** 这条是 [architecture.md](../architecture.md) 第 8 节与 05.01 的既有约束，不因拆 crate 改变；NSH 的错误详情经 Rust 桥接写入，不经 TSX。

目标是：日志 crate 无 NSH 依赖、无窗口依赖，提供初始化、写入、级别设置、导出，以及上述观察者接口；根 crate 保留 `diagnostic_state` 适配与 `settings_bridge` 的 `log_event`。**实施前提：** 并行的状态管理与更新界面改动已提交或结束，否则会与它们对 `diagnostics.rs` 的未提交改动冲突。

## 9. 配置

检查延迟、检查间隔、失败停止阈值等放在 `ui/config/updates.json`，TSX 侧读取，不在代码里写字面量。**具体数值在实施前单独列出，经批准后写入**，见 CONTRIBUTING 对硬编码的要求。更新源地址属于外壳，写在 `nsh.json` 的 `updater` 中（字段名见 V1）。

## 10. 安全与失败边界

- NSH 包签名与根密钥校验始终开启，**不提供运行时跳过校验的开关**。D6 只关闭 Apple 签名与公证。
- 根密钥私钥不进入仓库与 CI。
- 清单与包的校验失败不得回退到未校验的路径。
- 吊销列表与防降级下限由 NSH 信任库持久化，应用不另存。
- 日志使用结构化事件，不记录凭据与用户正文。

## 11. 验证记录与待验证项

第 1 批在本机 `nsh 0.5.0`（`nsh-core` 固定为 0.5.0）上验证，方法是读 `nsh-updater`、`nsh-engine`、`nsh-macos-app` 0.5.0 的源码，并在临时目录的项目副本和临时 `HOME` 中用一次性密钥实际打包、签名、运行；没有改动仓库文件，临时文件已删除。

| 编号 | 问题 | 结论 | 状态 |
| --- | --- | --- | --- |
| V1 | 更新源字段 | `updater.github`（`owner/repo`）或 `updater.url`（HTTPS）二选一；`publicKey` 是清单密钥，用 `url` 时必填，用 `github` 时设置后拒绝未签名清单；`packages.roots` 至少一个根公钥。`github` 的地址固定为 `releases/latest/download/latest.json`。打包后写入应用内的 `assets/nsh-updater.json` | 已验证 |
| V2 | 运行时能否切换更新源 | **不能。** 配置在打包时写入应用，`manifest_url()` 只读它。更新源因此是构建属性，单一更新源（D7、5.3） | 已验证 |
| V3 | 应用能否自己加载 `.nshpkg` | **能。** 0.5.0 的运行器 `main()` 调用 `nsh_engine::updates::launch`（此前 06.04 的否定结论基于 0.4.0，已过时）。实测完整周期：`next.nshpkg` 在下次启动被运行并改名 `trying.nshpkg`；进程被杀后下次启动日志为 `the package started last never ran 10s without a fatal error; it is deleted and the app goes back`，包被删除并回到内置代码；正常运行超过 10 秒则晋升为 `current.nshpkg`，并创建 `trust/`。数据目录为 `~/Library/Application Support/<app id>/updates/` | 已验证 |
| V4 | NSH 0.5.0 是否具备本文依赖的接口 | 具备：`nsh` 命令行含 `keygen`、`certify`、`publish`、`update-manifest`、`verify`、`revoke`、`package`；`nsh-updater`、`nsh-package`、`nsh-engine` 源码均在本机 | 已验证 |
| V5 | 整包替换后未签名 `.app` 能否启动 | 替换前 `nsh-updater` 要求 `codesign --verify --deep --strict` 通过；`nsh package` 产物为 ad hoc 签名，实测通过，所以**不依赖 Apple 证书**。替换用目录改名，失败时放回旧的。**替换后首次启动是否被 Gatekeeper 拦截未验证**，取决于下载文件是否带隔离标记 | 部分验证，真机待验 |
| V6 | 发布命令流程 | `keygen`、`certify`、`publish` 已实跑：产出 487 KB 的 `.nshpkg`，`nsh verify --root` 通过，`nsh publish --key --cert --build --app-version -o` 可用。`update-manifest` 只读了参数（`--package`、`--key`、`--notes`、`--base-url`、`--out`、位置参数为整包归档），**未实跑** | 部分验证，批次 2 补 |
| V7 | 与 Sparkle 设想是否冲突 | 不冲突，也不需要 Sparkle：整包替换由 `nsh-updater` 的 `install` 完成。06.04 的 Sparkle 设想作废 | 已验证 |

### 仍待验证

| 编号 | 问题 | 影响 |
| --- | --- | --- |
| V5 | 经真实下载（带隔离标记）的整包替换后，首次启动是否被 Gatekeeper 拦截 | 整包路径在无 Apple 证书下的真实体验；若被拦截，面板需要给出放行说明 |
| V6 | `nsh update-manifest` 的实际输出与签名文件格式 | 批次 2 的发布流水线 |
| V8 | `installAndRestart` 在应用位于 `/Applications`（用户可写、系统管理员安装）与其他位置时的行为，以及 `canInstall` 的判定 | 面板何时给手动下载链接 |
| V9 | AppKit 变体（`macos-appkit`）与标准变体的整包是否可以用同一份更新清单 | **已决定**：清单只列 `standard` 变体的 `.tar.gz`（`update-manifest` 按 `<os>-<arch>` 只收一个），`appkit` 变体只作为手动下载的 zip。`appkit` 用户经更新会换成 `standard`，是否可接受待确认 |
| V10 | 仓库私有时更新器能否读到更新 | **不能**，见 4.4。实测未登录访问返回 404 |
| V12 | 进度环用什么画 | **已查 `nsh-core` 0.5.0 源码**：`nsh/base` 的 `Progress` 只有线性条；NSH 有 `<canvas>`（2D 上下文含 `arc`、`lineCap`，数值可绑定动画值）和 `<svg>`（需 `svg` feature）。倾向 `<canvas>` 画环，**但本仓库尚未用过 canvas，TSX 侧 API 与动画写法待在 `tests/ui` 里实测**；不需要新增原生组件 |

已验证（V12 之外）：`downloadUpdate` 与 `downloadPackage` 的 `onProgress` 回调给出 `{ received, total? }`，`total` 在服务端不给长度且清单没有大小时为空（空则退化为不确定旋转）；**没有取消接口**，下载是一个不可中断的异步任务，且同一时刻只允许一个更新操作（再次调用会被拒绝）；校验在下载 Promise 内部完成，**没有独立的校验阶段回调**，所以界面无法区分"下载中"与"校验中"。

## 12. 本地更新模拟（D14）

本地模拟用于更新流程回归，不访问 GitHub 或真实密钥。当前可执行范围是预置代码包的校验、启动、健康晋升和回滚；检查、下载与整包替换仍由 pending 场景跟踪。更新流程变化时，场景与断言同步更新。

当前实施批次（SM6 本机续验）：将模拟器改为每轮唯一测试应用 ID，旧壳与新代码包使用相同测试身份和一次性测试密钥；每个场景的数据、缓存和 saved-state 均位于独立临时根。通过系统目录 API 为测试 ID 创建专属链接，不改 HOME，拒绝覆盖现有目录或悬空链接。先停止并确认测试进程退出，再删除本轮链接和临时树；停止失败时保留隔离并报告失败。源码、应用和密钥在整轮结束后清理，构建仅复用 Cargo 缓存，执行日志与 JSON 证据保留。`serve` 由脚本启动及拥有测试应用，Ctrl+C 后统一清理，不再输出可脱离隔离生命周期的手工启动命令。有效场景完整执行与 pending 分开统计；仅有 pending 的运行不得作为验证成功。隔离与清理单测位于 scripts/tests/test_update_sim.py，共享链接保护逻辑复用于发布路由验收。逐场景状态、进程日志和清理证据保留在 target/test-reports/update-sim-*/；测试密钥不进入报告。2026-11-13 在本机 macOS arm64 最终完整运行通过 4 项、失败 0 项、pending 7 项，报告为 target/test-reports/update-sim-d8uds6dq/report.json；核对 16 个唯一隔离路径，残留为 0。真实 SIGTERM 清理另外验证通过，中断运行不计入成功数量。

### 12.1 命令

```bash
scripts/update-sim/sim.sh list                  # 列出场景
scripts/update-sim/sim.sh run <场景>            # 运行一个场景并断言结果
scripts/update-sim/sim.sh run --all             # 运行全部场景
scripts/update-sim/sim.sh serve <场景>          # 启动隔离测试应用与更新源，Ctrl+C 后清理
scripts/update-sim/sim.sh clean                 # 删除旧 .nsh/update-sim/ 缓存，保留验收报告
```

### 12.2 工作方式

| 部分 | 做法 |
| --- | --- |
| 更新源 | 本机 `http://127.0.0.1:<端口>`，用 Python 标准库的 HTTP 服务器提供静态文件，可按场景限速、中途断开、篡改响应；不新增依赖 |
| 密钥 | 每次运行用 `nsh keygen` 现场生成一次性的清单密钥、根密钥与发布密钥，`nsh certify` 签发证书，放在本轮临时树，结束后删除。**不读取任何真实密钥** |
| 被测应用 | 用 `nsh package` 打出"旧版"应用，`updater.url` 指向本机更新源。每轮生成唯一测试 ID，旧壳与新代码包身份相同，测试副本移除公开 scheme；只复用 target/update-sim-build 的 Cargo 产物，临时应用不跨轮复用 |
| 新版产物 | 用 `nsh publish` 签 `.nshpkg`，`nsh update-manifest` 写清单与签名 |
| 隔离 | 不改 HOME。系统 API 提供 data/cache/library 位置，只在唯一测试 ID 的命名空间创建链接，指向每个场景的临时数据根；已有路径或悬空链接均拒绝。进程确认退出后删除拥有的链接和数据；停止失败或路径归属变化则保留隔离并失败 |
| 判定 | 对每个场景检查应用数据目录的文件状态（`next`、`trying`、`current`、`trust/`）和应用日志，不靠截图 |

### 12.3 场景与 mock 数据

场景由 `scripts/update-sim/scenarios/*.json` 定义，每个文件描述清单内容、更新源行为、应用启动动作和预期。新增场景只加文件，不改脚本。

场景有两种状态。**有效**的场景走预置 `next.nshpkg` 的启动路径，应用不需要检查更新就能触发，已在本机运行通过。**`pending`** 的场景需要应用自己调用 `checkForUpdate` 或 `checkForPackage`，而这些调用要到批次 4、5 才有；它们在 `run --all` 里显示为 pending，不计入通过；必须至少执行一个有效场景且所有执行场景成功，命令才返回 0。只运行 pending 场景返回非零。批次 5 完成后删除场景文件里的 `pending` 字段并跑通，才算验收。

| 场景 | 模拟什么 | 预期 |
| --- | --- | --- |
| `up-to-date` `pending` | 清单构建号不高于当前 | 检查结果为已是最新，数据目录无变化 |
| `package-update` ✅ | 更高构建号的代码包，外壳满足要求，预置为 `next.nshpkg` | 启动后运行新包，10 秒后晋升为 `current.nshpkg`，`trust/` 创建，无残留。已通过 |
| `shell-update` `pending` | 清单只有整包，没有代码包 | 不提供代码包；整包下载、校验、替换后新版本启动。模拟里的整包是占位文件，真正的替换还需要真实整包归档 |
| `bad-signature` `pending` | 清单签名被篡改 | 拒绝，报签名错误，数据目录无变化 |
| `wrong-app-id` `pending` | 清单属于别的应用 | 拒绝，报应用不符 |
| `tampered-package` ✅ | 包内容被改一个字节 | 启动时拒绝，日志含 `the signature does not verify`，不出现 `trying` 与 `current`。已通过 |
| `downgrade` ✅ | 包构建号不高于应用的 | 拒绝运行，日志含 `is not newer than the app's`。已通过 |
| `crash-on-start` ✅ | 新包从未证明健康（进程被杀） | 下次启动回滚，包被删除，日志含 `never ran 10s without a fatal error`。已通过 |
| `download-fail` `pending` | 更新源中途断开 | 失败，可重试，不留残留的 `.part` 文件 |
| `slow-download` `pending` | 限速 | 进度单调递增；取消后可重新开始，用 `serve` 在界面上观察 |
| `offline` `pending` | 更新源不可达 | 后台检查静默，手动检查报网络错误 |

### 12.4 范围与限制

- 当前只有 4 个场景有效，7 个是 `pending`（见 12.3）。有效场景覆盖的是 NSH 引擎启动时的校验、试运行与回滚；检查、下载、取消等应用侧行为要等批次 4、5。
- 模拟覆盖 NSH 更新接口与应用数据目录的行为，**不覆盖 UI**：按钮状态与面板由界面用例（`tests/ui/`）覆盖，批次 5 加入。`serve` 命令由脚本拥有应用进程并提供人工观察会话，Ctrl+C / SIGTERM 后统一停止、清理；结果记录为 observed，不计作自动验收通过。
- 整包替换在模拟里替换的是临时目录里的 `.app`，不碰真实安装。Gatekeeper 与隔离标记无法在本机模拟（V5），仍需真机验证。
- 它证明不了真实 GitHub Release、真实密钥与证书有效期的行为。
- 与真实更新的差异：更新源是 HTTP 而不是 HTTPS（NSH 只对本机地址放行），其余校验路径相同。
- 每轮重新构建唯一身份的旧版应用，只复用 Cargo 编译缓存，不复用旧应用或测试密钥。clean 保留报告及停止失败时保留的隔离树，后者须先确认进程已退出再人工处理。

## 13. 实施批次

通用状态管理的前置关系见 [状态管理实施计划](state-management.md#9-分批实施计划)：本方案批次 3 的纯业务改造可独立进行；批次 4 的共享状态桥接以 SM1 至 SM3 为前置，批次 5 的界面投影以 SM4 为前置，第二领域复用与效果单执行者在 SM5 验收。此依赖不改变下表批次 3、4、5 的串行顺序；本轮只补设计，不推进代码状态。

| 批次 | 内容 | 状态 |
| --- | --- | --- |
| 1 | 验证 V1 至 V7，回写本文 | 已完成大部分，V5、V6 部分项和 V8、V9 留待后续 |
| 2 | 信任与发布（以首次发布 `0.1.0` 为目标，D13）：离线根密钥（由维护者生成）、`nsh.json` 的 `updater` 配置、外壳输入门禁、门禁只接受正式版标签、流水线产出 `.nshpkg` 与清单 | **演练已在 GitHub 上通过，真实发布待做**。密钥已生成并写入 `nsh.json`，三个 Secret 已配置。2026-11-12 在 GitHub 上手动运行 Release 演练：`gate`、`verify`、两个变体的 `build-macos`、`rehearse` 全部通过，`Sign and publish` 按设计跳过；`rehearse` 的三种损坏（换归档、去掉签名、与外壳结论不符）都被拒绝。**仍未验证，只有真实发布才会暴露**：三个真实 Secret 是否配对，`release` 环境、草稿转正式、`--latest`、构建来源证明。**代码包路径（`nsh publish`、`nsh verify`、换根被拒）没有在 CI 上执行过**，因为没有标签，`shell_changed=true`，只在本机跑过。更新源可见性见 4.4（仓库私有，更新器暂时读不到更新） |
| 3 | 逻辑层：删除 `signature.rs`、`usher-update-tool`、`trusted_keys.txt`，改造状态机（设计见 8.2），更新 crate 测试；另含日志 crate 拆分（8.6） | **进行中**（并行会话已结束；决定选项 B）。以下是此前暂停时的记录：串行顺序（3、4、5 依次做）与 `min_version` 删除已决定。仓库里已有并行工作依赖 `Machine` 的旧接口：`src/update_service/`（应用层协调器）、`usher-state`、`update` 集成夹具，以及未提交的 `diagnostics.rs` 改动；批次 3 会让它们编译不过，所以等其完成后一并对齐。设计已定稿，不需要再确认 |
| 4 | 桥接：`update_bridge.rs`、`[updates]` 设置（仅 `auto_check`）、稳定错误码 | 待办 |
| 5 | 界面：`ui/features/updates/`、布局契约扩展、`Rail.tsx` 状态、文案与界面用例 | 待办 |
| 6 | 文档与验收：同步 architecture、scaffolding、deploy、README，真机验收 | 待办 |

验证限制：`checkForUpdate` 在开发运行下返回 null，且更新源必须是 HTTPS，端到端验证需要打包构建和真实的 GitHub Release；发布流水线从未在 GitHub 上真实运行过，第一次真实发布必须是正式版标签；在此之前只用演练模式验证构建链路。

## 14. 变更记录

| 日期 | 变更 |
| --- | --- |
| 2026-11-12 | 对接通用状态管理设计：通用层独立为 `usher-state`，更新业务状态机保留在 `usher-update`；明确应用作用域、单执行者和 SM1 至 SM5 的实施前置关系，仅更新文档。 |
| 2026-11-12 | 建立本文：记录 D1 至 D12、第 3 至 12 节方案与 V1 至 V7 待验证项。 |
| 2026-11-12 | 新增第 12 节本地更新模拟（D14）：命令、工作方式、11 个场景与限制。4 个场景已在本机通过，7 个标为 `pending`，等批次 4、5 的检查逻辑。 |
| 2026-11-13 | 决定选项 B：删除取消与执行者交接（`update_service` 中的 `Cancel`、`Settlement`、`Recovery`、`revoke`、`settle`、`reconcile` 等），保留单执行者领取；脱敏范围定为只保留凭据类；并行会话已结束，批次 3 恢复。 |
| 2026-11-13 | 批次 3 状态机设计补充：失败分五类并各自对应面板处理与日志级别；删除 `Cancel` 事件（NSH 无取消接口，V12）；新增更新日志设计（状态机产生记录、桥接写出、只做 `$HOME` 替换为 `~`）；新增 8.6 日志 crate 拆分要求。发现并行会话已在 `Machine` 外加了 `src/update_service/`，批次 3 与日志拆分等其结束后实施。 |
| 2026-11-12 | 批次 2 演练在 GitHub 上通过；批次 3 开始，决定串行推进，删除 `min_version`，"外壳过旧"由代码包与外壳的主次版本是否相同表达。 |
| 2026-11-12 | 批次 2 实施：门禁改为只接受正式版并要求 `updater` 完整；新增外壳输入清单与检查、版本线约束、清单检查器、`make-update-files.sh`；`release.yml` 的 `publish` 与 `rehearse` 改用 NSH 原生命令；`rehearsal-sign.sh` 改为同一链路加损坏用例；删除 `collect-release-files.sh`。发现仓库私有，新增 4.4 与 V10。 |
| 2026-11-12 | 确定第一次发布为 `0.1.0`（D13），新增 4.5 说明首次发布的限制与验证办法。 |
| 2026-11-12 | 取消预览通道，只保留正式版（D7、D8）：删除预览清单与滚动 Release、通道状态与按构建生成两份配置的设计，门禁只接受正式版标签。 |
| 2026-11-12 | 第 1 批验证回写：通道改为构建属性（V2），设置页不再提供通道切换，`[updates]` 只含 `auto_check`；代码包路径可行（V3，推翻 06.04 的否定结论）；Sparkle 设想作废（V7）；新增 V8、V9；4.3 补充两个通道对应两种构建，预览用户升到正式版后仍留在预览通道。 |
| 2026-11-13 | SM6 本机续验：更新模拟采用唯一测试 ID / 临时数据根，禁止 HOME 重定向；补齐 16 项隔离清理单测，脚本全量 120 项通过，真实启动模拟 4 项通过 / 7 项 pending；普通结束与 SIGTERM 均核对无隔离残留。 |
