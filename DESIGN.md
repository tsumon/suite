---
name: Suite
description: Windows 托盘小工具。F1 截图、松手出工具栏、可钉可贴。网速进任务栏。对标 Snipaste 公开 UI，不是作品集落地页。
colors:
  accent: "#DC2828"
  overlay-dim: "#0000006E"
  overlay-hint: "#FFFFFFFF"
  overlay-commit: "#2080F0"
  toolbar-icon: "#323232"
  toolbar-separator: "#7B7B7B"
  toolbar-selected-dot: "#FF0000"
  toolbar-hover: "#E8E8E8"
  toolbar-pressed: "#DDDDDD"
  anchor-fill: "#FFFFFF"
  pin-glow-light: "#FFFFFF47"
  pin-glow-dark: "#00000073"
  surface-dark: "#2B2B2B"
  surface-light: "#F2F2F2"
  ink-dark: "#F0F0F0"
  ink-light: "#1A1A1A"
  line-dark: "#555555"
  line-light: "#C8C8C8"
  settings-muted: "#666666"
  netspeed-zero: "#888888"
typography:
  ui:
    fontFamily: "Segoe UI, Microsoft YaHei UI, sans-serif"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.35
    letterSpacing: "normal"
  overlay-hint:
    fontFamily: "Segoe UI, Microsoft YaHei UI, sans-serif"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.3
    letterSpacing: "normal"
  overlay-size:
    fontFamily: "Segoe UI, Consolas, monospace"
    fontSize: "11px"
    fontWeight: 400
    lineHeight: 1.2
    letterSpacing: "normal"
  settings-title:
    fontFamily: "Segoe UI, Microsoft YaHei UI, sans-serif"
    fontSize: "13px"
    fontWeight: 600
    lineHeight: 1.3
    letterSpacing: "normal"
  settings-help:
    fontFamily: "Segoe UI, Microsoft YaHei UI, sans-serif"
    fontSize: "12px"
    fontWeight: 400
    lineHeight: 1.4
    letterSpacing: "normal"
  rate:
    fontFamily: "Consolas, Cascadia Mono, Segoe UI, monospace"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.2
    letterSpacing: "normal"
  rate-taskbar:
    fontFamily: "Consolas, Cascadia Mono, Segoe UI, monospace"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.05
    letterSpacing: "normal"
rounded:
  none: "0px"
  pin: "0px"
  overlay-chip: "3px"
  toolbar: "2px"
  float: "6px"
  settings-control: "2px"
spacing:
  overlay-hint: "16px"
  toolbar-gap: "8px"
  toolbar-cell: "36px"
  pin-glow: "22px"
  settings-page: "16px"
  settings-section: "24px"
  settings-title-gap: "8px"
  settings-row: "6px"
  float-pad: "10px 8px"
  taskbar-pad: "8px 0px"
components:
  overlay-hint:
    backgroundColor: "transparent"
    textColor: "{colors.overlay-hint}"
    typography: "{typography.overlay-hint}"
    padding: "{spacing.overlay-hint}"
  size-chip:
    backgroundColor: "#000000CC"
    textColor: "#FFFFFFFF"
    typography: "{typography.overlay-size}"
    rounded: "{rounded.overlay-chip}"
    padding: "2px 6px"
  annotate-toolbar:
    backgroundColor: "{colors.surface-light}"
    textColor: "{colors.toolbar-icon}"
    height: "40px"
    rounded: "{rounded.toolbar}"
    padding: "2px 8px"
  settings-primary:
    backgroundColor: "{colors.surface-light}"
    textColor: "{colors.ink-light}"
    typography: "{typography.ui}"
    rounded: "{rounded.settings-control}"
    padding: "4px 12px"
    width: "88px"
  netspeed-float:
    backgroundColor: "{colors.surface-light}"
    textColor: "{colors.ink-light}"
    typography: "{typography.rate}"
    rounded: "{rounded.float}"
    padding: "{spacing.float-pad}"
    width: "168px"
    height: "56px"
  netspeed-taskbar:
    backgroundColor: "transparent"
    textColor: "{colors.ink-light}"
    typography: "{typography.rate-taskbar}"
    rounded: "{rounded.none}"
    padding: "{spacing.taskbar-pad}"
    width: "148px"
    height: "client"
