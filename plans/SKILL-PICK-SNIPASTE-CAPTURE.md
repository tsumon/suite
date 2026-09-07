# 本次截图该用哪些 skill（完完全全按 Snipaste）

- **日期：** 2026-09-05
- **任务：** 截图主路径改成 Snipaste：热键 F1；框选松手后必须出现附图那种浅色图标工具栏
- **本文件只做 skill 路由。** 不写业务代码、不改 `src/`、不重开调研、不换产品世界。
- **下一轮入口：** 先 **design**（`design/PROMPT-SNIPASTE-CAPTURE.md`），再 **coding**（design 三个文件齐了再叫；本文件 §6.2 给提示要点）

Joe 怒评当前工具栏「很差」，要的不是「更好看」，是 **完完全全按照 Snipaste**：

1. 热键 **F1**（无 Ctrl/Shift）。
2. 框选松手后立刻出 **浅色图标工具栏**，对照 `design/joe-snipaste-toolbar-target.png`。
3. 红框是 **目标**，不是讨厌。此前把同一张图写成反例，已纠正。

---

## 1. 这次是什么活

| 判定 | 结论 |
| --- | --- |
| 产品面 | WPF 托盘常驻小工具。截图遮罩 + 松手后工具栏，不是落地页、不是新品牌 |
| Impeccable 模式 | **Operate**：熟悉感是功能。不像 Snipaste 的工具栏就是失败 |
| 视觉权威 | 根目录 **已有** `DESIGN.md`。字体/色/圆角仍以它为底。**但是** 截图主路径相关段落把 Joe 要的栏写成禁止——本轮 **必须改这些段落和 token**，不是发明第二套世界 |
| 缺口类型 | （A）规格把目标图当反例；（B）实现是中文文字按钮条，不是附图图标栏；（C）热键/状态机文档仍写 Ctrl+Shift+F12、松手即钉 |
| 许可证 | 学 Snipaste **公开行为 + 附图结构**。不逆向二进制（EULA）。不抄 ShareX（GPL） |

Joe 的设计工作流（`design-md-workflow`）管路由：先读 `DESIGN.md`，再 **按需** 调允许的 skill。禁止 Taste / Frontend Design / UI Brain / UI/UX Pro Max。技能建议若跟已拍板的 Operate 契约打，契约赢——**唯一例外**是本次 Joe + 目标图直接打脸当前「禁止厚白图标栏 / 松手即钉」条款，那些条款要改。

上一轮 `plans/SKILL-PICK-UX.md` 把 `plans/joe-hate-toolbar-ref.png` 当讨厌对象，要求砍工具栏、默认 pin。那是误判。**同一红框现在叫** `design/joe-snipaste-toolbar-target.png`，是验收图。不要再执行「去厚工具栏」那套。

这是既有截图表面的 **refinement + 规格推翻**，不是 new-work。禁止 concept-seed、禁止五套视觉方向。

### 1.1 编码前就能钉死的证据（路由用，不当成已修）

三方对不上时听 **本次 Joe + 目标图**，不要听旧 spec，也不要听半截实现。

| 面 | 现在怎样 |
| --- | --- |
| Joe | 很差。完完全全按 Snipaste。F1。松手必须出附图那种浅色图标栏 |
| 目标图 | `design/joe-snipaste-toolbar-target.png`：选区下一条浅灰底图标栏；从左到右约 形状▾、箭头（红点=当前工具）、笔、马克笔、马赛克、T、橡皮、撤销、重做、×、钉、存、复制。选区上有蓝色虚线变换框、圆控制点、尺寸片 `235 × 172 px` |
| `DESIGN.md` | Don't：主路径禁止厚白图标栏。标注只许右键菜单或 ≤28px **文字**悬停条。选区红描边、无控制点、无蓝色变换框 |
| `design/CAPTURE-PIN-SPEC.md` | 开篇把该红框写成反例。状态机默认 `PinAndCopy`。热键 Ctrl+Shift+F12。主路径禁止一排图标 |
| `design/SETTINGS-PAGE-SPEC.md` §4.2 | 仍写默认 Ctrl+Shift+F12；自动钉帮助仍是「松手立刻钉、Shift 才标注」 |
| 代码（半截改了） | `HotkeyBinding` 默认 F1；`CaptureSettings.PinAfterCapture` 默认 false；schema 3 会把旧设置迁到 F1 + 关自动钉；`CaptureUx.OverlayHint` =「松手出工具栏」；`AnnotationWindow` 是 `#F0F0F0` 高 36 的 **中文文字按钮**（矩形/箭头/文字/马赛克/复制/钉图/取消），没有附图那 13 个图标，没有笔/马克笔/橡皮/撤销重做/存 |
| 测试（还停在旧规格） | `CaptureUxTests` 仍断言「松手钉图」；`SettingsJsonTests` 仍断言默认 pin=true、热键 Ctrl+Shift+F12、schema 2 |

