# 本次设置页 + 任务栏网速该用哪些 skill

- **日期：** 2026-09-05
- **任务：** 设置页不要丑；网速补 TrafficMonitor 级外观设置；任务栏网速字更大、垂直居中、别底下空一截
- **本文件只做 skill 路由。** 不写业务代码、不改 `src/`、不重开调研、不换视觉世界。
- **下一轮入口：** 先 **design**（`design/PROMPT-SETTINGS-NETSPEED.md`），再 **coding**（`plans/PROMPT-POLISH-SETTINGS-NETSPEED.md`）

Joe 要的不是「换一套皮」，是三件能指着看的事：

1. 设置页分组清楚、控件对齐，仍是系统表单。
2. 网速设置能调字号、上/下行颜色、显隐、背景透明或着色、网卡、钉栏/悬浮。
3. 任务栏网速方框：字够大、垂直居中、边框克制或可关。反例：`design/joe-netspeed-ugly.png`。

---

## 1. 这次是什么活

| 判定 | 结论 |
| --- | --- |
| 产品面 | WPF 托盘常驻小工具。设置页 + 任务栏槽，不是落地页 |
| Impeccable 模式 | **Operate**：熟悉感是功能。陌生装饰是失败 |
| 视觉权威 | 根目录 **已有** `DESIGN.md`。字体/色/圆角/间距以它为准。**但** 当前 token 把丑写死了（见 §1.1），本轮 **允许改这些 token**，不是发明第二套世界 |
| 缺口类型 | （A）设置页节奏差；（B）`NetSpeedSettings` 缺外观字段；（C）任务栏槽字小、顶对齐、底下空 |
| 许可证 | 学 TrafficMonitor **行为与设置项**。不抄源码（Anti-996）。颜色选择器用系统对话框 |

Joe 的设计工作流（`design-md-workflow`）管路由：先读 `DESIGN.md`，再 **按需** 调允许的 skill。禁止 Taste / Frontend Design / UI Brain / UI/UX Pro Max。技能建议若跟已拍板的 Operate 契约打，契约赢——**唯一例外**是 Joe 这张丑图直接打脸当前 `rate-taskbar` / `netspeed-taskbar` token，那些 token 要改。

上一轮 `plans/SKILL-PICK-UX.md` 写「仓库没有 DESIGN.md」。那是当时。**现在有。** 不要再发明 Inter / 圆角卡 / Hero。也不要把现有丑 widget 用 `/document` 固化进系统。

### 1.1 编码前就能钉死的证据（路由用，不当成已修）

三方对不上时听 **本次 Joe**，不要听旧 token。

| 面 | 现在怎样 |
| --- | --- |
| Joe | 字小、偏上、底下空、方框不好看（`design/joe-netspeed-ugly.png`：双行顶对齐，底下一大块空，1px 框） |
| `DESIGN.md` | `typography.rate-taskbar` **11px**；`spacing.taskbar-pad` **6px 2px**；`components.netspeed-taskbar` **宽 148 / 高 40**、直角、透明底 |
| 代码 | `NetSpeedWindow.ApplyEmbeddedChrome`：嵌入时字号 11、Padding `(6,2)`、**Height 固定 40 DIP**、`StackPanel` 默认顶对齐、边框仍 1px。槽计算本身用任务栏客户区高（`TaskbarEmbed.ComputeSlot`），窗体高度却锁 40 |
| 设置契约 | `NetSpeedSettings` 只有 Visible / EmbedInTaskbar / 网卡 / Left / Top。没有字号、颜色、显隐上/下行、背景 |
| 设置页 | `SettingsWindow.xaml` 一条 `StackPanel` 把开机/网速/热键/截图/任务栏效果/主题顺排。有分组标题，缺分组节奏。`design/SETTINGS-PAGE-SPEC.md` **还不存在** |

结论：丑图不是「没按 DESIGN.md 做」，是 **按 DESIGN.md 做出来的**。design 必须先改 token 和规格；coding 再跟新规格。不要 coding 先改、design 后补，也不要 coding 死守 11px/40。

