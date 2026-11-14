# Bug History / Bug 历史

[交互设计 / Interaction design](docs/interaction-design.md) · [批次与验收 / Delivery and acceptance](docs/todo.md) · [架构 / Architecture](architecture.md)

采用同级 Slint Usher 的编号、日期、领域、缺陷证据、状态与处理方案表结构。本仓库独立编号；历史失败及未验证范围保留，不能用后续通过覆盖首次失败。详细批次状态与执行证据以 `docs/todo.md` 为准。

| ID | Date | Area | Defect and evidence | Status | Resolution / target |
| --- | --- | --- | --- | --- | --- |
| UI-001 | 2026-11-14 | Shared control interaction / 共享控件交互 | 点击状态栏 Switch 或其他设置操作时，无关的外观、语言、背景、主题及标题栏控件短暂灰显再恢复。`appearance.sync` 发布 saving → `appearance.busy()` → 多个控件共用 disabled → 禁用透明度产生集体闪烁；滑块还在 trackClass 中带 disabled opacity。之前仅日志开关删除灰显样式，未形成公共契约。全项目复核还发现更新页／浮层把 `!canCheck` 中的共享 working 映射为灰显，已改为临时交互锁。 | Fixed; shared regressions and macOS native acceptance passed | 在公共控件层分离业务不可用 `disabled`、临时提交保护 `interactionBlocked`、当前任务加载 `busy`；NSH 保留行为禁用，视觉只由真实不可用／当前任务决定；AppKit 保留 enabled 外观，在目标动作拦截并恢复受控选择。保留按钮重入锁、appearance 请求锁和 Rust 串行提交。公共回归、真实设置接入及 macOS 像素对照接入现有测试入口；全项目复核后验证：Rust 241、UI 47、state-ui 45、脚本 141、macOS 原生 11 项通过；NSH／AppKit 控件及设置内容区域临时锁前后像素差为 0，真实禁用仍灰显。首次沙箱禁止本地监听导致原生启动失败，允许隔离宿主监听后完整重跑通过；详细证据与未测范围见 `docs/todo.md` UI-001。 |

| UI-002 | 2026-11-14 | Slider completion / 滑块完成事件 | UI-001 新增的 ThemeSlider 把 onChangeEnd 也按当前交互锁过滤；当 onChange 获取锁时，同一次已接受操作的完成事件被吞掉。键盘步进回归实际得到 Calls:1、Blocked:true、Ends:0，预期 Ends:1。 | Fixed; headless and macOS native regression passed | 保留新输入的锁检查，透传 NSH 为已接受操作发出的 onChangeEnd，以完成提交和预览清理。新增共享回归在无头与原生入口执行；初始错误的拖动定位未触发控件，修正为四个 Tab 后 Right 才得到有效失败证据。完整复核结果见 docs/todo.md UI-001 / UI-002。 |

| UI-003 | 2026-11-14 | AppKit empty selection / 原生空选择恢复 | 当前受控 ID 无匹配项时，逐段取消 selected / selectItem(nil) 未能清空真实 AppKit 选中；临时锁拒绝后观察到 popup=0、segments=1，预期均为 -1。完整原生补验 10 通过／1 失败。 | Fixed; complete macOS native regression passed | 在受控投影和目标动作回退中统一使用 AppKit 的空选择索引 -1，且空选中时不调用会自动选回第一项的 synchronizeTitleAndSelectedItem；补充不存在 ID、业务禁用下直接动作及解除后回调的原生回归，结果见 docs/todo.md UI-003。 |

| UI-004 | 2026-11-14 | Reactive prop forwarding / 控件属性冻结 | 用户反馈状态栏 Switch 首次切换后无法切回，窗口仍可操作。UI-001 的 ValueControls 用对象 rest 解构复制 props，求值并丢失 checked getter，底层 Switch 持续按初始值生成相同的变更；Slider、Select 和 AsyncButton 转发也有同源风险。同页 Gallery 连续切换回归第二次等待状态栏显示超时，额外等待 300ms 仍失败。 | Fixed; repeated-input and full-application macOS acceptance passed | 公共 forwardProps 复制属性描述符并排除内部字段，保留响应式 getter；修复全部四处快照转发。新增同挂载实例的受控值／标签回归及真正 ProductionApp 的连续四次 Gallery 状态栏切换。此前页面夹具固定值和跨页重挂载未覆盖此缺陷，不能以其通过证明连续操作正常。修复后完整 UI 49、state-ui 45、脚本 141、macOS 原生 13 项通过；完整 App 同页四次 Gallery 切换正常，像素防闪烁验收保留。 |

## UI-001 快速排查 / Troubleshooting

1. 先确认变化是否来自用户实际切换主题、选中值、焦点或悬停；这些正常变化不属于本缺陷。
2. 搜索 `disabled={busy()}`、`disabled={appearance.busy()}`、`enabled={!busy()}` 、派生的 `working` / `canCheck` 及部件的 `disabled:opacity`，检查是否把共享请求状态错误映射为视觉不可用。
3. 临时互斥改用公共控件的 `interactionBlocked`；能力缺失、无有效输入等真实前置条件仍用 `disabled`。不要全局删除禁用样式，不要让每个页面自行隐藏透明度，也不要把全局 busy 传成每个按钮的 loading。
4. 查看 `ui/components/interaction.ts`、`AsyncButton.tsx`、`ThemeControls.tsx`、`ValueControls.tsx` 与 `platforms/macos/controls.rs`。确认业务入口仍有串行／重入保护，AppKit 被拒绝的选择能够恢复。
5. 在仓库根目录执行 `nsh check`、`python3 scripts/test.py ui` 和 `python3 scripts/native_button_acceptance.py --junit target/test-reports/interaction-native.xml`。原生命令必须在解锁的 macOS 图形会话串行运行，脚本创建隔离源码与数据目录；查看生成的截图目录及 `pixel-comparison.json`，不能仅凭 enabled 断言判定视觉修复。

解决方案契约详见 [公共控件的可用性与临时交互锁](docs/interaction-design.md#公共控件的可用性与临时交互锁)。Windows、Linux、触屏与屏幕阅读器不在本次 macOS 验收结论内。

## UI-002 快速排查 / Troubleshooting

若滑块已发生值变化但提交／预览清理未执行，先核对 `onChange` 与 `onChangeEnd` 的计数，再检查完成事件是否被新获得的交互锁吞掉。完成通知不等于开始新操作；应保留框架的手势完成语义，而不是放开所有输入或绕过业务提交锁。回归入口为 `tests/ui/support/interaction-journeys.ts` 的 accepted gesture 用例。

## UI-004 快速排查 / Troubleshooting

首次操作有效、第二次重复旧值但窗口仍响应时，先检查公共组件是否用 `{ ...rest }`、普通展开或 `Object.assign` 转发响应式 props。分别观察 Store 当前值和底层控件生成的下一值；检查 checked / value / items / accessibilityLabel 的 getter 是否仍在。用 `forwardProps` 保留属性描述符，禁止通过增加等待、重新挂载或放开业务锁掩盖问题。回归为 `tests/ui/statusbar.test.tsx` 和 shared controls retain reactive values 用例。