结论：Joe 骂「很差」不是「没做工具栏」，是 **做了一条不像 Snipaste 的文字条**，而规格还在禁止做对的那条。design 必须先推翻契约；coding 再跟新规格。不要 coding 先改、design 后补，也不要 coding 继续「去工具栏」。

`plans/joe-hate-toolbar-ref.png` 与目标图是同一视觉。本轮只认 `design/joe-snipaste-toolbar-target.png`。旧文件名不要再当反例引用。

---

## 2. 必用

按调用顺序。每个 skill 只取下面写明的切片，不要整包跑。

### 2.1 `design-md-workflow`（路由器，先开）

**为什么用：** Joe 点名的设计入口。规定：`DESIGN.md` 管样子；skill 管交互对不对、反馈准不准、动作好不好做。禁包见 §4.1。

**怎么用：**

1. 先读根目录 `DESIGN.md`（Operate、系统字体、桌面图钉、贴图光环）。**截图主路径 / 标注 / Don't 工具栏** 这几段本轮要改，未改之前不要拿它们挡目标图。
2. 允许的下游只开：Impeccable **具名命令**、design-engineering **单个节点**。GSAP / transitions.dev 关掉。
3. 不要给仓库装 Impeccable 自动 hook，不要默认跑整套 design-review。
4. 本轮 **要改** `DESIGN.md` 里截图遮罩、选区铬、标注条 token。改完，新 token 才是权威。贴图光环、网速槽、设置页节奏 **不动**。

### 2.2 `intended-vs-implemented`（三列：Joe / 文档 / 代码）

**为什么用：** 扫描器看不到「规格写禁止，Joe 说这就是产品」。缺口是意图差。旧 `SKILL-PICK-UX.md` 的三列已经作废。

design / coding 前写三列，没有两边证据就标「待查」：

| 意图（本次 Joe / 将写的规格） | 实现（先核对再改） | 听谁的 |
| --- | --- | --- |
| 热键 F1 | 代码默认 F1；规格和部分测试仍 Ctrl+Shift+F12 | **听 Joe。** 设置可改键；失败气泡沿用「热键 {键} 注册失败…」 |
| 松手出浅色图标工具栏 | `PinAfterCapture` 默认 false 已对；窗是文字按钮，图标集/顺序不对 | **听目标图。** 栏必须是图标，顺序对照附图 |
| 选区松手后的蓝虚线框 + 控制点 + 尺寸片 | 规格明确禁止；实现也没有 | **听目标图。** 这是松手后的 Snipaste 铬，不是「讨厌的变换框」。拖的过程仍可红描边；松手切到附图那种蓝框 |
| 主路径不再是松手即钉 | 规格默认 PinAndCopy；设置项还在 | 默认关自动钉。勾上「截完后自动钉成贴图」才跳过工具栏。不要删这个设置 |
| 标注四件套 vs 附图 13 键 | `AnnotationWindow` 只有矩形/箭头/文字/马赛克 + 复制/钉/取消 | 工具栏 **外观** 听附图全套。实现深度见 §2.5 distill：图标都在；手绘至少笔或马克笔一种能画 |
| 不逆向 Snipaste | 无二进制依赖 | 继续。WPF 自绘 Path，对照附图 |

### 2.3 `impeccable`（只开 Operate + 下列命令）

**为什么用：** 允许清单里唯一能管「松手后这块铬怎么收口」的设计 skill。必须 **Operate**：工具消失在任务里。成功标准是用过 Snipaste 的人松手后不愣。

禁止 `/document` 从当前文字条倒抽系统。禁止 new-work / concept-seed。