---

## 2. 必用

按调用顺序。每个 skill 只取下面写明的切片，不要整包跑。

### 2.1 `design-md-workflow`（路由器，先开）

**为什么用：** Joe 点名的设计入口。规定：`DESIGN.md` 管样子；skill 管交互对不对、反馈准不准、动作好不好做。禁包见 §4.1。

**怎么用：**

1. 先读根目录 `DESIGN.md`（Operate、系统字体、设置单列、任务栏网速直角透明）。
2. 允许的下游只开：Impeccable **具名命令**、design-engineering **单个节点**。GSAP / transitions.dev 关掉。
3. 不要给仓库装 Impeccable 自动 hook，不要默认跑整套 design-review。
4. 本轮 **要改** `DESIGN.md` 里网速槽与设置页密度相关 token。改完，新 token 才是权威。未改之前，不要拿 11px/40 当「不能动」。

### 2.2 `intended-vs-implemented`（三列：Joe / 文档 / 代码）

**为什么用：** 扫描器看不到「规格写了 11px，Joe 说太小」。缺口是意图差，不是空指针。

编码/出规格前写三列，没有两边证据就标「待查」：

| 意图（Joe / 将写的规格） | 实现（先核对再改） | 听谁的 |
| --- | --- | --- |
| 设置页不丑：分组清晰、间距对齐 | `SettingsWindow.xaml` 单列 StackPanel，无 `SETTINGS-PAGE-SPEC` | 听 Joe。规格里定分组：常规 / 截图 / 网速 / 任务栏效果 / 关于。仍单列，不要侧栏三卡 |
| TrafficMonitor 级外观项 | `NetSpeedSettings` 无字号/色/显隐行/背景 | 听本次硬需求。学项，不抄 MFC 对话框 |
| 任务栏字更大、垂直居中、底别空 | DESIGN + 代码都是 11px + 顶对齐 + 高 40 | **听丑图。** 改 token：字号、行高、垂直居中、高度跟任务栏客户区，不要只加 padding 把空挪到上面 |
| 边框更克制或可关 | 嵌入仍 `BorderThickness=1` | 默认无框或极淡；可关。不要品牌描边 |
| 不抄 TrafficMonitor | 嵌入走自己的 `SetParent` | 继续。外观设置自己建模 |

### 2.3 `impeccable`（只开 Operate + 下列命令）

**为什么用：** 允许清单里唯一能管「设置怎么排、槽里字怎么坐」的设计 skill。必须 **Operate**：工具消失在任务里。

这是 **既有表面的 refinement**，不是 new-work。禁止 concept-seed、禁止五套视觉方向、禁止 `/document` 从当前丑代码倒抽系统。

| 开 | 作用（本轮这一刀） | 不要做成 |
| --- | --- | --- |
| 模式 **Operate** | 设置=开关档位；槽=任务栏通知区密度 | 作品集设置中心、侧栏、Hero |
| `/shape`（压缩 brief，**不问卷**） | 五条内写清：设置 IA 分组；网速外观必选项；槽垂直居中成功标准；哪些 TM 能力明确不做 | 再发现「用户是谁」。Joe 已钉死 |
| `/layout` | 设置：组内紧、组间松、标签/控件对齐。槽：内容在任务栏高度里 **光学垂直居中**，高度跟客户区，禁止固定 40 DIP 顶对齐 | 三列功能卡、Bento、网页栅格 |
| `/typeset` | 只动速率角色：任务栏字号从 11 提到能扫的档（建议规格里给默认 + 用户可调范围）；等宽、行高吃满槽；悬浮仍可与槽不同档 | 换 Inter/Geist；clamp；把 Segoe 换成展示字 |
| `/distill` | TM 只收外观项。砍 CPU/GPU/温度/皮肤/历史流量/插件 | 把设置页「极简」到砍掉恢复手段或网卡选择 |
| `/clarify` | 新控件人话：字号、上行色、下行色、显示上行、显示下行、背景透明/着色、钉栏。失败句沿用现有「无法钉到任务栏…」 | 营销口吻；HWND / DIP / IfIndex 当主文案 |
| `/harden`（coding 轮） | 旧 `settings.json` 缺新字段要有默认；自定义颜色对比度不够时仍可读；只显一行时槽仍居中 | 把 CSS clamp、RTL、i18n 大礼包搬进 WPF |

