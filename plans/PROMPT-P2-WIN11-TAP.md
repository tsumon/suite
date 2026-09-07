# coding：P2 Win11 透明（GPL，可用 TranslucentTB）

## 许可证（新拍板）
- 整包 **GPL-3.0**。见 `architecture/DECISION-GPL-TRANSLUCENTTB.md`、`NOTICE`。
- **允许** clone / 改写 TranslucentTB（GPL-3.0）的 ExplorerTAP / 任务栏外观路径。
- 必须：保留上游版权头；更新 NOTICE；根目录放完整 LICENSE 文本。
- **禁止** 掺 TrafficMonitor（Anti-996）源码。截图继续自写即可。

## 目标
tsumon Win11：设置启用任务栏效果后，Clear/Opaque 肉眼可见；Acrylic 尽力。不再永远「Win11 路径未交付」。

## 必读
- `plans/P2-WIN11.md`
- `architecture/ADR.md` §5.3（路径仍对，许可证段落以新决定为准）
- `architecture/DECISION-SINGLE-PROCESS.md`
- `security/AUDIT.md` F-01/F-04（绝对路径、杀软画像仍在）
- 现码 `src/TaskbarFx.Native/`、`Suite.App/TaskbarFxRuntime.cs`

## 做法建议
1. `git clone --depth 1` TranslucentTB 到 `vendor/TranslucentTB/`（或等价），不要无版权粘贴。
2. 优先复用其 Win11 TAP + 改 Taskbar 背景的能力，接到我们的 mode/argb；不要整份搬它的 XAML 设置 UI。
3. Native/Suite：ModernXaml 分支走 TAP；经典栏仍 SWCA。
4. 交接 `plans/P2-WIN11-HANDOFF.md`：编法、验收、如何 Reset、杀软可能报警。

## 验收
Clear 透明、Opaque 实色、关闭恢复；截图/网速/托盘深浅色仍可用。

DONE = 代码可在 tsumon 编 + 交接文档 + LICENSE/NOTICE 齐。