| 开 | 作用（本轮这一刀） | 不要做成 |
| --- | --- | --- |
| 模式 **Operate** | 截图 = 热键 → 选区 → 栏上点一下就结束。熟悉的 Qt 工具栏结构，不是新皮肤 | 作品集截图编辑器、玻璃岛、底部 Dock |
| `/shape`（压缩 brief，**不问卷**） | 五条内写清：F1；松手出栏；图标顺序；Esc；哪些 Pro 能力明确不做 | 再发现「用户是谁」。Joe 已钉死 |
| `/layout` | 栏贴在选区**下方**（下方不够则上方）；不挡图；图标等距、分组（绘制 \| 历史 \| 提交）；蓝虚线框与控制点贴选区边；尺寸片在框外上沿 | 把栏做成窗底通栏、FAB、右侧竖条、网页浮动岛 |
| `/typeset` | **只动尺寸片**：`235 × 172 px`（空格 + × + 空格 + `px`）。栏上无字。不要换 UI 字体 | 给图标配中文标签；换 Inter/Geist；clamp |
| `/distill` | 砍 OCR、滚动截图、GIF、取色完整面板、编号、双向箭头、高斯模糊全套。栏上图标仍按附图排，不要 distill 到又只剩四个字 | 把栏「极简」回文字按钮或右键菜单 |
| `/clarify` | 遮罩一句改成松手出工具栏；设置热键帮助改 F1；自动钉帮助改「默认先出栏，勾上才松手钉」；图标靠 tooltip 不靠按钮字 | 营销口吻；「AnnotateToolbarHWND」当主文案 |

**`/document` 本轮禁止当第一步。** 现码和旧 spec 都是反例。token 由 design **手改** `DESIGN.md`。

**`/polish` `/audit` 不作为开工命令。** 真机可点、规格已落地后再说。WPF 上 `detect.mjs` 无效。`/audit` 的 native 变体是 iOS/Android，不是 Win32。

**不要开：** `/bolder` `/quieter` `/delight` `/overdrive` `/animate` `/colorize` `/live` `/onboard` `/extract` `/init`。`/colorize` 会把浅栏改成品牌色或暗色「现代条」——附图是浅灰。`/adapt` 只在真机多 DPI 栏错位时由 coding 开。

`/harden` 留给 **coding 轮**：Esc 在文字输入中先退出输入框；撤销栈空则撤销禁用；存盘/剪贴板失败气泡；遮罩忙碌时 F1 不再开第二层。

craft-floor 只在 **开始改 UI 文件前** 读。本路由文件和纯规格阶段不要加载。

### 2.4 `design-engineering`（只读节点，不跑子代理全图）

**为什么用：** 判断「该不该动、会不会做成营销页、图标会不会换成 Lucide」。桌面工具只借原则，不借 CSS 配方。

| 节点 | 用来干什么 |
| --- | --- |
| `using-design-md` | YAML token 是契约。本轮 **提案并写入** 工具栏底色/描边/高度、图标尺寸、选区松手后描边色、控制点、尺寸片。改完才准 coding 硬编码这些数 |
| `pointing-beats-describing` | **目标 PNG 就是规格。** design 必须从左到右点名每一个图标，不要写「一排好看的工具」。对照文件：`design/joe-snipaste-toolbar-target.png` |
| `marketing-vs-product-ui` | 钉死：这是第 100 次截图。密度、系统字体、Snipaste 熟悉结构是对的 |
| `ai-default-tells` | 「工具栏很差」的修法 **不是** Lucide、Phosphor、Tabler、胶囊、暗色玻璃、红强调铺满。那些比文字条更不像 |
| `icon-systems` | **只取纪律，不取配表面板。** 单一描边家族、同一视口、光学居中、选中=红点或微填充（附图箭头上有红点）。**禁止**按该节点去「挑选一个 icon pack」。glyph 对照 PNG 自绘 WPF `Path`/`Geometry`，一份 stroke |
| `states-are-the-work` | 拖中 / 松手出栏 / 当前工具 / 撤销空 / 重做空 / 文字输入中 Esc / 整次 Esc / 自动钉开启跳过栏 / 栏贴不下翻到选区上方。每态都要在规格里有样子 |
| `hover-states-subtle` | 图标悬停浅底、按下略压。不要弹跳、不要 tooltip 动画 |
| `copy-voice` | tooltip 短词：矩形、箭头、铅笔、马克笔、马赛克、文字、橡皮、撤销、重做、取消、钉图、保存、复制。失败句沿用现有气泡，不要 Oops |
| `animation-decision-framework` | **不要加动效。** 栏随松手出现，即时。贴图仍禁止从 0 弹出 |
| `border-radius` | 栏跟随附图：浅底、细灰描边、小圆角（现码 `CornerRadius=4` 可作上限，以 PNG 为准）。选区直角。不要胶囊栏 |

