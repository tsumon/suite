# OCR + 滚动长截图 — Operate 合约（v1.1）

日期：2026-09-09（滚动改为用户自滚 + 空闲停）。权威视觉：根目录 `DESIGN.md`。工具栏像素：`SNIPASTE-TOOLBAR-SPEC.md`。  
本文件定 **入口 / 状态 / 失败句**。不写 `src/`。模式：**Operate**（第 100 次截图一样快；无入场动画）。

交互总览见 `INTERACTION-P1.md`。

## 本轮已实现（box 2026-09-06）

- [x] `WindowsOcrService`（WinRT `OcrEngine`，BGRA8 → SoftwareBitmap）；不可用时 `NullOcrService`
- [x] 松手栏 `Ocr` 键：`Text`/`Eraser` 之间；tooltip「识字」；成功「已复制文字。」；栏不关
- [x] `ScrollCaptureService` + `ScrollCaptureSession`（点选/拖区 → **用户自滚** → 屏矩 BitBlt 拼接 → 空闲 `IdleStopMs` 默认 1.5s 停；**不**注入滚轮）
- [x] 托盘「滚动长截图」+ 设置·截图次要按钮；**不**改 F1
- [x] 截图帧 / Bake 缓存 / 会话结束 `ReleasePixels`；滚动中间帧释放

真机 Win11 矩阵、Chromium 自绘、语言包路径 —— **未在 Debian 验证**（见 `plans/BOX-DONE-20260906.md`）。

---

## 1. OCR 识字

### 1.1 入口

| 项 | 合约 |
| --- | --- |
| 主入口 | 松手后标注工具栏 **图标键**「识字」（id `Ocr`） |
| 栏上 | **无字**；tooltip：`识字` |
| 位置 | 绘制组：`Text` 与 `Eraser` 之间插入一键（见 §1.5） |
| 命中 | 同栏：36×36 格，图标 16×16，色 `{colors.toolbar-icon}` |
| 图标形态 | 「T + 横线」或「文」线框（一份描边 ~1.5px @16）；**不要**中文大按钮、不要 Lucide 彩图标 |
| 何时可点 | 松手后栏已出；选区有像素。自动钉跳过栏时：**无** OCR 入口（要识字请关自动钉再截） |
| 非入口 | 托盘菜单、设置页、F1 修饰键 —— v1 不做 |

### 1.2 范围与引擎

| 项 | 合约 |
| --- | --- |
| 范围 | 当前标注选区位图（含已画笔画的提交图；与复制/保存同源） |
| 引擎 | `Windows.Media.Ocr.OcrEngine`；优先用户 UI 语言对应语言包 |
| 成功动作 | 纯文本写入剪贴板；**不**钉图、不存 PNG、不关遮罩（栏仍在，可继续操作） |
| 非目标 v1 | 框选单字、翻译、编辑气泡、云 API、多页 PDF、实时预览层 |

### 1.3 状态

| 状态 | UI |
| --- | --- |
| 空闲 | `Ocr` 可点；无红点（识字不是绘制工具） |
| 进行中 | 键半透明或短暂禁用；遮罩可保留；**不要**全屏 spinner / 进度环 |
| 成功 | 托盘气泡或遮罩左上 hint 一句：`已复制文字。` ≤2s 消失；栏不关 |
| 无文字 | 失败句见下；栏不关 |
| 语言包缺失 | 失败句见下；可附「打开语言设置」次要动作（系统设置，可选 v1） |
| 引擎不可用 / stub | 失败句见下 |
| Esc | 识字进行中：取消本次识别，回到松手栏；不取消整次截图 |

### 1.4 失败句（人话，禁止 Oops / 内部码当主句）

| 情况 | 句 |
| --- | --- |
| 识别为空 | `没识别到文字。` |
| 语言包缺失 | `系统缺少 OCR 语言包，请在 Windows 语言设置里安装后再试。` |
| 引擎不可用 | `本机暂时无法识字。` |
| stub / 未接线 | `识字尚未在本机启用。` |
| 取消 | 静默（无句） |

### 1.5 工具栏顺序（相对 07 插入一键）

```
Shape, Curve, Pencil, Marker, Mosaic, Text, Ocr, Eraser | Undo, Redo | Close, Pin, Save, Copy
```

