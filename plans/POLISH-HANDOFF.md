## Appendix — OCR / 滚动 / 恢复默认 / 浅色提示（2026-09-06）

跟 `design/INTERACTION-P1.md` + `OCR-SCROLL-SPEC.md` v1.1。浅色+透明=内联 tip，不强制降级 Clear。
细节与「仍需 Windows」见 `plans/BOX-DONE-20260906.md`。

---

# 网速 + 设置页抛光交接（2026-09-06，box 源码）

> Debian/box 上写完代码与单测。**没有** .NET / 不能编 WPF，**不能**声称设置页或任务栏槽已绿。等 tsumon（Windows）开机后编跑再点检。
>
> **2026-09-06 设置页视觉抛光 pass 2（box）已做完**——节奏 / token 样式 / 浅深色铬 / About 文案。
> **2026-09-06 设置交互抛光（自动应用）已做完**——改即存盘 / 滑块 200ms / `已保存` / 级联禁用 / `立即保存`。仍 **awaiting boot** 真机点检。

## 改了什么（大白话）

### 设置页
- 五个分组清楚了：**常规 / 截图 / 网速 / 任务栏效果 / 关于**，组间有空和发丝。
- 标题不再写 `HKCU\...\Run`；主题按钮改成「**切换系统深浅色**」，放在常规组（托盘仍是主入口）。
- **网速组**补上 TrafficMonitor 常用外观（自写，没抄源码）：
  - 字号滑块 10–18（只改任务栏槽；悬浮仍 13）
  - 显示下行 / 显示上行
  - 下行颜色 / 上行颜色（色块 + 选颜色 + 默认）
  - 背景透明；关掉后才能选背景颜色
  - 可选：显示边框、双行、加粗
  - 网卡、钉在任务栏（原来就有）
- 上、下行：禁止关掉最后一行（弹回 +「至少显示上行或下行。」），不写坏组合。
- **改完即应用并自动存盘**（Joe 覆盖原 apply-only）：勾选/下拉/失焦/选色立刻；字号与任务栏透明度滑块防抖 ~200ms。底栏「立即保存」冲刷防抖；关闭无丢弃警告。
- 级联禁用：关 PNG 保存→目录；关显示网速→其余网速块；背景透明→背景色；实色/Normal→透明度；关任务栏效果→档位/色/透明度。

### 任务栏网速槽
- 对照反例 `design/joe-netspeed-ugly.png`：不再写死 11px + 高 40 + 顶对齐。
- 默认字号 **13**；高度跟任务栏客户区；速率块用公式垂直居中（+1 DIP 光学下移）。
- 默认 **无框**；背景默认透明。
- 布局纯函数 `TaskbarSlotChrome` 可在 Linux 上单测。

### 设置存盘
- `NetSpeedSettings` 新字段；**未 bump schema**（仍为 3）。旧 JSON 缺字段 = 用默认。
- 反序列化会 `Normalize()`：字号夹到 10–18；两行都关则恢复都开。

## Joe 开机后点什么

