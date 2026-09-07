# 截图交接（完完全全按 Snipaste）

- **日期：** 2026-09-05
- **远程：** Debian 只有 .NET 8，**没有** .NET 10 / Windows Desktop。这里 **不能** 当绿，更不能声称工具栏已像 07。请在 Windows 上编、测、点。
- **没 push。**
- **没动** `TaskbarFx.*`、网速槽、贴图光环算法。

design 三份已齐：`design/CAPTURE-PIN-SPEC.md`、`design/SNIPASTE-TOOLBAR-SPEC.md`、`DESIGN.md`。本轮 **没有** 再执行「去工具栏」。文字按钮条换成对照 `design/snipaste-ref/07-toolbar-live-2.png` 的浅色图标栏。

## 现在怎么用

1. **F1**（无 Ctrl/Shift）或托盘「区域截图」→ 全屏压暗。左上角：`悬停点窗，拖动手选。松手出工具栏。Esc 取消。`
2. 悬停选窗（红 3px）或手拖。拖中只有红框 + 尺寸片 `宽 × 高 px`，没有锚点、没有栏。
3. **松手立刻**（默认）：选区留在原地，切蓝虚线框 + 八个锚点 + 三分辅助线 + 尺寸片，底下出现浅灰图标栏。
4. 栏从左到右（对照 07）：形状▾ · 曲线 · 铅笔 · 马克笔 · 马赛克 · T · 橡皮 | 撤销 · 重做 | × · 钉 · 存 · 复制。栏上无字，靠 tooltip。当前绘制工具右上红点。撤销/重做空则变淡。
5. **复制 / Enter / 左键双击** → 写剪贴板，退出，不钉。
6. **钉图 / 中键** → 写剪贴板 + 1:1 钉在选区原点（向外光环），退出。
7. **保存** → 存 PNG，栏还在；失败气泡「无法保存文件：…」。
8. **关闭 / Esc / 右键**（点空白，不是工具栏）→ 整次取消。文字输入中 Esc 先退出输入框。
9. 勾上「截完后自动钉成贴图」→ 松手跳过栏，直接钉。Shift+松手仍出栏。
10. **F3** 或托盘「贴图（从剪贴板）」→ 钉在指针附近。本轮设置页没有单独一行 F3。

形状键：左键 = 矩形；右键菜单可选椭圆。铅笔和马克笔都能画出线（马克笔更粗、半透明）。

## Windows 上编 / 测

```bat
dotnet build src\Suite.sln -c Debug -p:Platform=x64
dotnet test  src\Suite.sln -c Debug -p:Platform=x64
```

重点测试（远程没跑）：

- `Suite.Contracts.Tests`：默认 F1、无修饰；`PinAfterCapture=false`；schema 3；旧 schema &lt; 3 迁到这两项；F3 常量 `0x72`。
- `Suite.Capture.Tests`：OverlayHint 含「工具栏」不含「松手钉图」；尺寸片 `235 × 172 px`；`AnnotateToolbar.Order` 13 键；栏在选区下、比选区宽时右对齐；椭圆描边不填心。

## 手工点检（真机对照 07）

| # | 动作 | 预期 |
| --- | --- | --- |
| 1 | F1 → 拖一块松手 | 浅灰约 40px 图标栏，不是「矩形/箭头/文字」字条；能数出 13 个图标、两组竖线 |
| 2 | 同一松手 | 蓝虚线 3px + 八个白芯蓝圈 + `数字 × 数字 px` |
| 3 | 点曲线 | 该键右上 5×5 红点；其它键没有 |
| 4 | 点复制 / Enter / 双击 | 画图能粘贴；没有新贴图；遮罩没了 |
| 5 | 点钉图 / 中键 | 1:1 钉在选区原点；遮罩没了 |
| 6 | 点关闭 / Esc | 没遮罩、没贴图、剪贴板没被这次写入 |
| 7 | 勾自动钉 → 拖一块松手 | 跳过栏，直接钉 |
| 8 | F3（剪贴板有图） | 钉一张；无图则气泡「剪贴板里没有图片。」 |
| 9 | 遮罩还在时再按 F1 | 不叠第二层 |

Debian 远程 **不能** 声称第 1 条已绿。

## 改了哪些文件

| 路径 | 干什么 |
| --- | --- |
| `src/Suite.Capture/AnnotateToolbar.cs` | 13 键顺序、tooltip、贴边算法（可单测） |
| `src/Suite.Capture/AnnotateIcons.cs` | 自绘 16px Path，不引用 icon pack、不抠 Snipaste |
| `src/Suite.Capture/AnnotationWindow.*` | 图标栏 + 蓝框锚点 + 笔/马克笔/橡皮/撤销重做/存 |
| `src/Suite.Capture/CaptureUx.cs` | 尺寸片带 `px` |
| `src/Suite.Capture/PixelDraw.cs` | 椭圆、带色折线、橡皮还原 |
| `src/Suite.Capture/RegionOverlayWindow.cs` | 拖中红 3px；松手后辅助线；尺寸片顶居中 |
| `src/Suite.Capture/RegionSelectSession.cs` | 松手后进入 Annotate，遮罩不卸 |
| `src/Suite.Capture/CaptureService.cs` | 默认留遮罩出栏；保存失败不结束会话 |
| `src/Suite.App/HotKeyHost.cs` / `AppController.cs` | F1 截图；F3 贴图；忙碌不重入 |
| `src/Suite.Contracts/HotkeyBinding.cs` | `PinClipboardVirtualKey = F3` |
| `src/Suite.App/SettingsWindow.xaml` | 自动钉帮助改成「默认先出栏」 |
| `tests/Suite.Capture.Tests/*` | 新契约 |
| `tests/Suite.Contracts.Tests/SettingsJsonTests.cs` | 跟上 schema 3 / F1 / 默认不自动钉 |

## 没做 / 不要误会

- **没有** 逆向 Snipaste，**没有** 抄 ShareX。图标对照 07 结构自绘。
- **没有** OCR / 滚动截图 / GIF / 取色面板 / Esc 确认框。
- 椭圆是形状键右键菜单，不是完整下拉动画。
- 贴图光环、网速槽、任务栏效果 **没改**。
- v1 标注集合按 2026-09-05 目标图扩（笔/马克笔/橡皮/撤销重做），不是开 Snipaste Pro。