**不要：** `ui-reviewer` 十一行网页 checklist、`motion-auditor`、Agentation 浏览器批注、从 58 个网站偷阴影。`design-md-consumer` 子代理不必开，节点自己读即可。

### 2.5 `ponytail`（full，design 与 coding 都开）

**为什么用：** 最大风险是做成「迷你 Snipaste Pro」，或反向做成又一次「去工具栏」。Joe 要的是 **那一条栏 + F1**。

对齐就停：

- F1 进入遮罩；悬停选窗 + 手拖保留（上一轮微信式选窗继续，不回退）。
- 松手：选区留在原地，浅色图标栏出现，蓝虚线框 + 尺寸片。
- 点复制 / 钉 / 存 / × 结束会话；Esc / 右键整次取消（不钉、不写剪贴板、不存盘），与旧 Esc 契约相同。
- 图标顺序对照附图。绘制类：形状、箭头、笔、马克笔、马赛克、文字、橡皮。历史：撤销、重做。提交：取消、钉、存、复制。
- 手绘：铅笔与马克笔在栏上都在；实现至少一种能画出线（另一种可先等同粗细/半透明笔，但按钮不能是空的）。
- 形状键外观带小三角；v1 按下 = 矩形。椭圆下拉不阻塞本轮，写可选。
- 「截完后自动钉成贴图」默认关；打开则跳过栏，走旧的松手即钉。

不要顺手做：OCR、滚动截图、GIF、取色面板、编号、双向箭头、高斯模糊、贴图分组、命令行、把栏做成可停靠 IDE。

### 2.6 `diagnosing-bugs`（coding 开工时开，design 不必）

**为什么用：** 「很差」看起来像审美，根因是 **规格禁止图标栏 → 实现改成文字条**。Skill 要求：先有一条能变红的检查，再改。不要先把按钮改成 Lucide 看看。

**本轮收口：**

- Debian 上能红能绿：默认热键显示名 `F1`；`PinAfterCapture` 默认 false；`OpenAnnotationAfterRegion(false, false)==true`；OverlayHint 含「工具栏」不含「松手钉图」；工具顺序常量与规格一致。
- 现成红件（路由时已存在，coding 必须一起修）：`CaptureUxTests.Overlay_hint_is_the_default_path_sentence` 仍要「松手钉图」；`SettingsJsonTests` 仍要 pin=true / Ctrl+Shift+F12。以 **新规格** 改测试，不要改代码去迁就旧测试。
- 不能在远程变绿的：真机松手后的栏是否像 PNG。不要拿 `dotnet test` 绿冒充 Joe 那张图已修。

### 2.7 `tdd`（coding 轮，三条 seam）

**为什么用：** 远程能证明的只有契约，不是截图像素。

1. 默认：热键 F1、无修饰键；`PinAfterCapture=false`；schema &lt; 3 迁到这两项（已有 schema 3 逻辑，测试要跟上）。
2. 松手决策：自动钉关 → 进工具栏；自动钉开且无 Shift → 不进工具栏；Esc 整次取消的纯函数/状态枚举（若 design 把状态机写下）。
3. 工具顺序：规格里的 13 键顺序有一份可断言的列表，XAML/枚举跟它走。

不要测私有 WPF 布局，不要为「像不像 PNG」写在 Linux 上恒绿的 UI 测试。不要为 `SetParent` / 任务栏再扩网。

### 2.8 `operating-coding-change`（真正改 `src/` 时的入口）

**为什么用：** 写代码的唯一入口。本文件不写代码；design 出规格后按它路由。

| 切片 | 路由 | 原因 |
| --- | --- | --- |
| 热键默认 F1、schema 3、设置帮助句 | **scoped** | 代码已大半在；补测试与文案 |
| 状态机：松手 → 工具栏（默认） | **scoped** | 现有 Capture 边界内；`PinAfterCapture` 已是开关 |
| `AnnotationWindow` 换成附图结构的图标栏 + 选区蓝框 | **scoped** | 同一标注窗的 chrome；不换捕获 API |
| 笔 / 马克笔 / 橡皮 / 撤销重做 | **scoped** | 画布已有矩形/箭头/文字/马赛克；补工具，不新进程 |
| `TaskbarFx.*` / 网速槽 / 贴图光环 | **不动** | 不是本轮。光环上一轮已做 |

