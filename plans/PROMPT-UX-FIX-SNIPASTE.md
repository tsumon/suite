# 紧急修复：对齐 Snipaste 体验 + 网速钉任务栏

你是 coding。Joe 反馈不好用。先读 `/home/box/agent-data/workflows/diagnosing-bugs/SKILL.md`。对照 `/workspace/windows-suite-app/research/REPORT.md` §2.1 Snipaste。

## Joe 要求（必须满足）

1. 网速要**固定在任务栏**，不要只靠悬浮窗。
2. 截图对齐 Snipaste：截好**直接 pin**；pin **一比一**；不好就 **Esc** 退出。
3. 不抄 Snipaste/ShareX/TrafficMonitor 源码。

## 已观察线索（验证后改）

- `PinAfterCapture` 默认 false → 应默认 true
- 流程像「框选→标注→Enter」不像「松手就钉」
- `PinWindow` Image 未按 DPI 设成像素 1:1 DIP，高 DPI 会巨大
- 任务栏 `SetParent` 嵌入未做

## 必做

A. 默认：框选确认 → 立刻 1:1 pin（靠近选区）+ 剪贴板；标注可选。Esc 各阶段干净取消。Pin 尺寸 = PixelWidth/Height 按 DPI 换 DIP，Zoom=1。
B. 网速默认任务栏嵌入（Shell_TrayWnd SetParent）；失败提示并回退悬浮；设置可切回仅悬浮。
C. 写 `plans/UX-FIX-HANDOFF.md` 大白话。
D. 可测：DPI→DIP 尺寸纯函数测试。

避开正在改的 TaskbarFx.Host/Native 无关冲突。不 git push。DONE + 路径。

## 追加（Joe 2026-09-05，必须本轮）
- 贴图：Snipaste 式向外光环/光晕；不要蠢土方框厚标题。
- 截图：微信式自动选窗（悬停高亮窗口/控件，点击截取）+ 拖拽手选；Esc 取消。UIA/WindowFromPoint，禁止逆向。
见 plans/UX-ADD-GLOW-WINDOW.md