**`/document` 本轮禁止当第一步。** 现码是反例。token 由 design **手改** `DESIGN.md`，不是从 XAML 倒生成。

**`/polish` `/audit` 不作为开工命令。** 真机可点、规格已落地后再说。WPF 上 `detect.mjs` 无效（它读 HTML/CSS）。`/audit` 的 native 变体是 iOS/Android，不是 Win32。

**不要开：** `/bolder` `/quieter` `/delight` `/overdrive` `/animate` `/colorize` `/live` `/onboard` `/extract` `/init`。`/adapt` 只在多 DPI 槽高仍错时由 coding 开，不要把网页响应式搬来。

craft-floor 只在 **开始改 UI 文件前** 读。本路由文件和纯规格阶段不要加载。

### 2.4 `design-engineering`（只读节点，不跑子代理全图）

**为什么用：** 判断「该不该动、会不会做成营销页」。桌面工具只借原则，不借 CSS 配方。

| 节点 | 用来干什么 |
| --- | --- |
| `using-design-md` | YAML token 是契约。本轮 **提案并写入** 新的 `rate-taskbar` 字号、行高、槽高、padding、设置分组间距。改完才准 coding 硬编码这些数 |
| `marketing-vs-product-ui` | 钉死：这是第 100 次打开的设置。密度、系统控件、Segoe/雅黑是对的 |
| `ai-default-tells` | 「设置丑」的修法 **不是** Inter、紫渐变、三卡片、Lucide、大圆角。那些是更丑 |
| `cards-design` | **反用：** 设置分组用标题 + 间距（必要时一根发丝分隔），不要 Group 卡片套卡片 |
| `forms-validation` | 只取：标签常驻、帮助句不复述标题、点「应用」才写入（现规格已禁拖滑块预览任务栏）。不要搬「输入中绿勾 + blur 红框」网页表单套路 |
| `copy-voice` | 开关说结果（「显示下行」），不要说实现（「绑定 DownText.Visibility」） |
| `animation-decision-framework` | **不要加动效。** 网速 1 秒一跳，不要数字滚动；设置打开不要入场 |
| `border-radius` | 槽继续 `{rounded.none}`。悬浮 6px 是唯一小块。不要给任务栏网速加圆角「好看」 |
| `states-are-the-work` | 槽：双行 / 只下行 / 只上行 / 断网卡 `↓ —` / 嵌入失败回退悬浮。每态都要在规格里有样子 |

**不要：** `ui-reviewer` 十一行网页 checklist、`motion-auditor`、Agentation 浏览器批注、从 58 个网站偷阴影。`design-md-consumer` 子代理不必开，节点自己读即可。

### 2.5 `ponytail`（full，design 与 coding 都开）

**为什么用：** 最大风险是做成「迷你 TrafficMonitor 标准版」。Joe 只要外观可调 + 槽好看。

对齐就停：

- 设置：单列、系统控件、分组标题、应用/关闭。
- 网速外观必选：字号、上行色、下行色（可同一默认）、显示上行、显示下行、背景透明或着色、网卡、钉栏/悬浮。
- 可选（规格里写可选，默认双行、常规字重即可）：单行/双行、加粗。
- 槽：字够大、垂直居中、高度 = 任务栏客户区高、默认无框或极淡。

不要顺手做：CPU/内存/GPU/温度、历史流量、皮肤市场、插件、WinRing0、副屏任务栏网速、实时预览拖滑块改系统任务栏。

### 2.6 `diagnosing-bugs`（coding 开工时开，design 不必）

**为什么用：** 「偏上、底下空」看起来像审美，根因是布局度量。Skill 要求：先有一条能变红的检查，再改。

**本轮收口：**

