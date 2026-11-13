# 部署与发布

构建、签名和发布由两个 GitHub Actions 工作流完成：[ci.yml](../.github/workflows/ci.yml) 检查代码，[release.yml](../.github/workflows/release.yml) 构建、签名并发布。两者使用同一套固定版本的 Rust 工具链和 NSH 命令行，由 [scripts/ci/](../scripts/ci) 里的脚本安装与执行，这些脚本在本机都跑过。

## 两种启动方式

`release.yml` 只有两种启动方式，没有分支触发，没有定时任务。更新行为见 [更新设计](update.md)，本文规定发布管线与操作契约：

| 方式 | 是什么 | 做什么 |
| --- | --- | --- |
| 推送新的 `v<主>.<次>.<补丁>` 标签 | **发布** | 构建、用真正的更新密钥签名、发布到 GitHub Release。**只接受正式版**，带连字符的标签会被拒绝 |
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
| `gate` | 检查三处版本一致、构建号有效、`nsh.json` 的 `updater` 完整，运行脚本测试与外壳输入清单检查，对比上一个正式版判断外壳是否变化，解析 macOS 签名开关；在运行摘要里写明这是演练 |
| `verify` | 格式、Clippy、全部 Rust 测试、`nsh check`、界面测试 |
| `build-macos` | 用 `nsh package` 打 `standard` 和 `appkit` 两个变体，核对包内版本、构建号和应用 ID |
| `rehearse` | 用**临时生成的三把密钥**走和 `publish` 同一套脚本：产出代码包（外壳未变时）、清单与签名，用同一个检查器验证；再故意弄坏几份拷贝（换归档、去掉签名、与外壳变化结论不符、换根验证代码包、改代码包一个字节），确认都会被拒绝 |
| `publish` | **不运行** |

演练**一定不会**做这些事：读取真正的更新密钥 `NSH_MANIFEST_KEY`、`NSH_RELEASE_KEY`、`NSH_RELEASE_CERT`、写入 GitHub Release、生成构建来源证明、做 Apple 签名与公证。`rehearse` 作业没有任何 Secret、没有写权限、没有绑定 `release` 环境；这些在工作流里是结构上保证的，不是靠约定。演练也有自己的并发分组，不会拖慢或取消正在进行的发布。

### 怎么看结果

- **运行摘要**（Run 页面最上面）会写明“这是演练”，并回答第二个问题：**“用这个版本打标签会被接受”或“会被拒绝”**，拒绝的原因在 `gate` 作业的警告里。
- 构建好的文件在页面底部的 **Artifacts**：`macos-standard`、`macos-appkit` 是两个变体，`rehearsal-<版本>` 是用临时密钥签过名的整套更新文件（整包归档、可能有的 `.nshpkg`、清单与签名），保留 3 天，可以下载下来检查。
- 全部绿色，加上 `rehearse` 最后一行 `Rehearsal signing passed.`，说明构建与签名链路是通的。

### 演练里哪些是警告、哪些会失败

演练的目的是让你随时能测，所以只有会让构建本身失去意义的问题才会让它失败，只对发布有意义的问题改成警告，并在摘要里标出“标签会被拒绝”：

| 问题 | 演练 | 真正的发布 |
| --- | --- | --- |
| `nsh.json` 与 `Cargo.toml` 的版本不一致 | 失败 | 失败 |
| 构建号不是正整数 | 失败 | 失败 |
| 更新日志里没有这个版本的小节 | 警告，使用占位说明 | 失败 |
| 版本带预发布后缀（如 `0.1.0-alpha.1`） | 警告 | 失败 |
| `nsh.json` 缺少完整的 `updater`（`github` 指向本仓库、`publicKey`、`packages.roots`），或设置了 `updater.url` | 警告 | 失败 |
| 外壳输入有改动，但主版本和次版本没有升高 | 警告 | 失败 |
| 构建号不比上一个发布高 | 警告 | 失败 |
| 标签不是新建的、提交不在默认分支上 | 不适用（没有标签） | 失败 |

### 演练证明不了什么

