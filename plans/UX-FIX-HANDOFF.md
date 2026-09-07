# UX 修复交接（Snipaste 式贴图 + 网速钉任务栏）

- **日期：** 2026-09-05
- **远程：** Debian 只有 .NET 8，**没有** .NET 10 / Windows Desktop。本机 **不能** 当绿。请在 Windows 上编、测、点。
- **没 push。** 没动 `TaskbarFx.Host` / `TaskbarFx.Native`。

Joe 嫌不好用。对上的问题：默认不自动钉、框选完还要进标注按 Enter、高 DPI 贴图巨大、网速只有桌面悬浮。

## 现在怎么用

### 截图（默认对齐 Snipaste「截完就钉」）

1. `Ctrl+Shift+F12`（或托盘「区域截图」）→ 屏幕变暗，十字光标。
2. **拖一个框，松开鼠标** → 立刻在选区附近钉一张 **1:1** 贴图，同时进剪贴板。
3. 觉得钉错了：贴图上按 **Esc**（或 Delete）关掉。框选过程中 Esc / 右键 = 整次取消，不钉、不写剪贴板。
4. 要先标注：框选时按住 **Shift 再松手**。标注里 Enter 只复制、Ctrl+Enter 或「钉图」才钉、Esc 取消。
5. 设置里可关掉「截完后自动钉成贴图」。关掉后，松手会进标注窗（老流程）。

贴图尺寸：`像素宽/高 × 96 / 该屏 DPI`，缩放一开始是 1。150%（144 DPI）下 200 像素宽的图是 133.3 DIP，看起来和屏幕上选的一样大，不会撑满半个桌面。标题条叠在图上，鼠标移上去才出现，不把图撑高。

### 网速（默认钉任务栏）

1. 启动后网速默认 **嵌进任务栏**（`FindWindow("Shell_TrayWnd")` + `SetParent`）。自己写的 Win32，**没抄** TrafficMonitor。
2. 钉不上（找不到任务栏、Win11 不让嵌）：气泡提示，**自动改回桌面悬浮窗**。
3. 设置里取消「钉在任务栏」= 只用可拖的悬浮窗。
4. 关掉「显示网速」则两者都没有。

Win11 居中任务栏 / 小组件仍可能挡一挡，这是系统限制，失败就回悬浮。

## Windows 上编 / 测

```bat
dotnet build src\Suite.sln -c Debug -p:Platform=x64
dotnet test  src\Suite.sln -c Debug -p:Platform=x64
```

重点测试：

- `Suite.Contracts.Tests`：默认 `PinAfterCapture=true`、`EmbedInTaskbar=true`；schema 1 旧文件会升到 2 并打开这两项；schema 2 里如果用户明确关掉，会保持关。
- `Suite.NetSpeed.Tests`：`DipConvertTests`（DPI→DIP）、`TaskbarSlotTests`（任务栏槽位纯函数）。
- `Suite.Capture.Tests`：`CaptureUxTests`（默认跳过标注；Shift 才标注）。

Debian 远程没有跑通上述命令。

## 手工点检

| # | 动作 | 预期 |
| --- | --- | --- |
| 1 | 热键 → 拖框松手 | 立刻出现贴图，大小跟选区差不多，位置靠近选区；画图能粘贴 |
| 2 | 150% DPI 再截一小块 | 贴图不是巨大一块 |
| 3 | 框选中 Esc / 右键 | 遮罩没了，没有贴图 |
| 4 | 贴图上 Esc | 这张贴图关掉 |
| 5 | Shift+松手 → 画箭头 → Esc | 不钉 |
| 6 | 启动后看任务栏 | 网速在任务栏里，不是挡桌面的悬浮窗 |
| 7 | 设置取消「钉在任务栏」点应用 | 变回可拖悬浮窗 |
| 8 | 设置取消「显示网速」 | 任务栏和悬浮都没了 |

## 改了哪些文件（审 diff）

| 路径 | 干什么 |
| --- | --- |
| `src/Suite.Contracts/CaptureSettings.cs` | `PinAfterCapture` 默认 true |
| `src/Suite.Contracts/NetSpeedSettings.cs` | `EmbedInTaskbar` 默认 true |
| `src/Suite.Contracts/AppSettings.cs` | schema 2 |
| `src/Suite.Contracts/SettingsJson.cs` | 旧 schema 1 升上来时打开钉图 + 任务栏网速 |
| `src/Suite.Platform/DipConvert.cs` | 像素 → DIP 纯函数 |
| `src/Suite.Platform/TaskbarEmbed.cs` | `SetParent(Shell_TrayWnd)`，失败让调用方回退 |
| `src/Suite.Capture/CaptureService.cs` | 默认松手就钉；Shift 才标注 |
| `src/Suite.Pinboard/PinWindow.*` | 按 DPI 设 1:1，Zoom=1 |
| `src/Suite.App/NetSpeedWindow.xaml.cs` | 嵌入 / 回退悬浮 |
| `src/Suite.App/AppController.cs` | 接上上面两套；**保留** 已有 TaskbarFx 管道调用 |
| `tests/Suite.NetSpeed.Tests/DipConvertTests.cs` | DPI→DIP |
| `plans/UX-FIX-HANDOFF.md` | 本文件 |