- Debian 上能红能绿：槽内容盒度量纯函数（任务栏客户区高 + 字号 + 行数 → 窗高、内容垂直偏移）。现在硬编码 40 DIP 就是红。
- 旧 JSON 缺新字段：反序列化后默认值测试（已有 `SettingsJsonTests` 可扩）。
- 不能在远程变绿的：真任务栏观感。不要拿 `dotnet test` 绿冒充 Joe 那张图已修。

不要跳到「先把 FontSize 改成 14 看看」。先改规格里的高度/对齐契约，再改一处 chrome。

### 2.7 `tdd`（两条 seam，coding 轮）

**为什么用：** 远程能证明的只有契约，不是截图。

1. `NetSpeedSettings` 新字段 roundtrip + 缺字段默认（字号、颜色、显隐、背景）。旧 schema 2 文件打开不炸、不丢钉栏。
2. 槽垂直度量纯函数（若 design 把公式写下）。不要测私有 WPF 布局，不要为「好看」写在 Linux 上恒绿的 UI 测试。

### 2.8 `operating-coding-change`（真正改 `src/` 时的入口）

**为什么用：** 写代码的唯一入口。本文件不写代码；design 出规格后按它路由。

| 切片 | 路由 | 原因 |
| --- | --- | --- |
| 设置页 XAML 分组/间距/对齐 | **scoped** | 现有窗体内排版，不换导航模型 |
| 槽字号、垂直居中、高度跟任务栏、边框 | **scoped** | `NetSpeedWindow` + 已有 `TaskbarEmbed.ComputeSlot`；根因是窗高/对齐不是嵌入 API |
| `NetSpeedSettings` + `settings.json` 新字段 | **scoped**（加字段 + 默认；旧文件仍能读） | 不是换格式。仅当要 bump `CurrentSchemaVersion` 并做不兼容迁移时才升 **governed**。优先：缺省字段 = 新默认，不强制 bump |
| `TaskbarFx.*` / Native 注入 | **不动** | 任务栏透明 ≠ 网速外观。两开关继续互不隐含 |

改之前仍守：无管理员、无驱动、不拷 TrafficMonitor 源码。颜色用系统 `ColorDialog`，不要自绘取色盘。

---

## 3. 条件用（默认关）

| Skill | 何时才开 | 现在为什么关 |
| --- | --- | --- |
| `ask-questions-if-underspecified` | 出现互斥解释（例如「美化设置」被理解成 WinUI 3 重做） | Joe 三项 + 必选设置项已够。单行/双行、加粗标可选即可，不必再问 |
| `impeccable /audit` `/polish` | 规格已落地、Windows 真机可点之后 | 开工缺口是 token 与字段，不是间距扫描。且 detector 不读 XAML |
| `impeccable /adapt` | 真机多 DPI 槽高仍错 | 先做客户区高度 + DIP 换算；不要网页断点 |
| `impeccable /critique` | design 交稿后 Joe 要打分 | 本路由不跑评分；有丑图当 backlog 更便宜 |
| `research` | 对标设置项文档缺了 | `research/REPORT.md` §2.3 已列应对齐/不应对齐。禁止再开竞品调研 |
| `code-review` | coding diff 落地后 | 现在还没有美化 diff |
| `intended-vs-implemented` 安全边界切片 | 不需要 | 这次不是权限/租户；只用意图差方法 |

---

## 4. 明确不用

这些会把本次活带歪，或平台不对。

### 4.1 Joe 工作流点名禁止

| Skill / 包 | 为什么不用 |
| --- | --- |
| Taste Skill、`design-taste-frontend`、Frontend Design、UI Brain、UI/UX Pro Max | 会换字体、圆角、配色、布局，做出「又一个 AI 站」。设置页要像系统选项，不像 Linear |
| `qiaomu-design` | 三拨盘、落地页试衣间血统。本轮没有视觉方向可选 |

### 4.2 会换世界或把丑固化的 Impeccable 命令

