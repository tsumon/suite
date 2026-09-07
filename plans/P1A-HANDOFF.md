# Suite P1a 交接（截图 + 贴图，仍无注入）

- **日期：** 2026-09-05
- **远程：** Debian 只出源码。本机 **没有** 跑通 WPF `dotnet build`，**不要**把远程当绿。
- **目标：** 在 P0「主题 + 网速」托盘小工具上，加上区域截图、四件套标注、贴图窗。仍然 **没有** TaskbarFx / 管道 / 注入。

## 0. 实现选择（审 diff 时先看）

| 项 | 选择 |
| --- | --- |
| 截图 API | 先 `IGraphicsCaptureItemInterop.CreateForMonitor`，失败再 DXGI Desktop Duplication，再 GDI `BitBlt`。受 `SetWindowDisplayAffinity` 保护的窗允许黑块，**不去破解**。 |
| 选区 UX | 先把每块屏幕抓进内存，再弹出全屏遮罩拖矩形。Esc / 右键取消，不留遮罩。 |
| 标注 | 截完后一个标注窗：矩形、箭头、文字、马赛克。能用即可。Enter 复制，Ctrl+Enter 钉图。 |
| 贴图 | WPF 置顶窗。拖动、滚轮缩放、Alt+滚轮透明度、关闭、鼠标穿透。不做分组/备份/GIF/虚拟桌面。 |
| 剪贴板 | 确认后写入 CF_DIB + PNG。不写临时位图文件。 |
| 存 PNG | 默认关。打开后写到「图片\Suite\Suite-时间.png」。取消或失败不落盘；写盘用同目录 `.tmp` 再改名，失败会删 `.tmp`。 |
| settings | 仍只在 `%LOCALAPPDATA%\Suite\settings.json`。只存开关和目录路径，**不存像素**。 |
| NuGet | **没有新的生产包。** 测试仍是 Microsoft.NET.Test.Sdk / xunit / xunit.runner.visualstudio。WinRT 投影靠目标框架 `net10.0-windows10.0.19041.0`（.NET 10 SDK 自带），不是手加 CsWinRT。 |
| 仍禁止 | TaskbarFx*、命名管道、SWCA、TAP、任何注入。 |

## 1. 工具链（Windows）

和 P0 一样，在 **VS 2026 / MSBuild 18.0+** 的开发者命令行，或已装 **.NET 10 SDK** 的 PowerShell：

```bat
dotnet --info
```

须看到 **.NET 10 SDK**，runtime 含 **Microsoft.WindowsDesktop.App**（WPF / WinForms）。架构 **x64**。

本轮工程目标是 `net10.0-windows10.0.19041.0`（Windows 10 2004 的 SDK 投影）。这是编译用的 Windows SDK 版本，**不是**「只支持 2004」。机器仍按 ADR：Win11 当前功能更新，Win10 22H2 作回归锚。

## 2. 编 / 测（只在 Windows 跑）

仓库根（本机把工作区放到哪就在哪；远程路径是 `/workspace/windows-suite-app/`）：

```bat
dotnet build src\Suite.sln -c Debug -p:Platform=x64
dotnet test  src\Suite.sln -c Debug -p:Platform=x64
```

预期：

- 产出 `src\Suite.App\bin\x64\Debug\net10.0-windows10.0.19041.0\Suite.exe`（若 SDK 未按 Platform 分子目录，则在 `bin\Debug\net10.0-windows10.0.19041.0\Suite.exe`）。以实际输出为准。
- 测试至少跑：原来的 Contracts / ThemeToggle / NetSpeed，再加上 `Suite.Capture.Tests`（选区数学、马赛克、跨屏裁切；**不**在测试里真截屏）。
- ThemeToggle 测试仍用假注册表，**不写真 HKCU Personalize**。
- `IpHelperWindowsTests` 仍会在 Windows 上真调 `GetIfTable2`。

**Debian 远程未执行上述命令，不能当已绿。**

## 3. 手工点检（截图贴图 + P0 不能坏）

启动：`src\Suite.App\bin\...\Suite.exe`。托盘出现「Suite」。

### 3.1 截图