---

# Design System: Suite

## Overview

**Creative North Star: "桌面图钉"**

Suite 是托盘常驻小工具。人按 F1，框一块或点一个窗，松手后选区留在原地，底下立刻出现对照 Snipaste 的浅色图标工具栏。点钉才 1:1 钉在原地，带一圈向外光环。设置页只是开关和档位，不是品牌展示。

对标 Snipaste 的**公开 UI 结构**（F1、松手出栏、蓝框锚点）与微信截图的**选窗**（悬停、手拖、Esc 整次取消）。对照 `design/snipaste-ref/07-toolbar-live-2.png`，不抄资源、不逆向、不做成作品集风落地页。

**Key Characteristics:**

- Operate：第 100 次截图必须和第一次一样快，没有入场动画可等。设置是第 100 次打开的系统表单。
- 系统字体、系统任务栏密度。不自带展示字体。
- 拖中选区红 `{colors.accent}`，松手后选区蓝 `{colors.overlay-commit}`，当前工具一颗红点。贴图靠光环从桌面里分出来，不靠白底标题栏。
- 失败说人话并回退，不装成功。

## Colors

托盘工具跟系统浅/深走。截图遮罩永远是暗的，和系统主题无关。

### Primary

- **选区红** (`{colors.accent}`)：悬停窗/手拖时的描边。只用于「还在选、还没松手」。
- **选区蓝** (`{colors.overlay-commit}`)：松手后的虚线框、锚点圈、辅助线。采样自 07 `(32,128,240)`。
- **工具栏红点** (`{colors.toolbar-selected-dot}`)：当前绘制工具右上 5×5。不是把整键刷红。

### Neutral

- **遮罩墨** (`{colors.overlay-dim}`)：未选中区域压暗。选中区域零压暗，像素所见即所得。
- **深表面** (`{colors.surface-dark}`) / **浅表面** (`{colors.surface-light}`)：设置页、网速悬浮底、截图工具栏底。贴图窗**不用**这两种做底——贴图没有底板。
- **深字** (`{colors.ink-dark}`) / **浅字** (`{colors.ink-light}`)。网速上/下行默认跟 ink，不跟选区红。用户在设置里自选的行颜色覆盖默认。
- **说明灰** (`{colors.settings-muted}`)：设置页帮助句（**浅色**）。深色设置帮助用 `SETTINGS-PAGE-SPEC` `--set-ink-muted` `#A8A8A8`，勿用 `#666`。错误状态不要用说明灰当正文。
- **空闲速率** (`{colors.netspeed-zero}`)：未自选行颜色时，`0 B/s` 用这色。自选行颜色后，0 也用该行颜色。

**The 一色规则。** 红只用于拖中选区和工具红点。蓝只用于松手后的选区铬。按钮、托盘、设置页、网速数字都不刷红或刷蓝。禁止紫粉渐变、禁止纯 `#000` 当产品「暗黑品牌色」。浅色工具栏底用 `{colors.surface-light}`，不要改成暗色品牌条。

## Typography

**UI / 正文：** Segoe UI + Microsoft YaHei UI。固定 px，不 `clamp`。
**速率：** Consolas（缺则 Cascadia Mono）。只给 ↑↓ 数字，不当装饰。等宽，避免宽度乱跳把通知区图标挤走。

### Hierarchy

- **Overlay hint** (13 / 400)：遮罩左上角一句操作。
- **Size chip** (11 / 400)：`宽 × 高 px`（空格 + × + 空格 + `px`）。拖中和松手后都在。栏上无字。
- **Settings title** (13 / 600)：分组名。
- **Settings help** (12 / 400)：一行人话，不复述开关标题。
- **Rate**（悬浮固定 13 / 任务栏默认 13，用户可调 10–18）。任务栏行高 1.05，两行收成一块，再放进槽里垂直居中。

**The 无展示字规则。** 禁止 Inter、SF Pro、Geist、衬线标题。中文界面用句首大写即可，按钮不要 Title Case 英文。

任务栏速率 **禁止再写死 11px**。11px + 顶对齐 + 槽高 40 就是 `design/joe-netspeed-ugly.png`：字小、偏上、底下空。

## Layout

三块互不套卡片：

