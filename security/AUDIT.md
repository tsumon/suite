# Windows 合集 App 安全审计（注入前）

- **日期：** 2026-09-05（Asia/Shanghai）
- **角色：** 安全审计（Grok Build）。只审计，不写业务代码、不出补丁、不定视觉、不出面试材料、不定架构方案。
- **性质：** 设计态审计。仓库内 **尚无** `src/`、安装脚本、CI、证书、NuGet/vcpkg 锁文件或发布产物。结论绑在已冻结 ADR 与公开文档上；实现落地后必须再审一遍。
- **不编 CVE。** 本产品未发布，没有 CVE 编号可引用。
- **禁止项遵守：** 本文不写 exploit、PoC、可复制攻击步骤。

---

## 0. 审计依据与核对日

| 材料 | 路径 / URL | 状态 / 核对 |
| --- | --- | --- |
| ADR（accepted） | `/workspace/windows-suite-app/architecture/ADR.md` | Joe 2026-09-05 拍板；本审计按此为合同面 |
| 调研 | `/workspace/windows-suite-app/research/REPORT.md` | 2026-09-05 |
| 主题补丁 | `/workspace/windows-suite-app/research/ADDENDUM-theme.md` | 2026-09-05 |
| 链接表 | `/workspace/windows-suite-app/research/SOURCES.md` | 2026-09-05 |
| 本目录 | `/workspace/windows-suite-app/security/` | 本轮只有 `PROMPT.md` + 本文件 |
| 代码 / CI / 密钥 | 工作区 `windows-suite-app` | **找不到** 业务源码、`.github/workflows`、证书、`.env` |

外部文档（本轮打开或检索，核对 2026-09-05）：