| 命令 | 为什么不用 |
| --- | --- |
| `/document` | 会从当前 11px/顶对齐 倒生成 DESIGN.md，把反例写成规范 |
| `/init` + new-work / concept-seed | 世界已有（桌面图钉 / Operate）。设置页是旧表面加字段，不是新品牌 |
| `/bolder` `/delight` `/overdrive` `/colorize` | 把托盘工具做成记忆点 |
| `/animate` `/live` | 浏览器挑元素、加动效。没有可点的网页 App；网速也不该动画 |
| `/onboard` `/extract` | 不是首跑，不是抽设计系统 |

### 4.3 网页 / 动效实现包

| Skill | 为什么不用 |
| --- | --- |
| `gsap-core` 及 GSAP 全家桶 | JS/DOM。仓库是 WPF |
| `transitions-dev`、`transitions-polish` | CSS 过渡目录。设置打开、数字刷新都不应动画 |
| gstack `browse` / `qa` / `design-html` / `design-shotgun` / `design-consultation` | 浏览器产品流程 |

### 4.4 会重开已拍板问题的包

| Skill | 为什么不用 |
| --- | --- |
| `grilling` | 再烤「要不要做成 TrafficMonitor 全功能」是挡实现。v1 不做硬件监控已在调研写死 |
| `create-plan`、`design`（design-doc-writer 长文）、`execute-plan` | 实现 brief 已在 `design/PROMPT-SETTINGS-NETSPEED.md` 与 `plans/PROMPT-POLISH-SETTINGS-NETSPEED.md`。本文件只路由 skill |
| Hengmu 架构审计 / 方案顾问 / 质量门 | 不是架构轮。单进程、许可证、嵌入失败回退已冻结 |
| `mobile-architecture-audit`、所有 `ios-*`、impeccable `audit.native` | 不是移动应用 |
| 上一轮 UX 的 DPI→DIP 贴图测试扩网 | 那是截图 1:1。本轮 seam 是设置字段 + 槽度量 |

### 4.5 其它噪音

面试材料、海报/插图/PPT、简历、PM 画布、Obsidian、fuzzing、加密协议、安全扫描（semgrep / codeql）。与「设置页分组 / 网速外观项 / 槽垂直居中」无关。

浏览器验收：本产品是 WPF 托盘工具，**没有** 可打开的本地网页。用户规则里的浏览器验收对本轮不适用；下一轮以 Windows 真机点检 + JSON/度量测试为准。Debian 远程不能声称设置页/任务栏已绿。

---

## 5. 和仓库旧决定打架时听谁的

| 旧决定 | 本次 Joe | 怎么处理 |
| --- | --- | --- |
| `DESIGN.md` 任务栏速率 11px、槽高 40、pad 6×2 | 字小、偏上、底下空 | **听 Joe。** design 改这些 token 和 `NETSPEED-TASKBAR-SPEC` §2.2。截图/贴图 token 不动 |
| `DESIGN.md` 设置：单列、页边 16、组距 12、系统控件、不要三卡 | 整个设置页丑 | **结构听 DESIGN.md，节奏听 Joe。** 加分组分隔与控件对齐，不改成营销页。允许微调 `settings-section` / `settings-page`，不许加侧栏 |
| `NETSPEED-TASKBAR-SPEC` 网速设置只有显示/钉栏/网卡 | 要 TM 级外观项 | **听本次。** 扩 §3，并反映到 `NetSpeedSettings`。任务栏效果那一节不要趁机重开 |
| ADR / 调研：不抄 TrafficMonitor；v1 不做 CPU/温度 | 只要外观设置 | 继续。设置项对齐 README 的网速外观，不对齐标准版硬件 |
| 上一轮 UX：网速默认 `SetParent` | 仍要钉栏，另外要好看 | 嵌入逻辑已有。本轮不重做嵌入，只做 chrome + 设置字段 |
| `security/AUDIT.md` 无驱动无管理员 | 不变 | 不覆盖 |

许可证红线 **不覆盖**：禁止抄 TrafficMonitor（Anti-996）、Snipaste、ShareX。字号/颜色/透明是产品设置，自己建模。