1. **截图遮罩**：每块屏一张全屏无边框窗。提示在左上，尺寸贴在选区外侧上沿。松手后工具栏贴选区下方（不够则上方），不挡选区像素。
2. **贴图**：出现在选区原点。内容尺寸 = 像素按该屏 DPI 换成 DIP，`Zoom = 1`。光环在像素外，不占内容盒。
3. **设置**：左导航五页（常规 / 截图 / 网速 / 任务栏效果 / 高级），不是一长页。契约与深色对比 token：`design/SETTINGS-PAGE-SPEC.md`（依据 taste-skill → WPF）。不要 hero、不要三列功能卡、不要 GroupBox 卡片套卡片。

网速默认进主任务栏通知区左侧槽。槽**高度 = 该任务栏客户区高度**（`height: client`），不是 40 DIP。失败才回到 168×56 可拖悬浮。

任务栏槽内：速率块光学垂直居中。左右 `{spacing.taskbar-pad}`（8px 0）。上下不靠加 padding 把空挪到另一侧。反例图那种「两行贴顶、底下一大块空」视为不合格。

## Elevation & Depth

贴图的深度 = **向外彩虹光环（发光彩虹）**，不是投影卡片，不是 1px 灰框，不是白晕硬框。

- 多层彩虹描边 + 外向软模糊（约 8px / 20px），色相缓慢循环；浅/深桌面都靠彩色对比分出来。
- 主题只调光环透明度，不再强制 `{colors.pin-glow-light}` / `{colors.pin-glow-dark}` 单色。
- 禁止：实心底板、厚白纸边、厚标题条、内阴影、玻璃模糊当装饰。

设置页、托盘菜单跟系统走，不加自定义阴影。任务栏网速默认**无框**；不要用 1px 品牌描边把槽画成方盒子。

**The 光环不是边框规则。** 光环在位图外；命中区仍是位图。不能先铺一层圆角白底再把图放进去。

## Shapes

- 贴图、任务栏网速：直角 `{rounded.none}`。像素要对齐，不能圆角裁图。不要给任务栏网速加圆角「好看」。
- 桌面网速悬浮：6px。截图工具栏：2px。都不是胶囊。
- 设置控件：系统默认；圆角上限 `{rounded.settings-control}`（2px），不要胶囊按钮。
- 选区拖中：3px 直角红描边，无锚点。松手后：3px 直角蓝虚线 + 八个圆锚点 + 三分辅助线（对照 `design/snipaste-ref/07-toolbar-live-2.png`）。这是目标铬，不是反例。
- 工具栏：2px 小圆角浅条，不是胶囊，不是选区那种直角位图。

## Components

### 截图遮罩

全屏、十字光标、无任务栏按钮。悬停窗/控件：红 3px 描边 + 窗外压暗。按下并拖过 4px：改手选矩形（仍红，无锚点）。松手：选区留下，切蓝虚线框 + 锚点 + 辅助线，下方出现工具栏。Esc / 右键：整次当没发生。勾了「截完后自动钉成贴图」才跳过栏、直接钉。

### 贴图

位图 + 向外光环。无标题栏、无 ×、无「穿透」按钮。拖动移动；滚轮缩放；Alt+滚轮透明度。Esc / Delete 关这一张。右键菜单：透明度、鼠标穿透、关闭。

### 标注工具栏（主路径，松手就有）

`{components.annotate-toolbar}`：高 40px，底 `{colors.surface-light}`，1px `{colors.line-light}`，圆角 2px。贴选区下方，缝 8px。图标 16px、色 `{colors.toolbar-icon}`，命中格 36px。

顺序（对照 07 + 识字）：形状▾ · 曲线 · 铅笔 · 马克笔 · 马赛克 · 文字 · **识字** · 橡皮 | 撤销 · 重做 | 关闭 · 钉图 · 保存 · 复制。栏上无字；tooltip 用这些中文。当前**绘制**工具：右上 5×5 `{colors.toolbar-selected-dot}`（识字键不打红点）。OCR / 滚动长图：`design/OCR-SCROLL-SPEC.md`、`design/INTERACTION-P1.md`。 P2：`design/INTERACTION-P2.md`。

像素契约：`design/SNIPASTE-TOOLBAR-SPEC.md`。禁止改回中文文字按钮条，禁止 Lucide/暗色玻璃栏。

### 网速

