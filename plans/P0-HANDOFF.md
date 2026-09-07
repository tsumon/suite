# Suite P0 交接（Windows 真机验证）

- **日期：** 2026-09-05
- **远程：** Debian 只出源码。本机 **没有** 跑通 WPF `dotnet build`，**不要**把远程当绿。
- **目标：** 无 TaskbarFx 的「主题 + 网速」托盘小工具。Joe 审 diff 后在 Windows 编、测、点。

## 0. 实现选择（审 diff 时先看）

| 项 | 选择 |
| --- | --- |
| 托盘图标 | **WinForms 互操作**：`Suite.App` 同时 `UseWPF` + `UseWindowsForms`，用 `System.Windows.Forms.NotifyIcon`。不是第三方 NuGet，不是 `Shell_NotifyIcon` P/Invoke。 |
| 生产依赖 | **零** `PackageReference`。无 HttpClient、无遥测、无来路不明 NuGet。 |
| 测试依赖 | 仅 `Microsoft.NET.Test.Sdk` 17.14.1、`xunit` 2.9.3、`xunit.runner.visualstudio` 3.1.4（为了 `dotnet test`）。 |
| 开机启动 | 只写 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`，值名 `Suite`。 |
| 热键默认 | `Ctrl+Shift+F12`（避开 PrtScr）。`RegisterHotKey` 文档写 F12 可能被调试器占用；失败弹气泡，设置里可改，不循环抢注。 |
| settings | `%LOCALAPPDATA%\Suite\settings.json`。不存截图像素、不存密钥。 |
| 网速 API | `iphlpapi!GetIfTable2` / `GetIfEntry2`，1 s 采样。睡眠恢复走 `SystemEvents.PowerModeChanged` → 丢掉上一拍，避免恒 0。 |

## 1. 工具链（Windows）

在 **VS 2026 / MSBuild 18.0+** 的开发者命令行或已装 .NET 10 SDK 的 PowerShell：

```bat
dotnet --info
```

须看到 **.NET 10 SDK**，且 runtime 含 **Microsoft.WindowsDesktop.App**（WPF / WinForms）。架构 **x64**。

## 2. 编 / 测（只在 Windows 跑）

仓库根（本机把工作区放到哪就在哪；远程路径是 `/workspace/windows-suite-app/`）：

```bat
dotnet build src\Suite.sln -c Debug -p:Platform=x64
dotnet test  src\Suite.sln -c Debug -p:Platform=x64
```

预期：

- 产出 `src\Suite.App\bin\x64\Debug\net10.0-windows\Suite.exe`（若 SDK 未按 Platform 分子目录，则在 `bin\Debug\net10.0-windows\Suite.exe`）。以实际输出为准。
- 测试至少跑：`Suite.Contracts.Tests`、`Suite.ThemeToggle.Tests`、`Suite.NetSpeed.Tests`。
- ThemeToggle 测试用假注册表，**不写真 HKCU Personalize**。
- `IpHelperWindowsTests` 会在 Windows 上真调 `GetIfTable2`；非 Windows 直接 return（标「须 Windows」）。

**Debian 远程未执行上述命令，不能当已绿。**

## 3. 手工点检（ADR §7 P0）

启动：`src\Suite.App\bin\...\Suite.exe`（路径以 build 输出为准）。托盘出现「Suite」。

| # | 动作 | 预期 |
| --- | --- | --- |
| 1 | 再开一次 `Suite.exe` | 第二份立刻退出，托盘仍一份 |
| 2 | 托盘 → 开机启动 勾上 | `reg query HKCU\Software\Microsoft\Windows\CurrentVersion\Run /v Suite` 有值（指向 Suite.exe） |
| 3 | 去掉勾 | 同一命令报找不到值 |
| 4 | 托盘 → 翻转主题 | 设置里的颜色 **与** 任务栏/窗口浅深一起变；网速窗换色。任务管理器里 **explorer.exe 未被结束** |
| 5 | 企业锁主题或写入失败 | 气泡/设置状态栏可见失败，不当成成功 |
| 6 | 网速窗拖到别处，杀进程再开 | 位置还在；settings.json 有 left/top |
| 7 | 设置里换网卡；传一个大文件 | 上/下行数字变化，不是恒 0 |
| 8 | 睡眠再唤醒 | 下一两秒后数字恢复，不是一直 0。若 IfIndex 变了，按 Alias 重绑，或提示选网卡 |
| 9 | `Ctrl+Shift+F12`（或设置里的键） | 托盘气泡「热键已触发（占位，P0 不截图）」；`%LOCALAPPDATA%\Suite\hotkey.log` 多一行。不崩 |
| 10 | 热键已被占用 | 气泡提示改键；不循环抢注 |
| 11 | 以管理员运行 | 气泡提示请用标准用户；manifest 是 `asInvoker` |
| 12 | 打开 `%LOCALAPPDATA%\Suite\settings.json` | 只有偏好（开机启动、热键、IfIndex、窗位置）。无 PNG/像素/密钥 |

卸载（P0 无安装包，手工）：

1. 退出 Suite。
2. `reg delete HKCU\Software\Microsoft\Windows\CurrentVersion\Run /v Suite /f`
3. 可选删除 `%LOCALAPPDATA%\Suite`。

## 4. 门禁对照（AUDIT §4.1）

| ID | 落地 |
| --- | --- |
| P0-1 | 无 TaskbarFx 工程，无 TAP/SWCA P/Invoke，无 CreateProcess 拉宿主 |
| P0-2 | settings 只在 `%LOCALAPPDATA%\Suite\`；schema 无截图/密钥字段 |
| P0-3 | Run 只 HKCU；设置可逆；上文卸载删 Run |
| P0-4 | 写入失败可见；不杀 explorer |
| P0-5 | `asInvoker`；High IL 气泡提示 |
| P0-6 | 根 `.gitignore` 含 bin/obj/`*.user`/证书；未建 GitHub |
| P0-7 | 无网络客户端；生产零 NuGet |

## 5. 已知风险边（不要在 P0 用杀 explorer 兜）

- Win11 上 `ImmersiveColorSet` 可能半刷新（ADR R6）。P0 只记录机型，**禁止** `taskkill explorer`。
- 睡眠后 IfIndex 变化 → 恒 0：已按 Alias 回退；仍失败则设置里手选。
- 热键 vs 系统 PrtScr：默认已避开；F12 可能和调试器冲突，改键即可。
- 与 Auto Dark Mode 互斥：设置页有一句说明。

## 6. 审代码入口

```
src/Suite.sln
src/Directory.Build.props          net10.0-windows, nullable, TreatWarningsAsErrors, x64
src/Suite.Contracts/               settings schema + JSON
src/Suite.Platform/                Mutex、HKCU Run、Personalize、RegisterHotKey、SendMessageTimeoutW
src/Suite.ThemeToggle/             读-翻-写-广播 + ThemeChanged（可注入假注册表）
src/Suite.NetSpeed/                GetIfTable2/GetIfEntry2、Δ/Δt、选网卡
src/Suite.App/                     托盘、设置窗、网速 Topmost 可拖窗、热键占位
tests/Suite.Contracts.Tests/
tests/Suite.ThemeToggle.Tests/
tests/Suite.NetSpeed.Tests/
```

**没有** `TaskbarFx*`、`Suite.Capture`、`Suite.Pinboard`。