改之前仍守：无管理员、无驱动、不逆向 Snipaste、不拷 ShareX。图标自己画。

---

## 3. 条件用（默认关）

| Skill | 何时才开 | 现在为什么关 |
| --- | --- | --- |
| `ask-questions-if-underspecified` | 出现互斥解释（例如「按 Snipaste」被理解成重写成 Qt） | Joe + PNG + F1 已够。蓝框是否要做：听 PNG，不必再问 |
| `impeccable /audit` `/polish` | 规格已落地、Windows 真机可点之后 | 开工缺口是契约推翻，不是间距扫描 |
| `impeccable /adapt` | 真机多 DPI 下栏跑到别的屏或挡图 | 先做选区下方 / 不够翻上 |
| `impeccable /critique` | design 交稿后 Joe 要打分 | 有目标图当 backlog 更便宜 |
| `impeccable /harden` | coding 轮 | design 不必做生产加固清单 |
| `research` | 公开行为文档缺了 | `research/REPORT.md` §2.1 已够。禁止再开竞品调研，禁止为「更像」去反编译 |
| `code-review` | coding diff 落地后 | 现在还没有本轮截图 diff |
| 安全扫描类 | 不需要 | 本轮是工具栏与热键，不是注入 |

---

## 4. 明确不用

这些会把本次活带歪，或平台不对。

### 4.1 Joe 工作流点名禁止

| Skill / 包 | 为什么不用 |
| --- | --- |
| Taste Skill、`design-taste-frontend`、Frontend Design、UI Brain、UI/UX Pro Max | 会换字体、圆角、配色、布局，做出「又一个 AI 站」。栏要像 Snipaste，不像 Linear |
| `qiaomu-design` | 三拨盘、落地页试衣间血统。本轮没有视觉方向可选 |

### 4.2 会换世界或把错误契约固化的 Impeccable 命令

| 命令 | 为什么不用 |
| --- | --- |
| `/document` | 会从「禁止工具栏」的 DESIGN.md / 文字条 XAML 倒生成规范，把误判写死 |
| `/init` + new-work / concept-seed | 世界已有（桌面图钉 / Operate）。这是截图主路径纠偏，不是新品牌 |
| `/bolder` `/delight` `/overdrive` `/colorize` | 把浅栏做成记忆点或暗色品牌条 |
| `/quieter` | 可能再把图标藏回右键菜单——那正是 Joe 骂的方向 |
| `/animate` `/live` | 浏览器挑元素、加动效。没有可点的网页 App |
| `/onboard` `/extract` | 不是首跑，不是抽设计系统 |

### 4.3 网页 / 动效实现包

| Skill | 为什么不用 |
| --- | --- |
| `gsap-core` 及 GSAP 全家桶 | JS/DOM。仓库是 WPF |
| `transitions-dev`、`transitions-polish` | CSS 过渡目录。松手出栏不应动画 |
| gstack `browse` / `qa` / `design-html` / `design-shotgun` / `design-consultation` | 浏览器产品流程 |

### 4.4 会重开已拍板问题的包

| Skill | 为什么不用 |
| --- | --- |
| `grilling` | 再烤「该不该出工具栏」是挡实现。误判已经纠正 |
| `create-plan`、`design`（design-doc-writer 长文）、`execute-plan` | 实现 brief 已在 `design/PROMPT-SNIPASTE-CAPTURE.md`。本文件只路由 skill |
| Hengmu 架构审计 / 方案顾问 / 质量门 | 不是架构轮。单进程、许可证、捕获 API 顺序已冻结 |
| `mobile-architecture-audit`、所有 `ios-*`、impeccable `audit.native` | 不是移动应用 |
| 上一轮 UX 的「去厚工具栏 / 默认 pin」指令 | **整包作废。** 不要把 `SKILL-PICK-UX.md` §2 当本轮约束 |
| 上一轮设置页 / 网速槽 token 再改一刀 | 那是另一轮。截图 token 以外不动 |

### 4.5 其它噪音