任务栏槽（`components.netspeed-taskbar`）：

- 宽 148 物理像素；**高 = 主任务栏客户区**，禁止写死 40 DIP。
- 默认 13px Consolas、行高 1.05、字重 400、透明底、无圆角、**无框**、不可拖。
- 两行默认 `↓ …` 在上、`↑ …` 在下。内容块在槽里光学垂直居中（Consolas 视觉重心略偏上，允许整体下移 1 DIP）。
- 字号用户可调 10–18，只改槽，不改悬浮。
- 上/下行颜色、显隐、背景着色：槽和悬浮共用。

悬浮（`components.netspeed-float`）：168×56 DIP，圆角 6px，13px，可拖。背景透明开关**只作用于槽**；悬浮在桌面上始终有 `{colors.surface-*}` 底板，除非用户关掉透明并选了着色，那时悬浮改用该色。

断网卡：`↓ —` / `↑ 选网卡`（只显示存在的行）。

### 设置

系统控件。开关说结果，帮助句说失败会怎样、或主入口在哪。**改完即应用并自动写盘**（滑块约 200ms 防抖）；见 `design/SETTINGS-PAGE-SPEC.md` §7。

系统浅/深色：**主入口是托盘右键「切换系统深浅色」**。设置页「常规」里保留同名次要按钮。任务栏效果只出现在设置，视觉不进截图/贴图。

## Do's and Don'ts

### Do:

- **Do** 默认热键 F1（无修饰）。松手立刻出对照 07 的浅色图标工具栏。
- **Do** 点钉图 / 中键才 1:1 钉在选区原点；复制 / Enter / 左键双击只写剪贴板。
- **Do** 微信式悬停高亮 + 点击选窗，同时保留手拖。
- **Do** 任何截图阶段 Esc / 右键 = 整次取消（不钉、不写剪贴板、不存盘）。文字输入中 Esc 先退出输入框。
- **Do** 贴图只用向外光环和桌面区分；光环可 ≤120ms 淡入。设置窗可选同等时长 Opacity 淡入。
- **Do** 网速默认钉任务栏；失败托盘气泡 + 设置状态栏说人话，并立刻改悬浮。
- **Do** 任务栏网速高度跟客户区，字用 `{typography.rate-taskbar}`，内容光学垂直居中。
- **Do** 托盘右键「切换系统深浅色」切系统 Apps+System 浅/深；不结束 explorer。
- **Do** 任务栏透明的文案只出现在设置：开关、档位、失败句。实现为 Suite **同进程内模块**（不拉起 TaskbarFx.exe）；Joe 接受透明挂了截图也可能挂。

### Don't:

- **Don't** 做成中文文字按钮条（现行「矩形/箭头/文字/马赛克/复制/钉图/取消」是反例）。
- **Don't** 用 Lucide / Phosphor / Tabler、暗色玻璃、胶囊 FAB、窗底通栏去「改进」07 那条浅栏。
- **Don't** 默认先钉再标。自动钉是设置，默认关。
- **Don't** 把 `design/snipaste-ref/07-toolbar-live-2.png` 或 `design/joe-snipaste-toolbar-target.png` 再写成反例。
- **Don't** 贴图用填色铬、圆角卡、标题条、作品集风渐变。
- **Don't** 紫渐变、Inter、Lucide 装饰图标、三卡片布局、弹跳入场、从 0 放大弹出贴图。
- **Don't** 把设置做成圆角 Bento / Hero / 一长页堆五组。设置**允许**左导航分页（见 SETTINGS-PAGE-SPEC v2）。
- **Don't** 任务栏网速写死 11px / 槽高 40 DIP / 顶对齐 / 默认 1px 框（反例 `design/joe-netspeed-ugly.png`）。
- **Don't** 给任务栏网速加圆角或品牌描边。
- **Don't** 网速数字滚动；设置打开不要弹跳/滑入/作品集入场。允许 ≤120ms Opacity 淡入（Operate 轻量）；截图工具栏松手必须零延迟。
- **Don't** 错误写「Oops」「出错了」。要写发生了什么、现在变成什么。
- **Don't** 任务栏效果失败还显示「已应用」。Win11 现代任务栏写「Win11 路径未交付」。
- **Don't** 为了好看改截图像素比例或给贴图加内边距。
---
