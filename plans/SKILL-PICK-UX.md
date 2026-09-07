# 本次 UX 该用哪些 skill

- **日期：** 2026-09-05
- **任务：** Joe Windows 合集 — 截图对齐 Snipaste（直接 pin、1:1、Esc），网速钉任务栏
- **本文件只做 skill 路由。** 不写业务代码、不改 `src/`、不重开调研、不重画视觉世界。
- **下一轮编码入口：** `plans/PROMPT-UX-FIX-SNIPASTE.md`

Joe 要的不是「更好看」，是「用起来像那两款工具」：

1. 截完立刻钉图，像素 1:1，不好就 Esc。
2. 网速钉在任务栏上，不要只靠桌面悬浮窗。

---

## 1. 这次是什么活

| 判定 | 结论 |
| --- | --- |
| 产品面 | 托盘常驻小工具（WPF），不是落地页、不是仪表盘重设计 |
| Impeccable 模式 | **Operate**：界面服务于任务；熟悉感是功能，陌生装饰是失败 |
| 视觉权威 | 仓库 **没有** `DESIGN.md` / `PRODUCT.md`。不要现编一套网页设计系统。权威是：现有 WPF 托盘/设置/贴图 chrome + 调研里 Snipaste / TrafficMonitor **行为**（`research/REPORT.md` §2.1、§2.3） |
| 缺口类型 | 意图已写清，实现偏了（标注挡路、默认不 pin、DPI 把贴图放大、网速没 `SetParent`） |
| 许可证 | 学行为 + Microsoft 文档 API。不抄 Snipaste / ShareX / TrafficMonitor 源码 |

Joe 的设计工作流（`design-md-workflow`）管路由：先读主题契约，再 **按需** 调允许的 skill，禁止用会接管审美的大礼包。本仓库没有 DESIGN.md，主题契约 = 现有 WPF 外观 + 对标行为。技能建议若跟这条打，这条赢。

---

## 2. 必用

按调用顺序。每个 skill 只取下面写明的切片，不要整包跑。

### 2.1 `design-md-workflow`（路由器，先开）

**为什么用：** 这是 Joe 的设计入口。它规定：DESIGN.md 管样子，skill 管交互对不对、反馈准不准、动作好不好做。禁止 Taste / Frontend Design / UI Brain / UI/UX Pro Max 一类会换字体换圆角换配色的包。

**怎么用：**

1. 确认没有 DESIGN.md → **不要发明** Inter/圆角卡片/营销 Hero。
2. 允许的下游只开：Impeccable **具名命令**、design-engineering **单个节点**。GSAP / transitions.dev 本轮关掉（见 §4）。
3. 不要给仓库装 Impeccable 自动 hook，不要默认跑整套 design-review。

### 2.2 `intended-vs-implemented`（先对齐「该怎样」和「代码怎样」）

**为什么用：** 这次的 bug 不是扫描器能扫出来的。文档说一套，代码做另一套。

| 意图 | 实现（本轮观察到的，编码前再核对） |
| --- | --- |
| 截完直接 pin | `CaptureSettings.PinAfterCapture` 默认 `false`；标注窗 Enter 只复制，Ctrl+Enter 才钉 |
| pin 1:1 | `PinWindow` 有 `Zoom`，Loaded 时没有按像素宽高 ÷ DPI 设 DIP；高 DPI 会巨大 |
| Esc 干净退出 | 选区有 Esc；标注 Esc 关窗；贴图 Esc 关窗。要对齐「各阶段取消、不留遮罩、不误 pin」 |
| 网速钉任务栏 | `NetSpeedWindow` 只是可拖悬浮窗；`NetSpeedSettings` 只有 Visible / 网卡 / Left / Top，没有嵌入开关 |
| ADR C4：v1 不做 `SetParent` | **被本次 Joe 口头覆盖。** 见 §5。不要假装 ADR 仍禁止嵌入 |