1. 同步 box 源码到 `C:\Users\Administrator\suite-app-fresh\windows-suite-app\`
2. 编：
   ```bat
   cd C:\Users\Administrator\suite-app-fresh\windows-suite-app
   dotnet build src\Suite.App\Suite.App.csproj -c Debug -p:Platform=x64
   dotnet test tests\Suite.Contracts.Tests\Suite.Contracts.Tests.csproj -c Debug -p:Platform=x64
   dotnet test tests\Suite.NetSpeed.Tests\Suite.NetSpeed.Tests.csproj -c Debug -p:Platform=x64
   ```
3. 杀旧进程再开 Suite.exe（输出目录同以前）。
4. 肉眼：
   - 任务栏右侧附近 ↑↓，字更大，**不要**再顶对齐留底空；默认无灰框。
   - 设置五个组一眼能数出来。
   - 改字号/颜色 → 片刻后槽跟着变，底栏「已保存」；「立即保存」可立刻冲刷。
   - 只勾下行：槽里一行 ↓，仍居中。
   - 关掉「钉在任务栏」：桌面悬浮，无失败气泡。

## 关键文件

| 文件 | 作用 |
| --- | --- |
| `src/Suite.Contracts/NetSpeedSettings.cs` | 外观字段 + Normalize |
| `src/Suite.Contracts/SettingsJson.cs` | 反序列化 Normalize |
| `src/Suite.Contracts/NetSpeedCopy.cs` | 至少一行 / 字号帮助文案 |
| `src/Suite.NetSpeed/TaskbarSlotChrome.cs` | 槽垂直居中纯函数 |
| `src/Suite.Platform/TaskbarEmbed.cs` | `TryGetTrayClientSize` |
| `src/Suite.App/NetSpeedWindow.xaml(.cs)` | 槽/悬浮外观 |
| `src/Suite.App/SettingsWindow.xaml(.cs)` | 分组 + 外观控件 + 自动应用/级联 |
| `src/Suite.App/AppController.cs` | ApplyAppearance |
| `tests/.../SettingsJsonTests.cs` | 默认 / 往返 / 缺字段 |
| `tests/.../TaskbarSlotChromeTests.cs` | 居中公式 |

## 残余风险

- box 无 Windows / 无 dotnet：**UI 未真机验证**。
- Win11 居中任务栏 / 小组件仍可能挤掉槽 → 应回退悬浮并说人话（原逻辑保留）。
- 截图/钉图路径本轮未改；若回归请对照 F1 工具栏。

---

## Appendix A — Settings visual polish pass 2（2026-09-06，box）

**状态：** 源码已改；**不能在 box 上跑 WPF**。等 Windows 开机编跑再点检。未改截图工具栏 / 钉图彩虹 / TrafficMonitor。

### Diff summary

| 文件 | 改动 |
| --- | --- |
| `src/Suite.App/SettingsWindow.xaml` | `Window.Resources`：`SectionTitle` / `HelpText` / `FieldLabel`(72) / `PrimaryButton`(88) / `Hairline` / `ColorSwatch`(18²、圆角 2)。页边 16。组间发丝居中于 24px（`Margin 0,12,0,12`），关于后无发丝。底栏：状态在上、应用/关闭右对齐。About 改为 toolbar-first。仍用系统 CheckBox/ComboBox/Slider/Button/TextBox，无 GroupBox/卡片/侧栏。 |
| `src/Suite.App/SettingsWindow.xaml.cs` | `ApplyChrome` / `ApplyTheme` / `ApplyChromeFromSystem`：读 `PersonalizeRegistry`（AppsUseLightTheme）翻 `surface-*` / `ink-*` / `settings-muted` / `line-*`。加载时上色；本窗「切换系统深浅色」后刷新；默认色块随主题 ink/surface。控件名与 ApplyAppearance 接线未动。 |
| `src/Suite.App/AppController.cs` | `ThemeChanged` 时若设置窗开着则 `_settingsWindow.ApplyTheme`（托盘切主题也能刷新设置铬）。 |

### 真机点检（开机后）

1. 打开设置：五组节奏 = 标题→8→控件行距 6→帮助→24 含发丝；关于下无发丝。
2. 浅色：底 `#F2F2F2`、字 `#1A1A1A`、发丝 `#C8C8C8`、帮助 `#666666`。
3. 点「切换系统深浅色」或托盘同项：设置窗底/字/发丝立刻翻深（`#2B2B2B` / `#F0F0F0` / `#555555`）。
4. About 句：`托盘小工具。截图出工具栏、网速进任务栏、托盘切深浅色。`（不再「截完即钉」）。
5. 「立即保存」(~100px) +「关闭」(88px)；改动自动写盘；外观字段仍生效。

---

## Appendix B — Settings interaction polish（2026-09-06，box）

**状态：** 源码 + 规格已改；**不能在 box 上跑 WPF**。等 Windows 开机编跑再点检。未改截图/钉图管道。

### Joe 覆盖
原 `SETTINGS-PAGE-SPEC` §7「点应用才写入 / 无实时预览」作废 → **Change → ApplySettings + save**。

### Diff summary

| 文件 | 改动 |
| --- | --- |
| `src/Suite.App/SettingsWindow.xaml` | 控件挂 Checked/Unchecked/SelectionChanged/LostFocus/ValueChanged；`NetSpeedDetailsPanel` / `SaveDirectoryRow` / `TaskbarFxDetailsPanel` / `TaskbarAlphaRow` 供级联；底栏「立即保存」。 |
| `src/Suite.App/SettingsWindow.xaml.cs` | `ScheduleApply` / `FlushApply`（滑块 200ms `DispatcherTimer`）；`UpdateCascades`；上/下行拦最后一行 Uncheck；成功状态 `已保存`（嵌入/热键/任务栏失败句仍可盖过）；`OnClosing` 仅当防抖 timer 仍在跑时 Flush（关闭=只关窗）。主题按钮仍立即、不走存盘队列。走 `AppController.ApplySettings`。 |
| `design/SETTINGS-PAGE-SPEC.md` | §7–8（及 §2/4.3/6/10 对齐）改为自动保存模型。 |

### 真机点检（开机后）

1. 勾「显示边框」→ 槽立刻变，底栏「已保存」；关窗再开仍勾着。
2. 拖字号：约 200ms 后槽字号变；狂拖不卡死。
3. 「立即保存」在防抖未到期时立刻落盘。
4. 关「显示网速」→ 下面网速块变灰仍可见勾选本身。
5. 只留「显示下行」时再点下行 → 勾选弹回 +「至少显示上行或下行。」
6. 关「截完后保存 PNG」→ 保存目录行禁用。
7. 任务栏效果关 → 档位/色/透明度淡化；档位切实色 → 透明度淡化。
8. 关窗无「放弃更改」；主题按钮仍立即翻铬。
