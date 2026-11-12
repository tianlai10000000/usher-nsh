# 部署与发布

构建、签名和发布由两个 GitHub Actions 工作流完成：[ci.yml](../.github/workflows/ci.yml) 检查代码，[release.yml](../.github/workflows/release.yml) 构建、签名并发布。两者使用同一套固定版本的 Rust 工具链和 NSH 命令行，由 [scripts/ci/](../scripts/ci/) 里的脚本安装与执行，这些脚本在本机都跑过。

## 两种启动方式

`release.yml` 只有两种启动方式，没有分支触发，没有定时任务：

| 方式 | 是什么 | 做什么 |
| --- | --- | --- |
| 推送新的 `v<语义化版本>` 标签 | **发布** | 构建、用真正的更新密钥签名、发布到 GitHub Release |
| 在 Actions 页手动运行 Release | **演练** | 同一条构建链路，随时可以测，**不用真密钥、不发布** |

[ci.yml](../.github/workflows/ci.yml) 另外在 `main` 推送和 PR 时运行，只检查（格式、Clippy、测试、lint），不打包、不签名、不发布。

## 演练模式：随时测试 CD 是否正常

演练用来回答两个问题：构建链路现在能不能走通，以及现在打一个标签会不会被接受。它不需要任何改动，也不会留下任何发布。

### 怎么跑

1. 打开仓库的 **Actions** 页，左侧选 **Release**。
2. 右上角点 **Run workflow**，在 **Use workflow from** 里选要构建的分支（通常是 `main`，也可以是还没合并的功能分支）。
3. 点绿色的 **Run workflow**。没有任何输入项。

或者用命令行（需要安装并登录 `gh`）：

```bash
gh workflow run release.yml --ref main
gh run watch
```

### 它会做什么

演练构建的是你选的那个分支的最新提交，版本取自 `nsh.json`，不需要标签。作业按这个顺序运行：

| 作业 | 演练里做什么 |
| --- | --- |
| `gate` | 检查三处版本一致、构建号有效，运行脚本测试，解析 macOS 签名开关；在运行摘要里写明这是演练 |
| `verify` | 格式、Clippy、全部 Rust 测试、`nsh check`、界面测试 |
| `build-macos` | 用 `nsh package` 打 `standard` 和 `appkit` 两个变体，核对包内版本、构建号和应用 ID |
| `rehearse` | 用一把**临时生成的密钥**给清单签名，并按客户端的规则验证；再故意弄坏几份拷贝，确认验证会拒绝它们 |
| `publish` | **不运行** |

演练**一定不会**做这些事：读取真正的更新密钥 `UPDATE_SIGNING_KEY`、写入 GitHub Release、生成构建来源证明、做 Apple 签名与公证。`rehearse` 作业没有任何 Secret、没有写权限、没有绑定 `release` 环境；这些在工作流里是结构上保证的，不是靠约定。演练也有自己的并发分组，不会拖慢或取消正在进行的发布。

### 怎么看结果

- **运行摘要**（Run 页面最上面）会写明“这是演练”，并回答第二个问题：**“用这个版本打标签会被接受”或“会被拒绝”**，拒绝的原因在 `gate` 作业的警告里。
- 构建好的文件在页面底部的 **Artifacts**：`macos-standard`、`macos-appkit` 是两个变体，`rehearsal-<版本>` 是用临时密钥签过名的整套文件，保留 3 天，可以下载下来检查。
- 全部绿色，加上 `rehearse` 最后一行 `Rehearsal signing passed.`，说明构建与签名链路是通的。

### 演练里哪些是警告、哪些会失败

演练的目的是让你随时能测，所以只有会让构建本身失去意义的问题才会让它失败，只对发布有意义的问题改成警告，并在摘要里标出“标签会被拒绝”：

| 问题 | 演练 | 真正的发布 |
| --- | --- | --- |
| `nsh.json` 与 `Cargo.toml` 的版本不一致 | 失败 | 失败 |
| 构建号不是正整数 | 失败 | 失败 |
| 更新日志里没有这个版本的小节 | 警告，使用占位说明 | 失败 |
| 构建号不比上一个发布高 | 警告 | 失败 |
| 标签不是新建的、提交不在默认分支上 | 不适用（没有标签） | 失败 |

### 演练证明不了什么

- **真正的更新密钥是否配置正确。** 演练用临时密钥，`UPDATE_SIGNING_KEY` 没被读取，所以它没设置或设错了，演练照样是绿的，只有真发布才会暴露。第一次发布请用预发布标签（如 `v0.0.1-alpha.2`）。
- **发布步骤本身**：`release` 环境、草稿转正式、构建来源证明。
- **Apple 签名与公证。** 演练跳过它们。
- **标签规则本身**（新建、在默认分支上）。演练没有标签，这部分只有真实推送才检查。

## 发一个版本

1. 在 `nsh.json` 和根 `Cargo.toml` 里把版本改成同一个值，例如 `0.1.0`。
2. 把 `nsh.json` 的 `build` 加一。构建号必须比上一个已发布的高，客户端会拒绝更低的构建。
3. 在 [CHANGELOG.md](../CHANGELOG.md) 里写一个 `## 0.1.0` 小节，内容就是发布说明。小节为空会被拒绝。
4. 合并到 `main`，等 CI 通过，**再手动跑一次演练**，看摘要是否写着“会被接受”。
5. 在 `main` 上的那个提交打标签并推送：

```bash
git tag v0.1.0
git push origin v0.1.0
```

带连字符的版本（`v0.1.0-alpha.1`）自动进入预览通道并标记为预发布；其余进入稳定通道。