| # | 动作 | 预期 |
| --- | --- | --- |
| 1 | `Ctrl+Shift+F12`（或设置里改过的键） | 屏幕变暗，光标变十字。**不是** P0 那句「占位，P0 不截图」。 |
| 2 | Esc 或右键 | 遮罩立刻消失，剪贴板不变，不崩。 |
| 3 | 拖一个矩形，Enter（或点「复制」） | 图进剪贴板。打开画图 / QQ 能粘贴。 |
| 4 | 拖矩形后画：矩形、箭头、点一下写文字回车、拖一块马赛克，再复制 | 粘贴出来看得到这四样（样子随便，能认出来即可）。 |
| 5 | 设置勾上「截完后保存 PNG」再截一张 | `%USERPROFILE%\Pictures\Suite\`（或你填的目录）多一个 `Suite-年月日-时分秒.png`。取消截图则 **不会** 多文件。 |
| 6 | 双屏时在副屏拖、或框跨两块屏 | 能选出区域；Esc 两块屏的遮罩都没了。 |
| 7 | 对着一个「防截屏」窗（网课/银行类，若有） | 那一块可以是黑的。这是正常，不要当 bug 去「修」。 |
| 8 | 热键已被占用 | 仍是气泡提示改键；不循环抢注。 |
| 9 | 托盘 → 区域截图 | 和热键一样进入选区。 |

日志（可选）：`%LOCALAPPDATA%\Suite\capture.log` 会记一行 `ok` / `cancel` / `fail` 和用了哪条 API（GraphicsCapture / DxgiDuplication / GdiBitBlt）。里面 **没有** 图片。

### 3.2 贴图

| # | 动作 | 预期 |
| --- | --- | --- |
| 10 | 标注窗点「钉图」，或设置勾上「截完后自动钉成贴图」 | 桌面出现置顶小窗，显示刚截的图。 |
| 11 | 托盘 → 贴图（从剪贴板） | 剪贴板有图则钉一张；没有图则气泡说明。 |
| 12 | 左键拖贴图 | 窗跟着动。 |
| 13 | 贴图上滚轮 | 放大/缩小。 |
| 14 | Alt+滚轮，或右键选 100%/75%/50%/25% | 透明度变。 |
| 15 | 点「穿透」或右键「鼠标穿透」 | 鼠标点到贴图会点到后面的东西。然后用托盘「取消贴图穿透」收回。 |
| 16 | 点 × / Esc / Delete（未穿透时） | 这张贴图关掉。 |

锁屏后贴图还在不在：请看一眼记下来即可（产品观察项，P1a 不改锁屏行为）。

### 3.3 P0 原功能（不能坏）

P0-HANDOFF 的表仍有效：第二份进程立刻退、开机启动 Run 键可逆、翻转主题不杀 explorer、网速窗位置还在、传文件数字会变、睡眠唤醒后非恒 0、管理员气泡、`settings.json` 仍无像素/密钥。

## 4. 门禁对照

### AUDIT §4.1（P0，仍遵守）

| ID | 落地 |
| --- | --- |
| P0-1 | 无 TaskbarFx 工程，无 TAP/SWCA P/Invoke，无 CreateProcess 拉宿主 |
| P0-2 | settings 只在 `%LOCALAPPDATA%\Suite\`；schema 无截图像素、无密钥 |
| P0-3 | Run 只 HKCU；设置可逆 |
| P0-4 | 写入失败可见；不杀 explorer |
| P0-5 | `asInvoker`；High IL 气泡提示 |
| P0-6 | 根 `.gitignore` 含 bin/obj/`*.user`/证书；未建 GitHub |
| P0-7 | 无网络客户端；生产零 NuGet |

### 本轮额外（截图，仍无注入）

| ID | 落地 |
| --- | --- |
| P1-6 | 尊重显示亲和；黑块即黑块 |
| P1-8 | 无截图临时目录。确认后才写 PNG；`.tmp` 失败即删。剪贴板和贴图都在内存。 |
| — | 设置页写明：会捕获当时屏幕可见内容；企业可能被策略/DLP 限制。 |

P1b（TaskbarFx + 管道 + Win10 SWCA）**还没做**。不要在本 diff 里找注入。

## 5. 已知风险边

- Graphics Capture 在部分 Win11 上可能闪一下系统黄框：我们抓完立刻停会话，不去申请 borderless。
- DXGI 复制在「已有其它程序占用桌面复制」时会失败，这时自动落到 GDI。GDI 在 HDR 屏上可能过曝，这是底线回退，不是 HDR 方案。
- 多屏不同 DPI：遮罩按每块屏自己的 DPI 摆。若某台机器框选对不齐，把机型/DPI 记下来。
- 热键 vs 系统 PrtScr：默认已避开；F12 可能和调试器冲突，改键即可。
- 贴图开穿透后，必须从托盘「取消贴图穿透」才能再点到它。

## 6. 审代码入口

```
src/Suite.sln
src/Directory.Build.props          net10.0-windows10.0.19041.0, nullable, TreatWarningsAsErrors, x64
src/Suite.Contracts/               多了 CaptureSettings（只开关/目录）
src/Suite.Capture/                 抓屏、遮罩、标注
src/Suite.Pinboard/                置顶贴图窗
src/Suite.App/                     热键真正开截图；托盘贴图/穿透；设置页
tests/Suite.Capture.Tests/
plans/P1A-HANDOFF.md               本文件
```

**没有** `TaskbarFx*`、命名管道、SWCA、TAP。

## 7. 卸载（仍无安装包）

1. 退出 Suite。
2. `reg delete HKCU\Software\Microsoft\Windows\CurrentVersion\Run /v Suite /f`
3. 可选删除 `%LOCALAPPDATA%\Suite`。
4. 若曾保存 PNG：`%USERPROFILE%\Pictures\Suite` 是截图，不是程序，删不删自己定。