- **真正的更新密钥是否配置正确。** 演练用临时密钥，三个 Secret 没被读取，所以它们没设置或设错了，演练照样是绿的，只有真发布才会暴露。**只有正式版，没有预发布可以先试**，第一次发布 `0.1.1` 就是真实发布，且标签不可移动，见 [更新方案 4.5](update.md#45-首次发布d13)。发布前请逐项核对下文“发布前核对”。
- **发布步骤本身**：`release` 环境、草稿转正式、构建来源证明。
- **Apple 签名与公证。** 演练跳过它们。
- **标签规则本身**（新建、在默认分支上）。演练没有标签，这部分只有真实推送才检查。

## 提交前检查

只要有人改了 Rust 文件却没格式化就提交，CI 的第一个检查（`cargo fmt --check`）就会失败。仓库用 [lefthook](https://lefthook.dev) 在提交产生之前就处理它，配置在 [lefthook.yml](../lefthook.yml)。**每个克隆装一次**：

```bash
brew install lefthook
lefthook install
```

`lefthook install` 会写入 `.git/hooks/pre-commit`，此后每次 `git commit` 先运行下面的检查。顺序是固定的，前一步的修复会被后一步看到：

| 顺序 | 检查 | 对象 | 做什么 |
| --- | --- | --- | --- |
| 1 | `format-rust` | 暂存的 `.rs` | **自动格式化**，并把结果重新加入暂存（`stage_fixed`），所以提交里永远是格式化后的版本 |
| 2 | `shellcheck` | 暂存的 `.sh` | 不通过就拒绝提交；没装 `shellcheck` 会打印一行“已跳过”，CI 会再检查 |
| 3 | `actionlint` | 暂存的工作流 | 工作流及 `.github/actionlint.yaml` / `.github/actionlint.yml` 配置均读取暂存区版本，包含自托管 runner 标签；没装则同样打印跳过 |
| 4 | `validate-lefthook-config` | `lefthook.yml` | 配置写坏了就拒绝提交 |
| 5 | `no-key-files` | 暂存的 `*.secret`、`*.pem`、`*.p12`、`*.key`、`*.pfx` | 拒绝提交 |
| 5 | `no-private-key-block` | 所有暂存的改动 | 发现私钥块就拒绝提交 |

**只暂存了一个文件的一部分时**（`git add -p`）：没暂存的改动不会被带进提交，也不会从工作区消失；格式化只作用在暂存的那部分。如果你先暂存了没格式化的版本、之后又在工作区手改成了格式化的版本，lefthook 没法把两者合并，会报 `Unable to restore previously hidden unstaged changes` 并拒绝这次提交，**你的工作区和暂存区都不会丢**，先 `git add` 一次再提交即可。

**格式化为什么读标准输入**：`rustfmt 文件路径` 会顺着 `mod x;` 去改旁边的文件，包括你没暂存的。所以 [format-rust.sh](../scripts/git-hooks/format-rust.sh) 一次只处理一个文件，从标准输入读、成功后才写回；语法错误的文件原样保留并拒绝提交。

紧急情况可以 `git commit --no-verify` 跳过，但 CI 做同样的检查。

### lefthook 保证不了的两件事（实测过）

1. **没装 lefthook，就没有任何检查。** 此时提交只打印一行 `Can't find lefthook in PATH`，照样成功。新克隆忘了装，保护就不存在，也没有提示。
2. **`lefthook.yml` 在 `lefthook install` 之前就是坏的，不会写入钩子，之后的提交完全不被检查，也没有提示。** 已经装好之后再把它改坏，则提交会被拒绝（第 4 项检查）。

所以 CI 里单独有一步 `lefthook validate`，并且运行整套钩子测试 [test_git_hooks.sh](../scripts/tests/test_git_hooks.sh)：它在临时仓库里真的提交一遍，覆盖自动格式化、部分暂存、语法错误、文件名带空格、`mod` 不会被牵连、密钥、工作流、自托管 runner 配置的暂存区隔离、`shellcheck` 和坏配置。早期用例曾用 7 种故意弄坏的实现检验，每一种都会让对应的用例失败。

## 发一个版本

1. 在 `nsh.json` 和根 `Cargo.toml` 里把版本改成同一个值，例如 `0.1.0`。**外壳输入有改动（Rust、原生适配、编进二进制的主题与配置、`nsh.json` 的权限等，范围见 [shell-inputs.txt](../scripts/release/shell-inputs.txt)）时，必须升主版本或次版本；只改 TSX 才能升补丁版本。** 门禁会检查，真实发布不满足就失败。
2. 把 `nsh.json` 的 `build` 加一。构建号必须比上一个已发布的高，客户端会拒绝更低的构建。
   同步 `Cargo.lock` 中根包的版本。仅根包发布版本号变化不计为外壳变更，依赖或其他 Cargo 配置变化仍要求升主版本或次版本。
3. 在 [CHANGELOG.md](../CHANGELOG.md) 里写一个 `## 0.1.0` 小节，内容就是发布说明。小节为空会被拒绝。
4. 合并到 `main`，等 CI 通过，**再手动跑一次演练**，看摘要是否写着“会被接受”。
5. 在 `main` 上的那个提交打标签并推送：

```bash
git tag v0.1.0
git push origin v0.1.0
```

只有正式版：带连字符的标签（`v0.1.0-alpha.1`）会被门禁拒绝，不构建，不发布。Release 不带 `--prerelease`，并显式标为 latest，因为更新器读取的是 `releases/latest/download/latest.json`。

标签推送会启动工作流，但 `gate` 会先检查，不满足就在几秒内结束，不会构建：事件必须是标签推送；标签必须是新建的（不是更新、强制推送、删除）；标签指向的提交必须已经在默认分支上（标签打在 `main` 之后又有新提交没有关系）；检出的提交就是标签指向的提交。

## 会构建哪些平台

目前只有 **macOS Apple Silicon（aarch64）**。一次发布产出：

| 文件 | 内容 |
| --- | --- |
| `Usher-NSH-<版本>-macos-aarch64.tar.gz` | **整包更新归档**，由 `nsh package` 生成的 `standard` 变体，文件名由 NSH 决定，更新器只读它。归档内容在 Apple 签名之后重新打包，保证装的是最终的 `.app` |
| `<应用 ID>-<构建号>.nshpkg` | **代码包**，由发布密钥签名，证书随包分发。只有外壳输入没变的版本才有，外壳变了的版本只发整包 |
| `latest.json`、`latest.json.sig` | 更新清单与清单密钥的签名。更新器读取它们，二者缺一不可 |
| `Usher-aarch64-apple-darwin-standard.zip`、`Usher-aarch64-apple-darwin-appkit.zip` | 给人手动下载的两个变体，更新器不读。`appkit` 变体在 macOS 26 及以上额外提供原生 AppKit 控件 |
| `build.txt` | 本次的构建号，下一次发布的门禁用它拒绝更低的构建 |

没有构建：Intel Mac、Windows、Linux、Android、iOS。NSH 的 macOS 运行时只有 arm64，`nsh package` 也没有架构选项，所以做不出 Intel Mac 版本；Windows 和 Linux 的界面与更新路径还没有验证过。要加平台，需要在 `build-macos` 之外新增作业，并更新清单检查器 [check_update_files.py](../scripts/release/check_update_files.py) 里的平台键。

## 更新签名密钥

NSH 原生体系有三类密钥，设计见 [更新方案 4.1](update.md#41-密钥与信任)：

| 材料 | 用途 | 保管 |
| --- | --- | --- |
| 清单密钥 | 签署 `latest.json`，公钥写在 `nsh.json` 的 `updater.publicKey` | `release` 环境的 Secret，另有离线备份 |
| 根密钥 | 为发布密钥签发证书、签发吊销列表，公钥写在 `updater.packages.roots` | **离线**，不进仓库、不进 CI |
| 发布密钥与证书 | 签署 `.nshpkg`，证书随包分发 | `release` 环境的 Secret |

公钥进仓库，私钥永不进仓库（`.gitignore` 忽略 `*.secret`，提交前检查拒绝密钥文件）。本机私钥放在 `~/.usher-nsh-update-signing/`。

**证书有有效期**，签发时由 `--days` 决定（当前签发的是 365 天，到 2027-11-12）。过期后新发布的代码包会被客户端拒绝，所以到期前必须重新签发并更新 `NSH_RELEASE_CERT`，把到期日记进发布检查清单。

### 发布密钥泄露处置

1. 停止发布，确认泄露的密钥 ID 和影响范围，隔离可能被修改的产物。
2. 用离线的根密钥生成吊销列表：`nsh revoke --root root.secret --key <泄露的发布密钥 ID> --out revocations`，列表的 `sequence` 必须高于以往任何一份。
3. 生成新的发布密钥，用根密钥重新签发证书（`nsh keygen`、`nsh certify`），更新 `NSH_RELEASE_KEY` 与 `NSH_RELEASE_CERT`。已安装的应用不需要更新，因为证书随包分发，信任的是根。
4. 清单密钥泄露时，应用内嵌的是它的公钥，必须发布一个内置新公钥的**整包**版本，旧安装需要手动安装一次，这是 NSH 体系下清单密钥无法在线轮换的限制。
5. 根密钥泄露是最严重的情况：已安装的应用只信任旧根，只能通过可信渠道重新分发内置新根公钥的版本。

发布侧 `nsh verify --revocations` 与客户端信任库是不同边界；仅执行发布侧检查不改变已安装客户端的信任状态。客户端吊销列表的接入与分发由 [REL3](todo.md#release) 跟踪。

## 需要配置的 Secrets 和变量

| 名称 | 类型 | 用途 |
| --- | --- | --- |
| `NSH_MANIFEST_KEY` | Secret | 清单密钥的私钥文件内容（`manifest.secret`），发布必须。取值：`tr -d '\n' < ~/.usher-nsh-update-signing/manifest.secret \| pbcopy`，不会打印私钥。演练不读取它 |
| `NSH_RELEASE_KEY` | Secret | 发布密钥的私钥文件内容（`release.secret`），签署代码包。取值方式同上 |
| `NSH_RELEASE_CERT` | Secret | 发布密钥的证书文件内容（`release.cert`），不是机密，但随私钥一起管理方便轮换。取值：`pbcopy < ~/.usher-nsh-update-signing/release.cert` |
| `MACOS_SIGNING_NOTARIZATION_ENABLED` | Variable | 是否做 Apple 签名与公证。不设或 `false` 为否，`true` 为是；其他值（含大小写变体和空白）在构建前就失败 |
| `MACOS_CERTIFICATE_BASE64`、`MACOS_CERTIFICATE_PASSWORD`、`MACOS_KEYCHAIN_PASSWORD`、`APPLE_SIGNING_IDENTITY`、`APPLE_ID`、`APPLE_TEAM_ID`、`APPLE_APP_PASSWORD` | Secret | 只在开启 Apple 签名时需要 |

此前配置记录中，三个 Secret 为**仓库级** Secret，正式发布前需复核。`publish` 作业绑定的 `release` 环境读得到它们，但把它们改放到环境级 Secret 范围更小，建议这样做。

另外要在仓库设置里建一个名为 `release` 的 Environment，可以在里面加人工审批，这样每次发布前都要有人点确认。演练不受它影响。

## 现在没有 Apple 开发者证书

当前没有 Developer ID，所以 `MACOS_SIGNING_NOTARIZATION_ENABLED` 不设置，发布走未签名路径：

- 应用仍然是 `nsh package` 生成的临时签名（ad hoc），包内自洽，但 Gatekeeper 不认。
- 构建日志、作业摘要和发布说明开头都会写明“未签名、未公证，Gatekeeper 可能阻止打开”，并给出打开的办法（右键打开，或 `xattr -dr com.apple.quarantine`）。
- 更新清单与代码包的签名不受影响，始终必须。
- Apple 签名、公证、钥匙串三组步骤已经写好并通过 lint，用一张自签名证书在本机演练过钥匙串的安装、`codesign` 和清理；公证和 `stapler` 需要真实的开发者账号，**没有演练过**。拿到证书后把变量设为 `true`、补上 Secrets 即可启用，不需要改代码。因为演练会跳过 Apple 签名，第一次启用只能在真实发布里验证，所以启用前请先在本机用自签名证书走一遍。

## 发布信任与构建边界

更新器匿名读取 GitHub latest 的清单与签名，更新源必须允许匿名下载。正式发布前的密钥配对、环境配置及资源可读性核验统一由 [发布批次](todo.md#release) 跟踪。

NSH CLI 版本由 Cargo.toml 中 nsh-core 的精确版本提供，install-nsh.sh 校验上游同目录 SHA-256 和安装后的版本。此校验可发现损坏或装错版本，不能防止上游同时替换归档与校验文件。GitHub Actions 固定提交哈希，Ubuntu runner 固定 ubuntu-24.04。

Rust、桥接与日志 crate 属于外壳输入，不能作为仅 TSX 代码包发布。清单按 OS / 架构收录 standard 整包，AppKit zip 用于手动下载。NSH 0.5.0 的整包替换不保证 rename 间隙自动恢复，也不验证签名有效归档内可执行文件的架构；签名校验与这些安装保护是不同边界。ad hoc 签名不建立 Gatekeeper 信任。修复与实际分发验收见 [更新批次](todo.md#updates)。