标签推送会启动工作流，但 `gate` 会先检查，不满足就在几秒内结束，不会构建：事件必须是标签推送；标签必须是新建的（不是更新、强制推送、删除）；标签指向的提交必须已经在默认分支上（标签打在 `main` 之后又有新提交没有关系）；检出的提交就是标签指向的提交。

## 会构建哪些平台

目前只有 **macOS Apple Silicon（aarch64）**。一次发布产出：

| 文件 | 内容 |
| --- | --- |
| `Usher-aarch64-apple-darwin.zip` | 更新包，由 `standard` 变体复制而来，更新器按这个精确名称选包 |
| `Usher-aarch64-apple-darwin-standard.zip` | `standard` 变体：适用于所有支持的 macOS 版本，使用 NSH 控件 |
| `Usher-aarch64-apple-darwin-appkit.zip` | `appkit` 变体：在 macOS 26 及以上额外提供原生 AppKit 控件 |
| `SHA256SUMS`、`SHA256SUMS.sig` | 已签名的清单与签名，客户端据此验证上面的包 |
| `build.txt` | 本次的构建号，下一次发布的门禁用它拒绝更低的构建 |

没有构建：Intel Mac、Windows、Linux、Android、iOS。NSH 的 macOS 运行时只有 arm64，`nsh package` 也没有架构选项，所以做不出 Intel Mac 版本；Windows 和 Linux 的界面与更新路径还没有验证过。要加平台，需要在 `build-macos` 之外新增作业，并更新签名清单里要求的文件名。

## 更新签名密钥

发布私钥保存在仓库之外，恢复私钥离线保存。客户端的可信公钥集维护在 [trusted_keys.txt](../crates/usher-update/trusted_keys.txt)，发布与恢复角色的权限见[发布信任边界](../architecture.md#发布信任边界)。新增公钥必须随应用版本分发，已安装客户端使用其内置的可信集合。

### 发布密钥泄露处置

1. 停止发布，确认泄露的密钥 ID 和影响范围，隔离可能被修改的产物。
2. 在可信环境中使用离线恢复密钥签发新版本清单，通过 `--revoke-key` 声明吊销泄露的密钥。发布工具的入口为 `usher-update-tool sign DIR --version V --key RECOVERY_KEY_FILE --revoke-key KEY_ID`。
3. 需要更换发布密钥时，通过 `keygen --add-to-keyset` 将新公钥加入内置集合，随恢复版本交付。该版本由旧客户端已经信任的恢复密钥签发。
4. 验证发布清单和产物后发布恢复版本，将流水线的 `UPDATE_SIGNING_KEY` 替换为新发布私钥，并将恢复私钥移回离线存储。

当前应用尚未接入吊销集合持久化和安装前复验。接入后，客户端只有收到并保存已验证的吊销信息，才会拒绝对应密钥；离线客户端在接收前仍持有旧信任状态。恢复密钥同时泄露时，需要通过可信分发渠道交付重新建立信任的版本。

## 需要配置的 Secrets 和变量

| 名称 | 类型 | 用途 |
| --- | --- | --- |
| `UPDATE_SIGNING_KEY` | Secret | 更新清单的 Ed25519 私钥，发布必须。取值：`tr -d '\n' < ~/.usher-nsh-update-signing/update-signing.secret \| pbcopy`，不会打印私钥。演练不读取它 |
| `MACOS_SIGNING_NOTARIZATION_ENABLED` | Variable | 是否做 Apple 签名与公证。不设或 `false` 为否，`true` 为是；其他值（含大小写变体和空白）在构建前就失败 |
| `MACOS_CERTIFICATE_BASE64`、`MACOS_CERTIFICATE_PASSWORD`、`MACOS_KEYCHAIN_PASSWORD`、`APPLE_SIGNING_IDENTITY`、`APPLE_ID`、`APPLE_TEAM_ID`、`APPLE_APP_PASSWORD` | Secret | 只在开启 Apple 签名时需要 |

另外要在仓库设置里建一个名为 `release` 的 Environment，可以在里面加人工审批，这样每次发布前都要有人点确认。演练不受它影响。

## 现在没有 Apple 开发者证书

当前没有 Developer ID，所以 `MACOS_SIGNING_NOTARIZATION_ENABLED` 不设置，发布走未签名路径：

- 应用仍然是 `nsh package` 生成的临时签名（ad hoc），包内自洽，但 Gatekeeper 不认。
- 构建日志、作业摘要和发布说明开头都会写明“未签名、未公证，Gatekeeper 可能阻止打开”，并给出打开的办法（右键打开，或 `xattr -dr com.apple.quarantine`）。
- 更新清单的签名不受影响，始终必须。
- Apple 签名、公证、钥匙串三组步骤已经写好并通过 lint，用一张自签名证书在本机演练过钥匙串的安装、`codesign` 和清理；公证和 `stapler` 需要真实的开发者账号，**没有演练过**。拿到证书后把变量设为 `true`、补上 Secrets 即可启用，不需要改代码。因为演练会跳过 Apple 签名，第一次启用请用预发布标签试，不要直接发稳定版。

## 已知限制

- 整包更新的下载、安装交接和客户端检查还没接进应用，所以这些发布文件现在只能手动安装。
- 发布依赖的 GitHub 动作都固定到提交哈希，NSH 命令行包固定了 SHA-256。升级它们要手动改工作流里的值。
- 整条流水线在本机逐段跑过（门禁、`nsh package`、归档、收集、临时密钥签名与拒绝测试），**但从未在 GitHub 上运行过**。第一次演练很可能会暴露一两个本机验证不到的问题：权限、`release` 环境、Secrets 的读取、`macos-latest` 上的具体行为。