面试材料、海报/插图/PPT、简历、PM 画布、Obsidian、fuzzing、加密协议、安全扫描（semgrep / codeql）。与「F1 / 松手出 Snipaste 浅色图标栏」无关。

浏览器验收：本产品是 WPF 托盘工具，**没有** 可打开的本地网页。用户规则里的浏览器验收对本轮不适用；下一轮以 Windows 真机点检 + 热键/状态机测试为准。Debian 远程不能声称截图工具栏已绿。

---

## 5. 和仓库旧决定打架时听谁的

| 旧决定 | 本次 Joe | 怎么处理 |
| --- | --- | --- |
| `CAPTURE-PIN-SPEC` / `DESIGN.md`：红框工具栏是反例；主路径禁止图标栏 | 红框是目标；松手必须出栏 | **听 Joe。** 这两份里禁止工具栏的句子作废，由 design 重写 |
| `plans/SKILL-PICK-UX.md`、`PROMPT-UX-FIX-*`、`UX-ADD-GLOW-WINDOW.md`：「讨厌厚白图标栏」 | 同一张图是目标 | **听本次。** 那些文件留作历史，编码不要再执行 |
| 默认松手即钉、`PinAfterCapture=true` | 先出工具栏 | **听本次。** 设置项保留，默认关。代码已关，规格和测试要跟上 |
| 热键 Ctrl+Shift+F12（避 PrtScr） | F1 | **听 Joe。** F1 可能被别的软件占用：失败气泡 + 设置改键，不循环抢注 |
| ADR §1.2 标注四件套（矩形/箭头/文字/马赛克），不做铅笔等 | 附图含笔、马克笔、橡皮、撤销重做 | **外观听附图。** ADR「不是 Snipaste Pro」仍约束 **Pro 能力**（OCR/滚动/GIF）。绘制键按 PNG 排；手绘至少一种能用。HANDOFF 写一句：v1 标注集合按 2026-09-05 目标图扩，不是开 Pro |
| 选区无控制点、无蓝变换框 | 目标图有蓝虚线框和圆点 | **听目标图（松手后）。** 拖的过程保持红描边+无手柄，避免拖的时候像编辑器 |
| 尺寸片可省略 `px` | 目标图是 `235 × 172 px` | **听目标图** |
| `SETTINGS-PAGE-SPEC` §4.2 热键/自动钉帮助 | 与本次冲突 | design **要改这几句**。设置页分组结构不动 |
| 贴图向外光环、微信式选窗、Esc 整次取消 | 没说砍 | **保留。** 本轮不加标题栏，不改光环 |
| `security/AUDIT.md` 无驱动无管理员；不逆向 | 不变 | 不覆盖 |
| 调研：F1/C/F3 取色 | Joe 要 F1 截图 | **听 Joe。** 取色面板本轮不做；遮罩忙碌时 F1 不重入 |

许可证红线 **不覆盖**：禁止抄 Snipaste 二进制/资源、ShareX（GPL）、TrafficMonitor（Anti-996）。对照 PNG 自绘。

---

## 6. 下一步叫哪个 agent

**先 design，后 coding。不要跳过 design 直接改 XAML。**

原因：当前 `DESIGN.md` 与 `CAPTURE-PIN-SPEC.md` 仍禁止目标栏。coding 若先改，下一轮又会按旧 Don't 打回来。`design/SNIPASTE-TOOLBAR-SPEC.md` 尚不存在，图标顺序和尺寸没有契约。

### 6.1 design agent（现在就叫）