编码前用这个方法写三列：意图原话、文件:行、该不该改。没有两边证据就标「待查」，不当成 finding。

### 2.3 `diagnosing-bugs`（PROMPT 已点名，修之前开）

**为什么用：** 已有线索（默认 false、标注挡路、DPI、没 SetParent），但线索不是根因证明。Skill 要求：先有一条能变红的命令，再假设、再改。

**怎么用（本轮收口）：**

- 能在 Debian 上红的：DPI 像素 → DIP 尺寸纯函数测试（PROMPT 已要求）。
- 不能在远程变绿的：真截屏、真 `SetParent`、真任务栏。不要拿「远程 `dotnet build` 过了」当 UX 已修。
- 不要跳到 Phase 5 直接改。Win11 任务栏嵌入失败必须是可观察状态（提示 + 回退悬浮），不是静默假成功。

### 2.4 `impeccable`（只开 Operate + 三个命令）

**为什么用：** 允许清单里唯一能管「交互怎么收口」的设计 skill。必须走 **Operate**：工具消失在任务里，不要做记忆点、不要做品牌秀。

| 开 | 作用 | 不要做成 |
| --- | --- | --- |
| 模式 **Operate** | 对标 Snipaste/TrafficMonitor 的熟悉手感 | 新视觉世界、`/document` 现写网页 DESIGN.md |
| `/distill` | 砍步骤：框选确认 → 立刻 1:1 pin + 剪贴板；标注改为可选，不是默认闸门 | 把设置页也「极简」到砍掉恢复手段 |
| `/harden` | Esc 各阶段取消；`SetParent` 失败可见并回退悬浮；高 DPI / 多屏贴图尺寸 | 把 CSS clamp、RTL、i18n 大礼包搬进 WPF |
| `/clarify` | 嵌入失败、回退悬浮、设置里「任务栏 / 仅悬浮」的人话 | 营销口吻、内部 HWND 术语当主文案 |

**`/shape` 本轮不必开。** Joe 已经把成功标准钉死（直接 pin、1:1、Esc、钉任务栏）。再跑 discovery interview 是挡路。若编码前要写一页 brief，最多五条 bullet，不要问卷。

**不要开：** `/bolder` `/quieter` `/delight` `/overdrive` `/animate` `/colorize` `/typeset` `/layout` `/live` `/onboard` `/extract`。这些会把托盘工具做成作品集。`/audit` `/polish` 留给改完后的视觉 QA，不是本轮开工命令。

### 2.5 `design-engineering`（只读节点，不跑子代理全图）

**为什么用：** 判断「该不该动、状态全不全、文案像不像产品」。本仓库是桌面工具，只借原则，不借 CSS 配方。

| 节点 | 用来干什么 |
| --- | --- |
| `marketing-vs-product-ui` | 钉死：这是产品 UI。密度、系统字体、标准控件是对的 |
| `states-are-the-work` | 选区 / 标注可选 / pin / Esc 取消 / 嵌入失败 / 回退悬浮，每个状态都要能指给 Joe 看 |
| `animation-decision-framework` | 结论先写上：**不要加动效。** 截图、Esc、钉图是一天上百次的动作 |
| `copy-voice` | 失败提示要说清发生了什么、现在变成悬浮、去哪改回来 |

**不要：** `ui-reviewer` 十一行网页 checklist、`motion-auditor`、Agentation 浏览器批注、从 58 个网站偷阴影栈。没有 DESIGN.md 时，`using-design-md` 无对象，不要补一个假的。

### 2.6 `ponytail`（full）

**为什么用：** 最大风险是做成「迷你 Snipaste Pro」。Joe 只要三条交互 + 网速进任务栏。

对齐就停：

- 默认松手/确认后立刻 pin，Zoom=1，尺寸 = 像素 ÷ DPI。
- Esc 退出当前阶段，不留窗。
- 网速默认 `SetParent(Shell_TrayWnd)`；失败提示并悬浮；设置可切回仅悬浮。

