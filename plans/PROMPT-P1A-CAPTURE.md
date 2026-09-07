# 任务：P1a — 截图 + 贴图（仍无注入）

你是远程 Grok Build 的「coding」。在已有 P0 骨架上加截图与贴图。Joe 是小白、不管环境；代码要能编，交接说明写大白话。

## 必读

- `/workspace/windows-suite-app/plans/P0.md`、`P0-HANDOFF.md`
- `/workspace/windows-suite-app/architecture/ADR.md` §1.2 / §3.3 / §4.1 / §7 P1（截图贴图部分）
- `/workspace/windows-suite-app/security/AUDIT.md` §4.1 仍遵守；本轮 **仍禁止** TaskbarFx / 管道 / SWCA / TAP（那是 P1b）
- 现有代码在 `/workspace/windows-suite-app/src/`

## 本轮 In

1. 新建 `Suite.Capture`：区域截图
   - 全屏遮罩、拖矩形、Esc 取消
   - API 顺序：`IGraphicsCaptureItemInterop.CreateForMonitor` → DXGI Desktop Duplication（能做再做）→ GDI BitBlt 回退
   - 结果：复制到剪贴板；可选存 PNG（设置开关）
   - 尊重 `SetWindowDisplayAffinity`（黑块可接受，不破解）
2. 新建 `Suite.Pinboard`：贴图窗
   - 截完或从剪贴板钉置顶窗
   - 拖动、滚轮缩放、透明度、关闭、鼠标穿透
   - **不做**分组/备份/GIF/虚拟桌面
3. 标注四件套（可做在 Capture 流程里）：矩形、箭头、文字、马赛克——能用即可，不定视觉
4. 接线 `Suite.App`：把原热键占位改成真正开截图；托盘加「贴图（从剪贴板）」；设置里热键仍可改（默认 Ctrl+Shift+F12）
5. 更新 `plans/P1A-HANDOFF.md`：大白话 Windows 怎么编、怎么试（Joe 可能仍不自己跑，但要写好）

## Out

- TaskbarFx*、命名管道、任何注入
- OCR、滚动截图、GIF、Super-snip
- 新 NuGet 除非 Microsoft 官方且必要（优先 CsWinRT / 系统自带）；写进 HANDOFF
- git push / 建仓

## 约束

- 闭源；不拷 ShareX/Snipaste 等 GPL/专有源码
- Debian 远程只出码，不要假称 build 绿
- 保持 P0 功能不坏（主题、网速、托盘、单实例、Run）

写完 stdout：`DONE` + 路径列表。