---

## 6. 下一步叫哪个 agent

**先 design，后 coding。不要跳过 design 直接改 XAML。**

原因：当前 `DESIGN.md` 与代码一致，且一致地丑。coding 若先改，下一轮又会按旧 token 打回来。`design/SETTINGS-PAGE-SPEC.md` 尚不存在，设置 IA 没有契约。

### 6.1 design agent（现在就叫）

- **提示词：** 已有 `design/PROMPT-SETTINGS-NETSPEED.md`。本路由是它的 skill 约束，不要另写长 PROMPT。
- **必读：** 本文件、`DESIGN.md`、`design/NETSPEED-TASKBAR-SPEC.md`、`design/joe-netspeed-ugly.png`、`research/REPORT.md` §2.3（只取应对齐/不应对齐）。
- **必开 skill：** `design-md-workflow` → `intended-vs-implemented`（文档级三列即可）→ Impeccable Operate + `/shape`（五条）`/layout` `/typeset` `/distill` `/clarify` → design-engineering 上表节点 → `ponytail` full。
- **产出（已在 design PROMPT 写明）：**
  1. 更新 `DESIGN.md`：设置密度；`rate-taskbar` 字号/行高；槽 padding；槽高度 = 任务栏客户区（不要 40 DIP）；垂直居中；默认边框。
  2. 更新 `design/NETSPEED-TASKBAR-SPEC.md`：列出 TM 对齐设置项（至少：字号、上/下行颜色或统一文字色、是否显示上/下行、背景透明/不透明、网卡、钉栏/悬浮；可选：双行/单行、加粗）。
  3. 新建 `design/SETTINGS-PAGE-SPEC.md`：分组 常规 / 截图 / 网速 / 任务栏效果 / 关于。Operate，不要营销页。
- **提示词要点：** 中文短句；不写业务代码；不跑 `/document` / new-work；不抄 TM 源码；截图/贴图视觉不动；任务栏效果文案已有的不要重写。
- **完成标志：** 三个路径都在，且新 token 能解释「字更大、垂直居中、底不空」。stdout `DONE`。

### 6.2 coding agent（design 三个文件齐了再叫）

- **提示词：** 已有 `plans/PROMPT-POLISH-SETTINGS-NETSPEED.md`。
- **必读：** 新 `DESIGN.md`、新/更新的两份 spec、反例图、本文件 §2.6–2.8。
- **必开 skill：** `design-md-workflow`（按 **新** token）→ `diagnosing-bugs` → `tdd` 两条 seam → Impeccable `/harden`（旧 JSON 默认、对比度）→ `ponytail` full → `operating-coding-change`（上表）。
- **提示词要点：** 单进程；不抄 TM；设置项按 spec 加到 `NetSpeedSettings` + 设置页网速分组；槽高度跟客户区、内容垂直居中、字号用新默认且可被设置覆盖；改完 `plans/POLISH-HANDOFF.md` 大白话。真机路径 suite-app-fresh。缺 Native 仍可跑。
- **若开工时 spec 还不在：** 按该 PROMPT「等最多 3 分钟再读；仍无则按硬需求实现并在 HANDOFF 注明规格后补」——这是 fallback，不是跳过 design 的理由。调度上仍应先等 design。

### 6.3 给调度的最短顺序

1. 读本文件。
2. 叫 **design**，约束见 §6.1。
3. 确认 `DESIGN.md`、`design/NETSPEED-TASKBAR-SPEC.md`、`design/SETTINGS-PAGE-SPEC.md` 已反映丑图与 TM 项。
4. 叫 **coding**，约束见 §6.2。
5. coding 写 `plans/POLISH-HANDOFF.md`。不 `git push`。

**本轮已完成：** 只产出本文件。没有改业务代码。

## 追加（Joe 2026-09-05 13:50）

托盘右键必须能切系统深浅色，不必进设置。实现已有（`TrayIconService` 首项 → `ToggleTheme`）；文案用「切换系统深浅色」。design/coding 规格把它定为**主入口**，设置页按钮为次要。