不要顺手做：OCR、滚动截图、强制标注、贴图分组、皮肤、完美躲开 Win11 小组件、CPU/GPU、把网速塞进 TaskbarFx。

Win11 占满/重叠是调研里的已知死角。失败回退就是产品，不是还没做完。

### 2.7 `tdd`（一条 seam）

**为什么用：** PROMPT 要求「DPI→DIP 尺寸纯函数测试」。这是远程能红能绿的唯一硬验收。

**怎么用：** 只测公开换算（像素宽高 + DPI → DIP 宽高，Zoom=1）。不要测私有 WPF 布局，不要为 `SetParent` 写在 Linux 上恒绿的假测试。

未跟 Joe 确认的 seam 不要扩测试网。标注可选、Esc 取消以手工点检 + 状态机可读为主。

### 2.8 `operating-coding-change`（真正改 `src/` 时的入口）

**为什么用：** 这是写代码的唯一入口 skill。本文件不写代码；下一轮按它路由。

| 切片 | 路由 | 原因 |
| --- | --- | --- |
| 默认 `PinAfterCapture=true`、跳过强制标注、Esc | **scoped** | 现有截图/贴图边界内的行为修复 |
| Pin 窗 1:1 DIP | **scoped** | 纯函数 + 设宽高；带测试 |
| 网速 `SetParent` 进 `Shell_TrayWnd` | **governed** | 碰 explorer 窗口树、Win11 已知死角、失败必须可见；不是注入，但仍是 Shell 宿主 |
| TaskbarFx.Host / Native | **不动** | PROMPT 写明避开；那是任务栏透明，不是网速嵌入 |

改之前读 `security/AUDIT.md` 的约束精神：无管理员、无驱动、不拷 GPL/Anti-996、注入代码禁止进 Suite。`SetParent` 不是 TAP/SWCA，不要为了嵌入去加载 explorer DLL。

---

## 3. 条件用（默认关）

| Skill | 何时才开 | 现在为什么关 |
| --- | --- | --- |
| `ask-questions-if-underspecified` | 出现互斥解释（例如「钉任务栏」被理解成改 TaskbarFx 透明度） | Joe 三条已经够清楚。嵌入失败路径已规定回退。ADR 覆盖见 §5，不必再问「要不要钉」 |
| `impeccable /audit` `/polish` | 行为修完、Windows 真机可点之后 | 本轮缺口是流程和尺寸，不是间距审查 |
| `impeccable /adapt` | 多屏 DPI 贴图在真机上仍错 | 先做纯函数 1:1；不要把网页响应式搬来 |
| `research` | 对标行为文档缺了 | `REPORT.md` §2.1 / §2.3 已够。禁止再开一轮竞品调研当拖延 |
| `code-review` | diff 落地后 | 现在还没有 UX 修复 diff |
| 安全扫描类（semgrep / codeql / c-review） | 嵌入代码进树且要发版审查 | 本轮是 UX 路由，不是注入审计 |

---

## 4. 明确不用

这些会把本次活带歪，或平台不对。

### 4.1 Joe 工作流点名禁止

| Skill / 包 | 为什么不用 |
| --- | --- |
| Taste Skill、`design-taste-frontend`、Frontend Design、UI Brain、UI/UX Pro Max | 会换字体、圆角、配色、布局，做出「又一个 AI 站」。本产品要像 Snipaste，不像 Linear |
| `qiaomu-design` | 三拨盘、四方向预览、58 站 DESIGN.md、frontend-design / taste / ui-ux-pro-max 血统。会把托盘工具做成落地页试衣间。本轮没有视觉方向可选 |

### 4.2 网页 / 动效实现包

| Skill | 为什么不用 |
| --- | --- |
| `gsap-core` 及 GSAP 全家桶（react / plugins / scrolltrigger / timeline / …） | JS/DOM。仓库是 WPF |
| `transitions-dev`、`transitions-polish` | CSS 过渡目录。截图/Esc/钉图不应动画 |
| `impeccable /animate` `/live` | 浏览器里挑元素、加动效。远程没有可点的网页 App |
| gstack `browse` / `qa` / `design-html` / `design-shotgun` / `design-consultation` | 浏览器产品流程 |