- `InitializeXamlDiagnosticsEx`：[learn.microsoft.com …/nf-xamlom-initializexamldiagnosticsex](https://learn.microsoft.com/en-us/windows/win32/api/xamlom/nf-xamlom-initializexamldiagnosticsex)（页内 ms.date 2018-12-05；页面 `updated_at` 2025-08-15）
- 屏幕捕获官方说明：[Screen capture](https://learn.microsoft.com/en-us/windows/apps/develop/media-authoring-processing/screen-capture)（ms.date 2026-08-23，页面更新 2026-08-27）
- `CreateForMonitor`：[IGraphicsCaptureItemInterop::CreateForMonitor](https://learn.microsoft.com/en-us/windows/win32/api/windows.graphics.capture.interop/nf-windows-graphics-capture-interop-igraphicscaptureiteminterop-createformonitor)（页内 last updated 2024-04-11）
- 打包模型：[Packaging overview](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/packaging/)（ms.date 2026-08-29）
- SmartScreen：[SmartScreen reputation](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation)（ms.date 2026-05-04，页面更新 2026-08-17）
- 签名选项：[Code signing options](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options)（ms.date 2026-08-29）
- Smart App Control：[Smart App Control overview](https://learn.microsoft.com/en-us/windows/apps/develop/smart-app-control/overview)（页面更新 2026-08-24）
- `PipeOptions.CurrentUserOnly`：[PipeOptions Enum](https://learn.microsoft.com/en-us/dotnet/api/system.io.pipes.pipeoptions?view=net-10.0)（ms.date 2025-07-01，页面更新 2026-05-27）
- `SetWindowDisplayAffinity`：[winuser SetWindowDisplayAffinity](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowdisplayaffinity)（页面更新 2026-08-05）
- Run 键：[Run and RunOnce Registry Keys](https://learn.microsoft.com/en-us/windows/win32/setupapi/run-and-runonce-registry-keys)（页面更新 2026-08-26）
- TranslucentTB README Security：[github.com/TranslucentTB/TranslucentTB](https://github.com/TranslucentTB/TranslucentTB)（调研核对 2026-09-05）
- TrafficMonitor Defender 驱动误报：[#2331](https://github.com/zhongyang219/TrafficMonitor/issues/2331)（2026-05-15；本产品 **不带驱动**）
- 误报提交：[Address false positives](https://learn.microsoft.com/en-us/defender-endpoint/defender-endpoint-false-positives-negatives)（页面更新 2026-09-02）；[Microsoft Security Intelligence](https://www.microsoft.com/en-us/wdsi/filesubmission)

---

## 1. 范围与假设

### 1.1 范围内

ADR §5.4 触点 **S1–S8** 加上 §2 进程模型：

| ID | 触点 | 本轮对象 |
| --- | --- | --- |
| S1 | 向 `explorer.exe` 加载自写 DLL | Win11：`InitializeXamlDiagnosticsEx` TAP off-label；Win10：进程内调用未文档 `SetWindowCompositionAttribute`（SWCA **不是** 把 DLL 载入 explorer，见 §2） |
| S2 | Authenticode / SmartScreen / Smart App Control | unpackaged 闭源安装包；EXE + Native DLL 必须同证书签 |
| S3 | 命名管道 ACL | `Suite.exe` ↔ `TaskbarFx.exe`；示意名 `\\.\pipe\Suite-TaskbarFx` |
| S4 | HKCU Run / 计划任务 | 托盘常驻、卸载清理 |
| S5 | 截图 API | `CreateForMonitor` → DXGI Duplication → GDI `BitBlt`；剪贴板 / PNG |
| S6 | 主题注册表 | HKCU `AppsUseLightTheme` / `SystemUsesLightTheme` + `ImmersiveColorSet` |
| S7 | 全局热键 | `RegisterHotKey` |
| S8 | 供应链 | 闭源、Grok 生成代码、无 GPL/Anti-996 合入、发版签名 |

进程与部署（ADR §2）：

```
Installer (EXE, 闭源, per-user unpackaged)
 ├── Suite.exe                 .NET 10 LTS + WPF（托盘、热键、截图、贴图、网速、主题）
 └── TaskbarFx.exe             C# 宿主
      └── TaskbarFx.Native.dll  C++；零托管；禁止 CLR 进 explorer
```

默认 **无管理员、无内核驱动**（ADR C8、§1.3）。v1 不做任务栏嵌入网速、不做 WinRing0。

### 1.2 范围外（明确不做）

- 写/改业务代码、补丁、PoC。
- 视觉、面试材料、替代架构方案。
- 对 TranslucentTB / ShareX / TrafficMonitor **源码**做漏洞挖掘（只引用其公开 Security/FAQ 作为画像旁证）。
- 对尚未存在的 CI YAML、NuGet 图、安装脚本做「已实现缺陷」判定。
- 企业机、MDM、具体 Windows build 的实测（主题补丁 §6 已声明未测）。

### 1.3 假设（成立才使本审计有效）

1. 实现将遵守 ADR：双进程、管道仅本机会话、注入只在 TaskbarFx.Native、不把 CLR 载入 explorer、不拷 GPL/Anti-996。
2. 目标是当前用户交互会话里的 `explorer.exe`（Medium 完整性、与标准用户同一 SID），不是 SYSTEM 服务、不是其他用户会话。
3. v1 无云端账号、无自动更新通道、无第三方遥测（ADR 未规划这些；若以后加，本审计作废）。
4. 「同一 Windows 用户、同一完整性级别」**不是** Windows 的保密边界。管道 ACL「仅当前用户」挡的是 **跨用户**，挡不住 **同用户其它进程**。出处：`PipeOptions.CurrentUserOnly` 文档原文 “the pipe can only be connected to a client created by the same user… On Windows, it verifies both the user account and elevation level.”
5. 本轮 **找不到** 实现代码，因此凡写「必须在编码时满足」的，都是门禁，不是已验证缺陷。

### 1.4 严重度

| 级 | 含义（本审计） |
| --- | --- |
| **Critical** | 跨用户提权、远程可达、密钥可被非本用户取走、或默认带内核驱动。本轮设计面 **未看到** 此类项。 |
| **High** | 会把代码载入 explorer / 静默全屏捕获 / 未签名即公开分发；或同用户任意进程可指挥注入。P1/P2 编码前必须有处置方向。 |
| **Medium** | 实现若按 ADR 字面最简路径落地，会扩大爆破面或杀软误报，但默认不跨用户。 |
| **Low** | 体验/持久化/热键冲突；安全影响有限。 |
| **Info** | 画像、对照、正面控制、或本轮找不到。 |

---

## 2. 威胁模型

### 2.1 资产

| 资产 | 为何敏感 | 出处 |
| --- | --- | --- |
| 屏幕像素（含其它应用窗口、通知、密码框若未设显示亲和） | 截图主路径静默抓监视器 | ADR §3.3；`CreateForMonitor` 文档 |
| 剪贴板中的 PNG/DIB | 截完默认进剪贴板 | ADR §4.1 |
| 用户桌面上的 PNG 文件 | 保存路径未在 ADR 钉死 | ADR §1.2「存 PNG」 |
| `explorer.exe` 地址空间与任务栏视觉树 | Win11 TAP 把自写 DLL 载入 Shell | ADR §5.3 路径 B；`InitializeXamlDiagnosticsEx` 参数 `wszTAPDllName` 文档写 “DLL to be injected in the process” |
| 任务栏外观（Opaque/Clear/Acrylic） | 可被管道命令改；失败则 Shell 不稳定 | ADR §5.2、R2 |
| 系统浅深色（HKCU Personalize） | 一键改 Apps+System | 主题补丁 §2.1 |
| `%LOCALAPPDATA%\Suite\settings.json` | 外观、热键、网卡选择；ADR 定为唯一写入者 | ADR §2.2 |
| 开机启动（HKCU Run） | 登录即起 | ADR §2.3；Run 键文档 |
| 发布用代码签名私钥 / Artifact Signing 身份 | 一旦泄漏可签任意 PE | [Code signing options](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options) 2026-08-29 |
| 闭源二进制与 Native 源 | 许可证隔离墙；被污染则无法闭源分发 | ADR §3.5、C2 |

**本轮找不到的资产：** 云端用户库、账号口令、更新服务器、崩溃上报密钥、分析 SDK。ADR 未列。

### 2.2 攻击面（设计面普查）

本产品不是网络服务。没有监听 TCP 端口的规划（ADR 明确不选 localhost TCP/gRPC，§2.2）。

| 面 | 谁能碰到 | 阶段 |
| --- | --- | --- |
| unpackaged 安装包 / MOTW / SmartScreen | 下载用户、邮件网关、企业策略 | 发版 |
| `Suite.exe` 全局热键、托盘、设置 UI | 本机交互用户 | P0+ |
| 屏幕捕获（无系统选取器） | 本进程；内容来自所有可见窗口 | P1 |
| 剪贴板读写 | 同用户其它进程也可读剪贴板（Windows 既有模型） | P1 |
| HKCU Personalize + `HWND_BROADCAST` | 本用户；广播到会话内顶层窗 | P0 |
| HKCU Run | 本用户；Autoruns/Sysinternals 可见 | P0 |
| 命名管道（本机会话） | 同用户、同完整性的其它进程（若只靠 CurrentUserOnly） | P1 |
| Win10 SWCA：本进程对 `Shell_TrayWnd` 设 composition | 不把 DLL 推进 explorer；仍是未文档 user32 | P1 |
| Win11 TAP：诊断 API 把 DLL 载入 explorer | EDR/Defender/Smart App Control 必看见模块加载 | P2 |
| Native DLL 文件（per-user 目录，用户可写） | 同用户可替换文件后再被宿主加载 | P1/P2 |
| 无 CI 的人工发版 / Grok 生成 Native | 供应链、许可证、误签测试包 | S8 |

**不是攻击面（本轮设计）：** 内核驱动、管理员服务、AppContainer 出口、商店身份、浏览器扩展。

### 2.3 信任边界

```
[其它 Windows 用户 / 其它会话]     边界 A：用户 SID + 完整性
        │
        ▼
[本用户 Medium IL 进程集合]        边界 B：Windows 基本不隔离
        │  Suite / 浏览器 / 同用户恶意软件 / 调试器
        ▼
[Suite.exe]  --命名管道-->  [TaskbarFx.exe]
        │                         │
        │ 捕获/剪贴板/HKCU        │ Native DLL
        ▼                         ▼
[桌面像素 / 设置文件]      [explorer.exe]   边界 C：进程，但 TAP 主动跨过
```

| 边界 | 谁该守 | 本设计实际 |
| --- | --- | --- |
| A 跨用户 | 管道 DACL + `CurrentUserOnly`（含 elevation） | ADR §2.2 要求「仅当前用户 SID，拒绝 Everyone/World」。**必须落地，否则升 Critical。** |
| B 同用户进程 | 管道客户端身份（映像路径 + 签名），不是「知道管道名就行」 | ADR 只写了 ACL 与「调试用显式开关」，**未**写校验对端 PE。见 F-03。 |
| C explorer | 仅 TaskbarFx.Native；失败降级；禁止 CLR | ADR §2.1、§5.3。P2 才跨这条边界。 |
| 包身份 / 隐私能力 | MSIX 的 `graphicsCaptureProgrammatic` 等 | unpackaged **没有** package identity（[Packaging overview](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/packaging/) 2026-08-29）。企业「按 PFN 管截图」政策 **对不上** 本安装形态。见 F-05。 |
| 发布信任 | Authenticode + 时间戳 + 稳定出版者身份 | ADR §5.4「未签名不作为对外 v1」。见 §5。 |

### 2.4 非目标对手（避免把桌面工具审成网关）

不把「已在本用户 Medium IL 跑起来的任意代码」当成要防住的完整保密边界——Windows 桌面模型做不到。本审计要防的是：

1. **跨用户** 指挥任务栏 / 读设置 / 连管道。
2. **产品自己** 把注入、持久化、静默截图做成「任意同用户进程的现成按钮」。
3. **发版通道** 未签名、签错文件、或把调试后门编进 Release。
4. **企业/Defender** 把预期行为打成恶意软件，导致用户被引导去关实时保护（这比误报本身更危险）。

---

## 3. 发现列表

编号 `F-xx`。每条：描述、出处、影响、建议处置方向（只方向，无补丁代码）。

### F-01 — Win11 TAP 是文档化的跨进程 DLL 载入，杀软会按注入画像处理

- **严重度：** High  
- **触点：** S1  
- **阶段：** P2（P0/P1 的 Win10 SWCA 路径不载入 explorer）

**描述：**  
ADR 路径 B 用 `InitializeXamlDiagnosticsEx` 把实现了 `IObjectWithSite` 的 DLL 载入 **目标 PID = explorer.exe**。Microsoft 自己的 API 文档把第四个参数写成 “The name of the DLL **to be injected in the process**”。这是 XAML **诊断/调试** 入口（文档第一句：entry point for any debugging tool），对 explorer 是 **off-label**，不是任务栏外观的官方 API。  
Win10 路径是本进程 `GetProcAddress(SetWindowCompositionAttribute)` 改 HWND，**不**把 DLL 映射进 explorer；两条路径的杀软画像不可混为一谈。

**出处：**

- ADR §5.3 路径 B、§5.4 S1；R3「先做安全审计再写注入」
- [InitializeXamlDiagnosticsEx](https://learn.microsoft.com/en-us/windows/win32/api/xamlom/nf-xamlom-initializexamldiagnosticsex)（核对 2026-09-05）
- 调研 §2.2：TranslucentTB 2026.2 仍用 ExplorerTAP；README Security “Some antiviruses are over eager”
- 对照：TranslucentTB 能上 Store 是开源 + 商店重签；本产品是闭源 unpackaged（ADR §2.3），不能套用其商店声誉

**影响：** 模块加载进 `explorer.exe` 会被 Defender / EDR / Smart App Control 看见。未签名或新哈希时：拦截、隔离、或「不安全应用」。explorer 若因 DLL 崩溃，任务栏/开始菜单短暂消失（ADR 已接受 R2，但不能当「没问题」）。

**建议处置方向：**

- P2 编码前锁定：仅绝对路径加载、加载前校验 Authenticode（同一出版者）、失败则 `unavailable`，禁止静默重试死循环（ADR 已有重试上限方向）。
- 发布说明用 **用户语言** 写清「任务栏效果会向资源管理器加载本产品签名的辅助模块」，不要教用户关 Defender。
- 误报走 [Security Intelligence 提交](https://www.microsoft.com/en-us/wdsi/filesubmission)，不要把「给 Defender 加排除」写成默认安装步骤（排除本身是攻击者常用持久化，见 Microsoft 对 exclusions 的警告：[Defender exclusions overview](https://learn.microsoft.com/en-us/defender-endpoint/microsoft-defender-antivirus-exclusions-overview) 2026-08-25）。
- 保持：无驱动、无 CLR 进 explorer、TaskbarFx 独立进程。

---

### F-02 — 对外 v1 未签名或只签安装包、不签 Native/TAP DLL，会被 SmartScreen / Smart App Control 拦住

- **严重度：** High  
- **触点：** S2  
- **阶段：** 任何对外构建；P2 尤其硬

**描述：**  
unpackaged 无商店证书。官方 SmartScreen 表：无签名 / 自签 → 「Windows 已保护你的电脑」，企业策略可禁止「仍要运行」。**EV 不再换立即声誉**（文档原句：EV certificates no longer bypass SmartScreen）。新哈希即使签了也会先告警，靠出版者身份随下载量积累。  
Windows 11 Smart App Control 在强制模式下：**未签名且云端无法判定安全的二进制直接阻止**；检查对象是 **所有可执行文件**，不只带 MOTW 的下载文件。注入进 explorer 的 DLL 若未签，比主 EXE 更容易被拦。

**出处：**

- [SmartScreen reputation](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation)（ms.date 2026-05-04）
- [Code signing options](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options)（ms.date 2026-08-29）：Artifact Signing（原 Trusted Signing）推荐非商店分发；地理限制：组织美/加/欧/英，个人仅美/加
- [Smart App Control overview](https://learn.microsoft.com/en-us/windows/apps/develop/smart-app-control/overview)（2026-08-24）：开发者应签 **exe、dll、安装临时文件、卸载器**
- ADR §2.3、§5.4、§7 P2「发布产物有 Authenticode；SmartScreen 仍可能拦」
- 调研 §2.1：Snipaste 中文故障排除亦有未签名「仍要运行」（完整 wiki 本轮未再抓，沿用调研）

**影响：** 用户装不上或 TAP 加载失败被当成「功能坏了」；若文档引导关 Smart App Control / 实时保护，等于削弱机器基线。Artifact Signing 2026 年中间 CA 轮换导致声誉中断，见 Microsoft Q&A（例：[5979088](https://learn.microsoft.com/en-ca/answers/questions/5979088/smartscreen-still-shows-unrecognized-app-for-trust)，讨论 2026-06 声誉传播修复；**无公开 SLA**）。

**建议处置方向：**

- 对外 v1：**安装包、Suite.exe、TaskbarFx.exe、TaskbarFx.Native.dll、TAP DLL（若与 Native 分离）** 用同一出版者身份签 SHA256 + 时间戳。开发自签仅限本机。
- 优先评估 Azure Artifact Signing；若 Joe 个人身份不在美/加，改 OV（同文档）。不要为 SmartScreen 去买 EV。
- 固定出版者身份，避免每版换证。
- 发布说明写：前几周可能出现无法识别应用；核对出版者名称后再继续。
- **本轮找不到** Joe 的国家/主体，无法选定具体 CA。编码前产品侧要定身份，否则 P2 不能称可分发。

---

### F-03 — 管道「仅当前用户」挡不住同用户其它进程指挥 TaskbarFx

- **严重度：** High（在启用注入之后）；P0 无 TaskbarFx 时不构成注入门禁  
- **触点：** S3  
- **阶段：** P1 起

**描述：**  
ADR 管道协议含 `Enable` / `Disable` / `SetAppearance` / `Shutdown`。ACL 计划「仅当前用户 SID」。.NET `PipeOptions.CurrentUserOnly` 的保证是：**同一用户 + 同一完整性（elevation）**。同用户、同 Medium IL 的其它进程仍可连接。  
ADR 还写「连接方必须是已启动的 Suite **或本机调试器（调试用显式开关）**」——若 Release 也能打开该开关、或开关把 DACL 放宽到 Everyone，属于 fail-open。  
示意管道名 `Suite-TaskbarFx` 可猜测。ADR 提到「实现可加随机后缀+发现」，但未把「防抢占」写成必须：若未设 `FirstPipeInstance`，同用户可先占同名管道，让 Suite 连到别人。

**出处：**

- ADR §2.2 IPC 表、ACL 句、调试开关句、管道名
- [PipeOptions.CurrentUserOnly](https://learn.microsoft.com/en-us/dotnet/api/system.io.pipes.pipeoptions?view=net-10.0)（核对 2026-09-05）
- [CreateNamedPipeW](https://learn.microsoft.com/en-us/windows/win32/api/namedpipeapi/nf-namedpipeapi-createnamedpipew)：新管道的 DACL 来自安全属性；缺省 ACL 不是「仅 Suite」

**影响：** 不会跨用户提权。会让「向 explorer 加载 DLL / 改任务栏」变成同用户任意进程可调用的能力，放大 EDR 噪声，也让本产品成为别人的现成注入器。调试开关若进 Release，边界直接消失。

**建议处置方向：**

- 默认 `CurrentUserOnly` + 显式 DACL（当前用户允许，Everyone/World/Anonymous **拒绝**）。
- 接受连接后核验对端：PID → 映像完整路径 → 位于安装目录 **且** Authenticode 为本产品出版者。不匹配则断开。不要在文档里写具体校验代码。
- `FirstPipeInstance`；管道名加每会话随机分量，发现手段勿在全局 Everyone 对象上。
- 调试放宽 ACL：**仅 Debug 构建**，Release 编译期去掉。
- 所有管道调用超时（ADR 已建议 2 s），超时不得阻塞截图热键线程。

---

### F-04 — per-user 目录里的 Native/TAP DLL 对同用户可写，加载路径若只用「文件名」会碰到搜索顺序

- **严重度：** Medium  
- **触点：** S1、S8  
- **阶段：** P1/P2

**描述：**  
per-user unpackaged 安装目录通常在用户可写位置。同用户替换 `TaskbarFx.Native.dll` 后，宿主会加载替换件。这是 per-user 模型的固有限制，不能靠「ACL 成 SYSTEM 只读」在无管理员前提下解决。  
`InitializeXamlDiagnosticsEx` 的 `wszTAPDllName` 文档类型是「**name** of the DLL」，不是「绝对路径」。若实现按文件名交给诊断栈，加载会走 DLL 搜索顺序，额外扩大「同名 DLL 先被找到」的风险。

**出处：**

- ADR §2.3 per-user unpackaged
- [InitializeXamlDiagnosticsEx 参数 wszTAPDllName](https://learn.microsoft.com/en-us/windows/win32/api/xamlom/nf-xamlom-initializexamldiagnosticsex)
- 本轮 **找不到** Microsoft 对该参数是否接受绝对路径的更细说明

**影响：** 同用户已能改用户目录，这不是跨用户提权。但「先改 DLL 再让已签名的 TaskbarFx 去加载」会让恶意模块借产品的签名进程进 explorer，EDR 记在本产品名下。

**建议处置方向：**

- 始终按安装目录绝对路径指定 TAP/Native。
- 加载前验证该路径的 Authenticode 与预期出版者；失败则拒绝注入并记 `GetStatus`。
- 安装/卸载约定：升级覆盖同路径；卸载后重启 explorer 确认模块列表不再含本 DLL（ADR P2 已列可验证标准）。

---

### F-05 — 静默监视器捕获绕过官方文档描述的选取器同意；unpackaged 对不上按 PFN 的企业截图策略

- **严重度：** High（隐私/合规画像）；产品作为截图工具是预期能力，不是漏洞编号  
- **触点：** S5  
- **阶段：** P1

**描述：**  
Microsoft 面向应用开发者的 [Screen capture](https://learn.microsoft.com/en-us/windows/apps/develop/media-authoring-processing/screen-capture) 文章把「安全系统 UI 选取 + 黄框」写成该 API 族的用户可见模型，并写：用户在系统 UI **明确同意** 后，同一 `GraphicsCaptureItem` 才可复用。  
ADR 主路径是 `IGraphicsCaptureItemInterop::CreateForMonitor`（静默、无黄框选取器），失败再 DXGI Desktop Duplication、再 `BitBlt`。这是 Win32 互操作扩展（[CreateForMonitor](https://learn.microsoft.com/en-us/windows/win32/api/windows.graphics.capture.interop/nf-windows-graphics-capture-interop-igraphicscaptureiteminterop-createformonitor)，Win10 1903+），官方未在该页写「必须先走 Picker」。  
打包应用要用 `graphicsCaptureProgrammatic` / `RequestAccessAsync(Programmatic)` 才能从 WindowId/DisplayId 创建 item（[App capability declarations](https://learn.microsoft.com/windows/uwp/packaging/app-capability-declarations) 中 Programmatic Graphics Capture；[TryCreateFromWindowId 备注](https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.graphicscaptureitem.trycreatefromwindowid)）。这些能力挂在 **包清单** 上。unpackaged **无 package identity**，企业 GPO `LetAppsAccessGraphicsCaptureProgrammatic_*` 按 **PFN** 白名单/黑名单（Microsoft Q&A [5820069](https://learn.microsoft.com/en-ca/answers/questions/5820069/inconsistent-screenshots-and-screen-recording-priv)，讨论企业策略）。**本轮找不到** 官方一句「unpackaged + CreateForMonitor 是否仍弹隐私开关」的完整说明。

**出处：** 上列 Learn 页；ADR §3.3；调研 §4 捕获顺序。保护窗：[`SetWindowDisplayAffinity`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowdisplayaffinity)（`WDA_MONITOR` / `WDA_EXCLUDEFROMCAPTURE`）；ADR 已写 v1 允许黑块、不承诺破网课防截。

**影响：** 热键一按即可取得本机屏幕（Snipaste/ShareX 同类工具亦如此）。在企业 DLP / 隐私审核里，unpackaged 静默捕获比 Store 应用更难用 PFN 策略约束。若实现去「补全」被 `WDA_EXCLUDEFROMCAPTURE` 挡住的窗口，会从工具变成对抗保护 API，杀软与企业不可接受。

**建议处置方向：**

- 保持 ADR：尊重显示亲和；黑块即黑块，禁止另寻路径去读被保护内容。
- 设置/隐私说明写清：截图会捕获当时屏幕上可见内容；企业环境可能被策略或 DLP 限制。
- 截图落盘给明确目录与清理策略（见 F-06）；不要为「更像 Snipaste」去关黄框而申请 borderless 能力——unpackaged 也没有该清单能力。
- Picker 仅调试（ADR 已定）。不要在 P0 提前引入捕获。

---

### F-06 — 截图进入剪贴板与本地文件后，没有产品级保密边界

- **严重度：** Medium  
- **触点：** S5、本地存储  
- **阶段：** P1

**描述：**  
ADR §4.1：确认后写入剪贴板 PNG/DIB，可选钉图。Windows 剪贴板对本用户其它进程可读，这是平台模型，不是实现 bug。ADR **未**规定：临时文件路径、是否用 DPAPI、失败截图是否残留、钉图窗是否在锁屏后仍显示内容。  
`%LOCALAPPDATA%\Suite\settings.json` 是明文配置预期；其中不应出现截图像素。本轮 **找不到** 设置 schema 终稿。

**出处：** ADR §2.2、§4.1、§1.2；工作区无 `settings.schema.json`。

**影响：** 同用户恶意软件本来就能截屏；产品额外把最近一张图放进剪贴板/磁盘，扩大「顺手拿走」的窗口。锁屏后钉图仍置顶属于隐私产品问题。

**建议处置方向：**

- 临时文件用用户私有目录、用完删除；不要写到共享或可被 Everyone 读的路径。
- settings.json 只存偏好，不存位图。
- 不引入云同步（v1 无此需求）。
- 钉图与锁屏交互：P1 真机列一条「锁屏后贴图是否仍可见」观察项（产品决策，不是补丁）。

---

### F-07 — HKCU Run 是文档化的登录启动；卸载与「以管理员运行」是已知坑

- **严重度：** Low（预期能力）；卸载残留升为 Medium  
- **触点：** S4  
- **阶段：** P0

**描述：**  
ADR 用 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` 或登录计划任务。这是 Microsoft 文档化的「每次用户登录运行」机制，也是 Autoruns 默认会列出的位置。企业可关启动项（TranslucentTB README 亦然）。  
调研：Snipaste「以管理员运行导致开机启动失败」；TrafficMonitor FAQ 同类。高完整性进程写的 HKCU Run 与普通登录启动的完整性不一致时，会出现「装了但登录不起」或 UIPI 怪癖。

**出处：**

- [Run and RunOnce Registry Keys](https://learn.microsoft.com/en-us/windows/win32/setupapi/run-and-runonce-registry-keys)（2026-08-26）
- [Autoruns](https://learn.microsoft.com/en-us/sysinternals/downloads/autoruns)
- ADR §2.3、§7 P0「Run 键可逆」、P2「卸干净 Run 键」
- 调研 §2.1、§2.3

**影响：** 不是漏洞。卸载后若残留 Run 值，每次登录会拉起缺失二进制，用户与 EDR 都当异常。默认提权运行会打乱启动与管道 elevation 校验（CurrentUserOnly 会区分完整性）。

**建议处置方向：**

- 安装程序只写 HKCU（不要 HKLM Run，那要管理员且变成全用户持久化）。
- 卸载删除本产品 Run 值；设置里的开关必须可逆（P0 验收）。
- 检测到自身以管理员/High IL 运行：拒绝注入路径并提示「请用标准用户重开」（方向；文案交产品）。
- 计划任务若用，ACL 限当前用户，卸载同样删除。

---

### F-08 — 主题键写入与全会话广播：无提权，但和企业锁策略、其它主题工具互踩

- **严重度：** Low  
- **触点：** S6  
- **阶段：** P0

**描述：**  
写 HKCU `Themes\Personalize` 两 DWORD + `SendMessageTimeout(HWND_BROADCAST, WM_SETTINGCHANGE, …, L"ImmersiveColorSet")`。HKCU 不需要管理员。主题补丁已说明：无官方 Toggle API；企业 MDM 锁主题后的行为 **未测**；与 Auto Dark Mode 抢同一键。广播会打到会话内所有顶层窗，是该技巧的既有做法，不是隐蔽信道设计。

**出处：** 主题补丁 §2.1、§2.4、§6.4；ADR §4.3；Windows Central / Mike van Oorschot 文（主题补丁链接表）。

**影响：** 无跨用户。失败应可见，避免「半切换」被当成成功。广播超时要用 `SMTO_ABORTIFHUNG`（主题补丁已写），以免卡死本进程。

**建议处置方向：**

- 写注册表失败 → UI 明确失败（策略锁定）。
- 设置页提示与 Auto Dark Mode 互斥（主题补丁 §3）。
- 默认不杀 explorer（ADR 已禁）。

---

### F-09 — 全局热键可与系统截图工具或其它应用抢注册

- **严重度：** Low  
- **触点：** S7  
- **阶段：** P0 占位 / P1 真用

**描述：**  
`RegisterHotKey` 是会话级抢注。ADR 已指向系统「PrtScr 打开截图工具」与 ShareX issue。失败或被抢不是内存破坏，是功能不可用。

**出处：** ADR §4.1、R8；SOURCES.md `ShareX/ShareX/issues/7847`；调研 §4。

**影响：** 用户以为热键被「劫持」。无提权。

**建议处置方向：** 可改键；注册失败可见；设置说明去关系统 PrtScr 开关。不要循环抢注。

---

### F-10 — 供应链：无仓库、无 CI、无锁文件；P2 Native 由 Grok 生成，GPL 污染与误签是真实风险

- **严重度：** High（作为 P2 门禁）；当前仓库状态为 Info（东西还不存在）  
- **触点：** S8  
- **阶段：** 建仓与发版前

**描述：**  
工作区无 `src/`、无 `.github/workflows`、无 `nuget.config`、无证书存储约定。ADR 禁止现在建 GitHub remote。代码将由 Grok Build 写、Joe 在 Windows 机编译。Native/TAP 若「长得像」TranslucentTB ExplorerTAP，闭源产品即许可证事故（ADR §3.5、R7）。  
.nuget 恢复若以后引入未钉版本包，属于经典供应链面；本轮 **找不到** 包清单。  
签名私钥若出现在仓库或聊天记录，是 Critical 级事故——目前 **找不到** 任何密钥，这是好事。

**出处：** 工作区目录列表 2026-09-05；ADR §3.5、§6、§8 R7、C2；调研许可证红线。

**影响：** 没有 CI 意味着：谁都能在 Joe 的机器打「看起来像正式版」的未签名包；依赖漂移无人看；GPL 片段没有自动化门禁。

**建议处置方向：**

- 建仓时：证书与 `*.pfx` / Artifact Signing 机密进 gitignore 与密钥库，永不进 Git。
- Native 评审当许可证审查（ADR 已要求 Joe 审 Native diff）。
- 发布构建只在受控 Windows 机；SignTool 不把私钥写进命令行历史可泄露的脚本（方向：用 CI 机密或本地令牌，细节交发版）。
- 有 NuGet 之日再做锁文件与 `dotnet list package --vulnerable`；本轮无法做 CVE 扫描。

---

### F-11 — 无第三方在线 API；网速 API 是本机 IP Helper

- **严重度：** Info  
- **触点：** 第三方 API  
- **阶段：** P0

**描述：** ADR 网速用 `GetIfTable2` / `GetIfEntry2`（[文档](https://learn.microsoft.com/en-us/windows/win32/api/netioapi/nf-netioapi-getiftable2)）。这是本机接口计数，不是云 API，不需要登录。v1 不做流量历史上传。  
**本轮找不到** 计划中的崩溃上报、许可服务器、广告 SDK、更新 XML。

**建议处置方向：** P0 保持无网络依赖（除用户自己的网卡统计）。以后若加更新通道，另开审计。

---

### F-12 — 正面控制：无驱动、双进程、禁止 CLR 进 explorer

- **严重度：** Info（保持这些才继续有效）  
- **触点：** S1、S8

**描述：** TrafficMonitor 标准版硬件监控曾带 WinRing0，Defender 报 `VulnerableDriver:WinNT/Winring0`（[#2331](https://github.com/zhongyang219/TrafficMonitor/issues/2331)，2026-05-15）。本产品明确不带任何驱动、默认不管理员，避免这条内核画像。双进程让截图热键不与注入同生共死（调研 §1、ADR §2.1）。禁止 CLR 进 explorer 避免多运行时与 GC 停顿打在 Shell 上（ADR §2.1，标「推断」——本审计同意作为编码约束，不是已证明的 CVE）。

**建议处置方向：** 保持。任何「为了方便把 C# TAP 推进 explorer」的提议应直接拒绝。

---

### F-13 — Win10 SWCA 仍是未文档 user32，但不是注入；P1 与 P2 门禁必须分开

- **严重度：** Medium（稳定性 + 未文档）；不是 S1 同级的「DLL 进 explorer」  
- **触点：** S1 的 Win10 侧  
- **阶段：** P1

**描述：** `SetWindowCompositionAttribute` 无正式 Win32 参考页。Microsoft Q&A 承认 TranslucentTB 走这条路（[1720291](https://learn.microsoft.com/en-us/answers/questions/1720291/taskbar-shell-traywnd-transparency-like-in-translu)，提问 2024-06-20）。sylveon：[WindowsAppSDK discussion #2711](https://github.com/microsoft/WindowsAppSDK/discussions/2711)（未文档、曾被更新弄坏）。调用发生在 **TaskbarFx 进程内**，目标是任务栏 HWND。

**影响：** Windows 更新可让效果消失或 HWND 行为异常；通常不表现为「explorer 加载了未知 DLL」。仍须失败可见（ADR §5.2）。

**建议处置方向：** GetProcAddress 失败/HRESULT 失败 → `unavailable`，禁止假成功。P1 不得在 Win11 现代 XAML 任务栏上「顺便」调用 TAP。

---

## 4. P0 / P1 / P2 编码门禁

原则：P0 与本审计并行是 ADR §8 已允许的。**注入相关代码（TAP，以及任何 `OpenProcess`/`InitializeXamlDiagnosticsEx` 针对 explorer 的路径）在下列门禁未满足前不得开始。** 本审计 **不** 给「已通过」印章给 P2——P2 还要在有二进制之后做一次差异审计。

### 4.1 P0（无 TaskbarFx、无捕获完整实现）——可以编码，须遵守

P0 做托盘、单实例、settings.json、HKCU Run、ThemeToggle、NetSpeed 桌面窗、热键占位。

| # | 必须在 P0 实现/约定里满足 | 对应 |
| --- | --- | --- |
| P0-1 | 不链接、不 P/Invoke TAP/SWCA、不 `CreateProcess` TaskbarFx | ADR §7 P0 |
| P0-2 | settings.json 只在 `%LOCALAPPDATA%\Suite\`（或同等用户私有目录），不存截图像素、不存密钥 | F-06 |
| P0-3 | Run 键只 HKCU、设置可逆；卸载计划写明删 Run | F-07 |
| P0-4 | 主题写入失败要可见；不杀 explorer | F-08 |
| P0-5 | 不以管理员为默认；若检测到 High IL，P0 即可提示（为 P1 管道 elevation 做准备） | F-07、F-03 |
| P0-6 | 仓库/gitignore 预留：证书、`bin/`、`obj/`；现在仍可按 ADR **不**建 GitHub | F-10 |
| P0-7 | 无网络客户端、无第三方 SDK | F-11 |

P0 **不需要** 代码签名即可在 Joe 真机自用；**不得**把未签名包当对外 v1。

### 4.2 P1（截图贴图 + TaskbarFx + **仅 Win10/经典任务栏 SWCA**）——编码前必须书面满足

| # | 门禁 | 不满足则 |
| --- | --- | --- |
| P1-1 | 管道：CurrentUserOnly + 拒绝 World/Everyone + 对端映像/签名校验方向已写进实现约束；Debug ACL 开关不能进 Release | F-03 |
| P1-2 | `FirstPipeInstance` + 非纯猜测的管道名 | F-03 |
| P1-3 | Native 只在 TaskbarFx 进程；Suite **零** 注入符号 | ADR C7、F-12 |
| P1-4 | Win11 设置页「路径未交付」，**禁止** 调用 `InitializeXamlDiagnosticsEx` | ADR §7 P1、F-01 |
| P1-5 | SWCA 失败 → unavailable，不假成功 | F-13 |
| P1-6 | 捕获：尊重 `SetWindowDisplayAffinity`；不破解保护窗 | F-05 |
| P1-7 | 对外试玩包若离开 Joe 机器：至少签 EXE（可仍 SmartScreen 告警） | F-02 |
| P1-8 | 截图临时文件生命周期有约定 | F-06 |

P1 通过的是「经典任务栏效果 + 截图」，**不是** 拍板意义上的 v1（C1 要求 Win11 亚克力）。

### 4.3 P2（Win11 TAP + 安装包收口 = 拍板 v1）——编码 TAP 前必须全部满足

| # | 门禁 | 对应 |
| --- | --- | --- |
| P2-1 | **签名身份已选定**（Artifact Signing 或 OV），地理/主体已核；发布流水线能对 **全部 PE** 签 SHA256+时间戳 | F-02 |
| P2-2 | TAP 用安装目录 **绝对路径**；加载前校验本产品签名；失败降级 | F-04、F-01 |
| P2-3 | 禁止 CLR / 托管 TAP 进 explorer | F-12 |
| P2-4 | 重试预算与 Disable→Normal（ADR §4.4、§5.2）写进实现约束 | F-01 |
| P2-5 | 用户可见说明：会向资源管理器加载已签名模块；误报走 WDSI，默认安装 **不** 加 Defender 排除 | F-01 |
| P2-6 | 管道门禁 P1-1/P1-2 已在代码中存在（P2 不得在无认证管道上首次接 TAP） | F-03 |
| P2-7 | 卸载：Run 键、可选设置目录、explorer 重启后无本 DLL | ADR §7 P2、F-07 |
| P2-8 | Native/TAP **自写**；PR 检查无 GPL/Anti-996 片段 | F-10 |
| P2-9 | Joe 真机：Win11 当前功能更新 + 至少一台 SmartScreen 默认开启的消费机，记录告警原文（不是「我们觉得没事」） | §5 |

**本审计对 P2 TAP 的结论：** 方向可接受（拍板 C1），**实现未开始，不能宣布「注入审计通过」**。满足上表后才能编码；有了签名二进制再做一次差异审计（行为 vs Defender 实际判定）。

---

## 5. 签名 / SmartScreen / Defender 预期画像

这是预期，不是已测报告。本轮 **没有** 可提交 VT 的哈希。

### 5.1 分发形态决定基线

| 形态 | SmartScreen / SAC | 本产品 |
| --- | --- | --- |
| Store MSIX | 商店重签，下载无 SmartScreen 告警 | ADR 否决 v1 Store（注入 + HKCU 与包模型冲突） |
| unpackaged 已签 OV/Artifact Signing | 新应用/新哈希：**会** 出现无法识别；出版者名可见；声誉靠量 | **v1 预期** |
| unpackaged 未签 / 自签 | 强提示或企业直接禁；SAC 可直接阻止 | 仅开发机 |

EV：**不要** 为躲避 SmartScreen 购买（[SmartScreen reputation](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation) 2026-05-04）。

### 5.2 预期用户可见现象（对外首发数周）

1. 浏览器下载带 Mark-of-the-Web 的安装包 → SmartScreen 应用声誉检查。
2. 「Windows 已保护你的电脑」或「不常见」——**即使签名正确**（官方：无精确阈值；「数周 + 大量干净安装」是文档量级，不是合同）。
3. Win11 干净安装且 Smart App Control 处于强制：未签名 DLL/EXE **直接拦**；已签但云端判「可能不安全」也可拦（Q&A 有已签仍拦的个案，[5757633](https://learn.microsoft.com/en-us/answers/questions/5757633/how-come-our-code-signed-executable-file-is-blocke) 2026-02，非本产品）。
4. 打开 TaskbarFx 后，Defender 保护历史可能出现与「进程加载异常模块 / 行为监控」相关的条目——**本轮找不到** 针对 `InitializeXamlDiagnosticsEx` 的官方检测显示名。旁证仅有 TranslucentTB「杀毒过激」与 TrafficMonitor 的 **驱动** 误报（后者与我们无关）。

### 5.3 分阶段画像（给发版说明用）

| 阶段 | 典型行为特征（产品自己的） | 相对风险 |
| --- | --- | --- |
| P0 | 托盘常驻、HKCU Run、写 Personalize、读网卡计数 | 低；像普通工具 |
| P1 | 静默屏幕捕获、剪贴板、独立进程、未文档 SWCA | 中；截图工具常见，SWCA 较少见但不开注入 |
| P2 | 上一条 + **向 explorer 加载自写 DLL** | 高；与 TranslucentTB 开源注入同类，但闭源无源码自证 |

### 5.4 建议的发版姿势（方向）

- 签 **每一个** 会执行或被加载的 PE，含卸载器。
- 不要签完再改文件（官方：改文件可破坏签名）。
- 同一出版者连续发版，让证书声誉有机会积累；Artifact Signing 短时证书轮换可能打断声誉（2026 Q&A，无官方修复时限）。
- 企业内测：内网路径可避开 SmartScreen 下载检查；仍过 SAC（SAC 看所有执行，不只下载）。
- 误报：WDSI 提交文件 + 说明「任务栏外观辅助模块，由用户安装的已签名应用加载」。不要默认加排除。
- **禁止** 在安装程序里改 Defender 排除或关实时保护。

### 5.5 本轮无法给出的画像数字

- SmartScreen 解除告警的精确下载次数：**找不到**（官方明确无公开阈值）。
- 本产品在 VirusTotal 的检出比：无二进制。
- 某 EDR 厂商对 TAP-into-explorer 的规则名：**找不到**。

---

## 6. 本轮找不到的项

下列曾检索或检视工作区，**没有** 达到可引用的官方/仓库证据。不编造。

| # | 寻找什么 | 结果 |
| --- | --- | --- |
| N1 | 业务源码、`Suite.sln`、Native 工程 | 工作区无 `src/` |
| N2 | CI（GitHub Actions / Azure Pipelines） | 无 |
| N3 | 代码签名证书、Publisher 主体、Joe 是否在 Artifact Signing 地理范围内 | 无 |
| N4 | 面向第三方的官方「切换系统浅深色」WinRT API | 主题补丁 §2.4、§6.1 已记；本轮未找到对等官方 API |
| N5 | `InitializeXamlDiagnosticsEx` 是否要求 `SeDebugPrivilege` 的官方语句 | API 页未写。同用户同完整性注入在 Windows 模型上通常不需管理员——**推断**，须真机核。不能写成「已证明无需任何特权」 |
| N6 | `wszTAPDllName` 是否接受绝对路径的官方细文 | 仅有 “name of the DLL” |
| N7 | unpackaged + `CreateForMonitor` 在 Win11 是否弹出「屏幕截图」隐私开关 / 是否受 `LetAppsAccessGraphicsCaptureProgrammatic` 约束 | 能力文档绑包清单与 PFN；unpackaged 行为 **无** 对等官方页 |
| N8 | Microsoft 对「用 XAML 诊断 API 改系统任务栏」的支持/禁止声明 | 只有诊断 API 文档；用途是调试工具 |
| N9 | 本产品 CVE、VT 报告、Defender 家族名 | 产品未发布 |
| N10 | SmartScreen 声誉定量阈值 | 官方写明无公开阈值 |
| N11 | 企业 MDM 锁 Personalize 后的确切错误码 | 主题补丁 §6.4 未测 |
| N12 | 与 Auto Dark Mode / ExplorerPatcher / 现网 TranslucentTB 同机的安全互操作实测 | 无 |
| N13 | 第三方云 API、更新服务器、遥测密钥 | ADR 未规划；工作区无 |
| N14 | `settings.schema.json` 终稿、截图默认保存路径 | ADR 仅有目录建议 |
| N15 | Snipaste 中文故障排除 wiki 全文（调研已注明完整页 fetch 失败） | 本轮未再打开成功；不把搜索摘要当唯一证据 |
| N16 | .NET 10 在消费者 Win10 22H2 上的安全支持合同 | ADR §2.4 已写官方矩阵冲突；属支持面不是本审计能「找齐」的漏洞 |

---

## 7. 结论（给编码与拍板）

1. **P0 可以按 ADR §7 开工**，遵守 §4.1。主题、网速、Run、设置文件不引入注入面。  
2. **P1 可以在管道身份校验与「Win11 禁止 TAP」写进约束之后开工。** SWCA 不是 DLL 注入，但仍是未文档 API。  
3. **P2 TAP 在签名身份、绝对路径加载、加载前验签、Release 无调试 ACL、用户说明不以「关杀软」为默认路径之前，不得编码。** 这是 ADR R3 的具体化。  
4. 设计面 **没有** 看到跨用户 Critical 问题，前提是管道 DACL 真正拒绝 Everyone 且不把服务做成 SYSTEM。  
5. 最大真实风险不是「被远程打穿」，而是：**闭源 unpackaged 注入 explorer + 静默截屏** 的 Defender/SAC/企业画像，以及 **同用户管道变成通用注入按钮**。  
6. 实现出现后必须复审：管道 DACL 实际 ACE、SignTool 输出、TAP 加载路径、卸载残留。

---

**本工具不能替代专业渗透测试或律师对 GPL 隔离的意见。** 本文件是注入前的设计态审计，置信度受「无代码、无真机 Defender 日志」限制。