## 没做 / 不要误会

- **没有** 抄 Snipaste / ShareX / TrafficMonitor 源码。
- **没有** 改 TaskbarFx 注入 / SWCA。网速嵌任务栏和任务栏亚克力是两件事。
- Win11 任务栏嵌不进去是已知死角，产品行为是提示 + 悬浮，不是假装成功。

## 第二节 · 光环 + 自动选窗 + 去厚工具栏（2026-09-05）

- **监控结论：** 指定 log `llm-build/logs/_____________________-20260905-032850.out`（终端「修复·光环选窗」/ `PROMPT-UX-FIX-ROUND2.md`）**失败提前退出**：约 03:28 UTC+0 启动，仅写出一句开场白（234B），**无 EXIT=/DONE**，进程约 03:31 已不在。该轮 **未** 自己改码也 **未** 更新本文件。
- **功能落地：** 同主题能力由后续「编码·按设计实现」（`PROMPT-IMPLEMENT-DESIGN.md`，log `…-20260905-034138.out`，约 04:01 UTC+0 写出 DONE）按 `DESIGN.md` / `design/CAPTURE-PIN-SPEC.md` 收口。详试步骤见 `plans/IMPLEMENT-HANDOFF.md`。**没 push。** Debian 仍不能当绿。

### 期望核对

| 期望 | 状态 | 证据 |
| --- | --- | --- |
| Pin 向外光环 / DropShadow | **有** | `PinWindow.xaml`：`GlowFar`/`GlowNear` + `DropShadowEffect`（Blur 22/8，ShadowDepth 0）；标题条 / × / 「穿透」按钮已去掉，仅右键菜单 |
| 自动选窗（WindowFromPoint 等） | **有** | `WindowPicker.cs`：`WindowFromPoint` + UIA `AutomationElement.FromPoint` + `ChildWindowFromPointEx` / `GetAncestor`；`RegionSelectSession` 悬停高亮，单击 `CommitHover` 立刻 1:1 pin；仍可手拖；Esc/右键整次取消 |
| 厚工具栏不再是默认主流程 | **有** | 默认仍松手即 pin（Round1）；`AnnotationWindow` 改为右键菜单 + ≤28px `HoverBar` 深色文字条；主路径无厚白图标栏 |

### 缺项 / 注意

1. **ROUND2 专用 agent 本身未交付**（log 僵死）；勿把 032850 当成功跑。
2. **无** `WindowPicker.Hit` 的独立单测（Win32/UIA）；纯函数测在 `CaptureUxTests`：`DecideRelease` / `IsDrag`(4px) / `SizeChip`（`235 × 172`）。
3. 远程 **未** `dotnet build/test`；需 Windows 真机点检（悬停选窗、光环、Shift 才进标注）。
4. 同一次 IMPLEMENT 还把任务栏效果改成 Suite **同进程**加载 `TaskbarFx.Native.dll`（不再主路径拉 `TaskbarFx.exe`+管道）。这超出 ROUND2 原文，但已落；**第一节「保留管道」表述已过时**，以 `IMPLEMENT-HANDOFF.md` 为准。
5. 未验证 Win11 真机选窗/嵌任务栏观感；UIA 控件级依赖环境，失败时应仍能手拖（规格如此）。

### 本轮相关路径（审 diff）

| 路径 | 干什么 |
| --- | --- |
| `src/Suite.Pinboard/PinWindow.xaml(.cs)` | 双层 DropShadow 光环；去掉厚铬标题条 |
| `src/Suite.Capture/WindowPicker.cs` | 悬停选窗 |
| `src/Suite.Capture/Native/CaptureNative.cs` | `WindowFromPoint` 等 P/Invoke |
| `src/Suite.Capture/RegionSelectSession.cs` / `RegionOverlayWindow.cs` | 悬停高亮 + 单击/拖动手选 |
| `src/Suite.Capture/AnnotationWindow.xaml(.cs)` | 右键 + 28px HoverBar，去掉顶栏厚工具条 |
| `src/Suite.Capture/CaptureUx.cs` + `tests/.../CaptureUxTests.cs` | 释放决策 / 尺寸片 / 拖拽阈值 |
| `plans/IMPLEMENT-HANDOFF.md` | 完整点检与同进程任务栏说明 |
| `plans/UX-FIX-HANDOFF.md` | 本第二节（监控追加） |