- **提示词：** 已有 `design/PROMPT-SNIPASTE-CAPTURE.md`。本路由是它的 skill 约束，不要另写长 PROMPT。
- **必读：** 本文件、`DESIGN.md`、`design/CAPTURE-PIN-SPEC.md`（当反例读）、`design/joe-snipaste-toolbar-target.png`、`design/SETTINGS-PAGE-SPEC.md` §4.2、`research/REPORT.md` §2.1（只取公开行为，不取「F1=取色」挡热键）。
- **必开 skill：** `design-md-workflow` → `intended-vs-implemented`（文档级三列即可）→ Impeccable Operate + `/shape`（五条）`/layout` `/typeset` `/distill` `/clarify` → design-engineering 上表节点（尤其 `pointing-beats-describing` + `icon-systems` 的纪律、不选 pack）→ `ponytail` full。
- **产出（已在 design PROMPT 写明）：**
  1. 重写 `design/CAPTURE-PIN-SPEC.md`：状态机改为 Snipaste 主路径（选区 → 工具栏 → 复制/钉/存/标注），F1，Esc；删掉「红框是反例」。
  2. 更新 `DESIGN.md` 截图/标注 token：浅色图标条、松手后选区描边、尺寸片。贴图光环 / 网速 / 设置节奏不动。Don't 改为禁止 **不像附图的** 文字条、禁止 Lucide 现代栏，而不是禁止浅色图标栏。
  3. 新建 `design/SNIPASTE-TOOLBAR-SPEC.md`：工具顺序对照附图（形状、箭头、笔、马克笔、马赛克、文字、橡皮 \| 撤销重做 \| 取消、钉、存、复制），尺寸、位置（贴选区下方）、禁用态、选中红点、tooltip。
  4. 顺手改 `design/SETTINGS-PAGE-SPEC.md` §4.2 热键帮助与自动钉帮助，避免设置页和截图规格再打架。
- **提示词要点（给 design，中文短句）：**
  - 不写业务代码，不改 `src/`。
  - 红框 = 目标。`plans/joe-hate-toolbar-ref.png` 不再当反例。
  - 完完全全按 Snipaste 的 **公开 UI 结构**，不是按旧 distill。
  - 不得逆向二进制；不得 `/document` / new-work。
  - 图标不选自网上 icon pack。
  - 蓝虚线框 + 控制点是松手后铬，写进规格，不要再标禁止。
  - MVP 可暂不做：OCR、滚动截图、GIF、取色完整面板。铅笔/马克笔至少一种手绘能用，栏上两键都要在。
  - 微信式选窗、贴图光环、Esc 整次取消保留。
  - 网速 / 任务栏效果 / 设置分组结构不重开。
- **完成标志：** 上面 1–3 个路径都在（§4.2 帮助已改更稳），且新规格能解释「F1 → 框选松手 → 附图那条浅色图标栏」。stdout `DONE`。

### 6.2 coding agent（design 规格齐了再叫）

- **提示词：** 本轮还没有单独的 coding PROMPT 文件。调度在 design 完成后写一份短 PROMPT，或直接把下面要点贴给 coding。不要沿用 `plans/PROMPT-UX-FIX-SNIPASTE.md` / `PROMPT-UX-FIX-ROUND2.md`（它们还在去工具栏）。
- **必读：** 新 `DESIGN.md`、新 `CAPTURE-PIN-SPEC.md`、新 `design/SNIPASTE-TOOLBAR-SPEC.md`、目标图、本文件 §2.6–2.8。
- **必开 skill：** `design-md-workflow`（按 **新** token）→ `intended-vs-implemented`（代码级三列）→ `diagnosing-bugs` → `tdd` 三条 seam → Impeccable `/harden`（Esc、禁用态、失败气泡、F1 忙碌）→ `ponytail` full → `operating-coding-change`（上表）。
- **提示词要点：**
  - 单进程；不逆向；不抄 ShareX。
  - 默认 F1；松手出栏；栏的图标/顺序/位置按 toolbar spec，不要中文按钮。
  - 自动钉仍是设置，默认关。
  - 修掉与新规格相反的测试，不要为了绿测把 Hint 改回「松手钉图」。
  - 不碰 `TaskbarFx.*`、网速槽、贴图光环算法。
  - 改完写 `plans/SNIPASTE-CAPTURE-HANDOFF.md` 大白话。真机路径 suite-app-fresh。缺 Native 仍可跑截图。
  - 不 `git push`。
- **若开工时 spec 还不在：** 等 design。不要用旧 CAPTURE-PIN-SPEC 开工。

### 6.3 给调度的最短顺序

1. 读本文件。
2. 叫 **design**，约束见 §6.1，提示词用 `design/PROMPT-SNIPASTE-CAPTURE.md`。
3. 确认 `CAPTURE-PIN-SPEC.md`、`DESIGN.md`、`design/SNIPASTE-TOOLBAR-SPEC.md` 已把红框当目标，且 F1 / 松手出栏写死。
4. 叫 **coding**，约束见 §6.2。
5. coding 写 `plans/SNIPASTE-CAPTURE-HANDOFF.md`。不 `git push`。

**本轮已完成：** 只产出本文件。没有改业务代码。