### 4.3 会重开已拍板问题的包

| Skill | 为什么不用 |
| --- | --- |
| `grilling` | Joe 已经覆盖 ADR C4。再烤「该不该钉任务栏」是挡实现 |
| `create-plan`、`design`（design-doc-writer）、`execute-plan` | 实现 brief 已在 `PROMPT-UX-FIX-SNIPASTE.md`。本文件只路由 skill |
| Hengmu 架构审计 / 方案顾问 / 质量门 | 不是架构轮。进程模型、许可证、双进程已冻结 |
| `mobile-architecture-audit`、所有 `ios-*` | 不是移动应用 |

### 4.4 其它噪音

面试材料、海报/插图/PPT、简历、PM 画布（PRD/OKR/persona）、Obsidian、fuzzing、加密协议。与「直接 pin / 1:1 / Esc / 钉任务栏」无关。

浏览器验收：本产品是 WPF 托盘工具，**没有** 可打开的本地网页。用户规则里的浏览器验收对本轮不适用；下一轮以 Windows 真机点检 + DPI 纯函数测试为准。Debian 远程不能声称截图/任务栏已绿。

---

## 5. 和仓库旧决定打架时听谁的

| 旧决定 | 本次 Joe | 编码时怎么处理 |
| --- | --- | --- |
| ADR C4、§1.2：v1 网速只做桌面悬浮，不做 `SetParent` | 网速要固定在任务栏 | **听本次 Joe。** 这是产品覆盖，不是实现偷懒。HANDOFF 里写一句：C4 的「v1 不做嵌入」被 2026-09-05 UX 轮覆盖；默认嵌入，失败回退悬浮，设置可改回 |
| `security/AUDIT.md`：v1 不做任务栏嵌入网速 | 同上 | 审计当时按旧 ADR。嵌入 **不是** 注入 explorer。仍遵守：无驱动、无管理员、不把 CLR/TAP 塞进 Suite、失败不假成功 |
| `PROMPT-P1B-TASKBAR.md` Out：网速 SetParent | 网速要钉任务栏 | P1b 管 TaskbarFx 透明度。网速嵌入走 Suite 主进程窗口，**不要** 写进 TaskbarFx.Native |
| P1a 交接：Enter 复制，Ctrl+Enter 钉图 | 直接 pin | 默认路径改成确认后立刻 pin+剪贴板。标注改为可选，不要删掉能力 |

许可证红线 **不覆盖**：仍然禁止抄 Snipaste（闭源、禁逆向）、ShareX（GPL）、TrafficMonitor（Anti-996）源码。`SetParent(FindWindow("Shell_TrayWnd"))` 是公开 Win32 行为，自己写。

---

## 6. 下一轮建议调用顺序

给编码代理，不要加步骤。

1. 读本文件 + `PROMPT-UX-FIX-SNIPASTE.md` + `research/REPORT.md` §2.1 / §2.3。
2. `design-md-workflow`：确认不发明 DESIGN.md，不启用禁包。
3. `intended-vs-implemented`：列意图 vs 代码 vs ADR 覆盖。
4. `diagnosing-bugs`：先写 DPI→DIP 测试（红）。
5. 读 Impeccable **Operate** + `/distill` `/harden` `/clarify` 的约束（不跑 `/live`）。
6. 读 design-engineering 四个节点（产品 UI、状态、别动画、失败文案）。
7. `ponytail` full + `tdd` 一条 seam。
8. `operating-coding-change`：截图/DPI = scoped；网速嵌入 = governed。避开 `TaskbarFx.*`。
9. 写 `plans/UX-FIX-HANDOFF.md` 大白话。stdout `DONE`。不 `git push`。

**本轮已完成：** 只产出本文件。没有改业务代码。