断言：`Ocr` 在 `Text` 与 `Eraser` 之间。提交组仍无字。更新 `SNIPASTE-TOOLBAR-SPEC.md` 键表时以此为准。

### 1.6 代码接口（契约）

- `Suite.Capture.Ocr.IOcrService`：`Task<OcrResult> RecognizeAsync(PixelBuffer image, CancellationToken ct)`
- `OcrResult`：`Succeeded` / `Text` / `Error`（中文，可空）
- stub：`NullOcrService` → `识字尚未在本机启用。`

---

## 2. 滚动长截图

### 2.1 入口（**不**进默认 F1 松手路径）

| 入口 | 角色 | 文案 |
| --- | --- | --- |
| 托盘右键 | **主入口** | `滚动长截图`（放在截图相关项附近；**不要**盖过「切换系统深浅色」第一项） |
| 设置 · 截图组 | 次要说明 + 可选「开始滚动长截图」次要按钮（系统按钮，左对齐） | 帮助：`从托盘也可开始。点选或拖区后自己滚动，停滚约 1.5 秒自动完成。` |
| F1 / 松手栏 | **禁止**默认塞入；禁止改成 F1 行为 | — |

点击入口后：拾取（点窗吸附或拖矩形）→ 用户自滚 → 空闲自动完成。

### 2.2 范围与产出

| 项 | 合约 |
| --- | --- |
| 范围 | 点选窗口客户区（悬停吸附）**或**拖拽矩形视口；捕获的是 **屏幕矩形**（BitBlt），不依赖 HWND 滚消息 |
| 产出 | 单张竖向拼图 → 剪贴板；若设置勾了「截完后保存 PNG」则同时存盘 |
| 滚法 | **用户**在选区内自己滚；Suite **不**发 `WM_MOUSEWHEEL` / `VSCROLL`。周期采样差分竖向拼接；拼高大于首帧后，**无新内容达 `IdleStopMs`（默认 1.5s）** 自动完成 |
| 非目标 v1 | 整桌自动滚、浏览器插件、横向长图、视频、跨进程注入、注入式滚轮 |

### 2.3 状态

| 状态 | UI |
| --- | --- |
| 拾取 | 点窗吸附 / 拖区；红描边；提示：`点选窗口或拖选区域；Esc 取消。`；Esc / 右键 = 整次取消 |
| 等待用户滚 | 选区红框（可点穿）；hint：`在选区内滚动；停滚约 1.5 秒后自动完成` |
| 拼接中 | `正在拼接长图…`；Esc 取消（已拼丢弃，静默） |
| 成功 | `已复制长图。`（若同时存盘：`已复制长图并保存。`） |
| 空闲完成 | 用户停滚约 1.5s 且已拼出新内容 → 成功句 |
| 失败 | 见 §2.4；不装成功 |

### 2.4 失败句

| 情况 | 句 |
| --- | --- |
| 找不到可滚窗 | `没找到可滚动的窗口。` |
| 不支持的合成/自绘 | `这个窗口暂时滚不动长图。` |
| 超时且无帧 | `滚动超时，没有截到内容。` |
| 取消 | 静默 |
| 未实现 / stub | `滚动长截图尚未在本机启用。` |

### 2.5 接口（可后补实现）

- `IScrollCaptureService.CaptureAsync(PixelRect screenRect, ScrollCaptureOptions options, CancellationToken ct)` — 只采样拼接，不滚
- `ScrollCaptureSession` — 拾取 + 录制编排；`IdleStopMs` 默认 1500
- BACKLOG：真机 Win11 矩阵（Chromium、平滑滚动、DPI 混合）再验一次


### 2.6 拼接匹配（防重复）

- 指纹：取上一帧底部条带，在下一帧用 subsampled BGR SAD 搜最佳 Y；`append = scrollDelta`，仅在 mean-abs < 阈值时接受。
- **拒绝坏匹配**：无过线候选则 **append 0**（更新 previous / idle），**禁止** `max/4` 一类猜测 overlap。
- 捕获矩形相对红描边 **inset ~4px**，避免描边进 BitBlt。

---

## 3. Don't

- Don't 为 OCR / 长截图加作品集入场动画或全屏庆祝
- Don't 默认改 F1；Don't 工具栏改回中文大按钮条
- Don't 第三方 OCR NuGet（优先 WinRT）
- Don't 失败用 Oops、英文 Title Case、「Error 0x…」当主句
