# Windows 合集 App：竞品与技术路线调研

- 调研日期：2026-09-05（Asia/Shanghai）
- 范围：Snipaste 级截图/贴图、TranslucentTB 级任务栏透明/亚克力、TrafficMonitor 级网速显示
- 方法：官方站 / Microsoft Learn / GitHub（`gh api` 核对 star、许可证、最近提交）。找不到的写「找不到」，不编造。
- 本轮只调研，不写业务代码。

---

## 1. 一句话结论

**一个安装包合理；一个进程只适合「截图+贴图+桌面网速悬浮窗」。不要把任务栏亚克力塞进同一个进程。**

| 合不合理 | 判断 |
| --- | --- |
| 一个安装包、托盘入口、统一设置 | 合理。三款对标本身都是托盘常驻小工具。 |
| 一个进程做完三件事 | **不合理作为 v1 目标。** 截图/贴图与网速悬浮窗可以同进程；任务栏亚克力需要对 `explorer.exe` 注入或挂钩，崩溃会拖垮整套工具，杀软画像也完全不同。 |
| 推荐形态 | **一个安装包 + 两个进程（或一个主进程 + 可选「任务栏效果」进程）**。主进程：截图、贴图、桌面网速。可选进程：任务栏外观。 |

**最大技术风险（按杀伤力）：**

1. **任务栏透明/亚克力（最高）**  
   Win11 任务栏已从 Win32 变成 XAML Island；TranslucentTB 自己在 2026.2 写明「Avoid ExplorerHooks in modern XAML taskbar」，并继续用 ExplorerTAP 注入 explorer。这是未文档化路径，Windows 小版本就能打断。出处：[TranslucentTB README](https://github.com/TranslucentTB/TranslucentTB)（核对 2026-09-05）；[Release 2026.2](https://github.com/TranslucentTB/TranslucentTB/releases/tag/2026.2)（发布 2026-08-31，核对 2026-09-05）；仓库 topics 含 `undocumented`（`gh api` 2026-09-05）。

2. **截图/贴图（中高）**  
   不是「截一张图」难，而是 **HDR/BitBlt 过曝、多屏 DPI、受保护窗口、Win11 元素检测、置顶分层窗口**。Snipaste 闭源且 EULA 禁止逆向；可参考的开源实现是 ShareX（GPL-3.0），不是 Snipaste。出处：[snipaste.com](https://www.snipaste.com/)（核对 2026-09-05）；[EULA](https://www.snipaste.com/eula.html)（页内 Last updated 2024/07/10，核对 2026-09-05）；[Snipaste/bitblt-hdr](https://github.com/Snipaste/bitblt-hdr)（核对 2026-09-05）。

3. **网速本身（低）；嵌进 Win11 任务栏（高）**  
   上传/下载速率可用文档化 IP Helper（`GetIfTable` / `GetIfTable2`）。难的是 **把窗口 `SetParent` 进 `Shell_TrayWnd`**：Win11 居中任务栏、小组件重叠、图标占满后遮挡——TrafficMonitor 官方 FAQ 写明部分问题「目前无法解决」。出处：[GetIfTable](https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getiftable)（核对 2026-09-05）；[TrafficMonitor Help.md](https://github.com/zhongyang219/TrafficMonitor/blob/master/Help.md)（核对 2026-09-05）；源码 `TaskBarDlg.cpp` 第 1017 行 `SetParent`（`gh api` 核对 2026-09-05）。

**许可证红线（实现前必须定）：** 不能把 TranslucentTB（GPL-3.0）或 TrafficMonitor（Anti-996）的代码拷进闭源合集。Snipaste 闭源，EULA 禁止反编译。能学的是行为与 Microsoft 文档 API，不是抄仓库。

---

## 2. 三款对标拆解

### 2.1 Snipaste（截图 / 贴图）

官方站：[https://www.snipaste.com/](https://www.snipaste.com/)（核对 2026-09-05）  
文档入口：[https://docs.snipaste.com](https://docs.snipaste.com)（官网链出，核对 2026-09-05）  
GitHub 只有反馈仓，**没有应用源码**：[Snipaste/feedback](https://github.com/Snipaste/feedback)（核对 2026-09-05）

#### 核心功能清单（Joe 可能要对齐的）

来自官网首页 + [PRO wiki](https://github.com/Snipaste/feedback/wiki/PRO)（wiki 最近编辑 2026-01-14，页面抓取失败一次，功能表以官网与下载页 changelog 为准；PRO 对照表以搜索摘要 + changelog 交叉，未完整打开 wiki 正文时不当成唯一证据）。

**免费档（个人使用）应对齐的核心：**

| 能力 | 证据 |
| --- | --- |
| 区域截图；自动检测 UI 元素；像素级移动 | [snipaste.com](https://www.snipaste.com/) 2026-09-05 |
| 多屏、HiDPI | 同上 |
| 取色（F1 / C / F3） | 同上 |
| 历史回放（`,` / `.`） | 同上 |
| 截完 F3 贴成置顶悬浮窗 | 同上；FAQ 明确必须后台常驻，因为它是 paste 工具：[FAQ](https://www.snipaste.com/faq.html) 2026-09-05 |
| 贴图来源：剪贴板图像 / 纯文本 / HTML / 颜色 / 常见图片文件（含 GIF） | 官网 2026-09-05 |
| 贴图：缩放、旋转、翻转、透明度、鼠标穿透、缩略图、隐藏、分组、自动备份恢复 | 官网 2026-09-05 |
| 标注：矩形、椭圆、折线、箭头、铅笔、马克笔、文字、马赛克、高斯模糊、橡皮、撤销重做 | 官网 2026-09-05 |
| 全局热键、托盘、便携版 | 官网 2026-09-05 |

**不要在 MVP 对齐的 Pro 能力（changelog / PRO 页）：** OCR、条码/二维码、Super-snip、圆角截图、虚拟桌面贴图、编号标注、双向箭头、滚动截图/GIF 录制进度页、命令行高级输出组合。出处：[download.html changelog](https://www.snipaste.com/download.html)（Latest **2.11.3**，2026-01-18，核对 2026-09-05）；[EULA](https://www.snipaste.com/eula.html)（2.x 商业用途要 Pro）。

#### 开源与否、许可证、语言/框架

| 项 | 事实 | 出处 / 核对日 |
| --- | --- | --- |
| 开源 | **否。** 组织仓只有 feedback、translations、qt-patches、qt、bitblt-hdr 等，无应用源码 | `gh api orgs/Snipaste/repos` 2026-09-05 |
| 许可证 | 专有 EULA。1.x 个人/商业免费；2.x 仅个人免费，商业要 Pro。禁止修改、逆向、反编译 | [eula.html](https://www.snipaste.com/eula.html) 2026-09-05 |
| 最新桌面版 | **2.11.3**（2026-01-18）；1.x 停在 **1.16.2**（2018-01-22） | [download.html](https://www.snipaste.com/download.html) 2026-09-05 |
| 框架 | **Qt。** 组织有 `Snipaste/qt-patches`（“Modifications made to Qt for Snipaste”，last push 2024-12-05）；changelog v2.8 写 “Upgrade to Qt 6.2.4” | `gh api repos/Snipaste/qt-patches`；[download.html](https://www.snipaste.com/download.html) |
| 故障排除里出现 Qt5 DLL | 中文故障排除页提到缺少 `Qt5***.dll` | [故障排除 wiki](https://github.com/Snipaste/feedback/wiki/%E6%95%85%E9%9A%9C%E6%8E%92%E9%99%A4)（搜索摘要，2026-09-05；完整 wiki 页本次 fetch 失败） |

`Snipaste/feedback`：3685 stars，844 open issues，last push 2026-01-08，**无 license 字段**（`gh api` 2026-09-05）。这是反馈仓，不是产品源码。

#### 实现手段摘要（有证据才写）

Snipaste **没有公开实现**。能确认的旁证：

- **Qt 6**（见上）。
- 截图路径至少用过 **GDI BitBlt 一类 API**：官方旁路项目 [Snipaste/bitblt-hdr](https://github.com/Snipaste/bitblt-hdr) 写明 “Fixes overexposed hdr screenshot for softwares that using Bitblt api”，并用 DXGI Desktop Duplication + DX11 compute shader；测试列表含 Snipaste 2.10.6。许可证 MIT。last push 2025-12-07（`gh api` 2026-09-05）。
- changelog **v2.11**（2025-12-08）：“Support HDR color correction”、ARM64、更可靠的显示器插拔、Win11 UI 元素检测改进。说明 HDR 与多屏是一等公民问题。
- 贴图 = 置顶无边框窗口（缩放/透明/穿透）。具体是 `WS_EX_LAYERED` + `UpdateLayeredWindow` 还是 Qt 无边框窗，**找不到源码，不能写死**。
- 元素检测：changelog 多次提到重构；Firefox 无障碍设置会影响检测（故障排除搜索摘要）。具体 Win32 Accessibility / UI Automation 调用 **找不到公开代码**。

Microsoft 侧、后来者应优先用的**文档化**截图 API（不是 Snipaste 源码，是平台能力）：

- **Windows.Graphics.Capture** + `IGraphicsCaptureItemInterop::CreateForMonitor` / `CreateForWindow`：Win10 1903+（build 18362）。[CreateForMonitor](https://learn.microsoft.com/en-us/windows/win32/api/windows.graphics.capture.interop/nf-windows-graphics-capture-interop-igraphicscaptureiteminterop-createformonitor)（核对 2026-09-05）；[Screen capture](https://learn.microsoft.com/en-us/windows/apps/develop/media-authoring-processing/screen-capture)（页内 updated 2026-08-27，核对 2026-09-05）。Picker 路径会画系统黄框；Interop 路径可对 HWND/HMONITOR 静默截（仍受 `SetWindowDisplayAffinity` 等保护影响）。
- **DXGI Desktop Duplication**（`IDXGIOutputDuplication::AcquireNextFrame`）：Win8+，适合连续抓帧/HDR 相关工作。[Desktop Duplication API](https://learn.microsoft.com/en-us/windows/win32/direct3ddxgi/desktop-dup-api)（页内 updated 2025-04-15，核对 2026-09-05）。

#### 已知坑

| 坑 | 证据 |
| --- | --- |
| 必须常驻后台 | [FAQ](https://www.snipaste.com/faq.html) 2026-09-05 |
| SmartScreen / 未签名（桌面版） | 中文故障排除：尚未数字签名，需「仍要运行」（搜索摘要 2026-09-05；完整 wiki fetch 失败） |
| 以管理员运行导致开机启动失败 | 同上；与 TrafficMonitor FAQ 同类 |
| DPI≠100% 时浏览器元素检测不准 | 同上 |
| 网课/防截图软件会杀截图进程 | 同上 |
| HDR 下 BitBlt 过曝 | [Snipaste/bitblt-hdr](https://github.com/Snipaste/bitblt-hdr)；changelog v2.11 HDR |
| 多屏插拔、跨 DPI 移动贴图曾崩溃 | changelog v2.5「Fixed crash due to moving image window across screens with different DPIs」等 |
| 受保护窗口 / DirectX 游戏黑屏 | 第三方 FAQ 站点有此说；**非官网，仅作风险提示，不当成 Snipaste 官方声明** |
| 不能当实现参考去逆向 | [EULA §3](https://www.snipaste.com/eula.html) |

---

### 2.2 TranslucentTB（任务栏透明 / 亚克力）

官方仓：[https://github.com/TranslucentTB/TranslucentTB](https://github.com/TranslucentTB/TranslucentTB)  
站点：[https://translucenttb.github.io](https://translucenttb.github.io)  
Store：`9PF4KZ2VN4W9`（README 链出）

`gh api` 核对 2026-09-05：

| 字段 | 值 |
| --- | --- |
| stars | **20260** |
| forks | 1251 |
| license | **GPL-3.0**（`LICENSE.md`） |
| language | **C++**（C++ 537111 / C 4928 / PowerShell / CMake） |
| latest release | **2026.2**，published **2026-08-31T21:01:53Z** |
| last push | 2026-08-31 |
| topics | 含 `undocumented`, `acrylic`, `taskbar`, `windows-11` 未在 topics 列表；README 写明 Win10/11 |
| open issues | 327 |

#### 核心功能清单（应对齐的）

来自 README（核对 2026-09-05）：

- 任务栏状态（互斥选一，除 Normal 外可调色）：**Normal / Opaque / Clear / Blur / Acrylic**
- Blur：**仅 Windows 10 与 Windows 11 build 22000**
- Acrylic：Fluent 风格磨砂
- 动态模式（可叠加）：有可见窗口 / 最大化窗口 / 开始菜单开 / 搜索开 / 任务视图开
- Win10：按动态模式显示或隐藏 Aero Peek 按钮
- Win11：按动态模式显示或隐藏任务栏底线
- 带 alpha 的取色器 + 实时预览
- 兼容 RoundedTB、ExplorerPatcher（README 声明；RoundedTB 仓库已 archived，见第 5 节）
- 便携版：**仅 Windows 11**
- 托盘「开机启动」；企业策略下可能灰掉，README 给出注册表键

Joe 的口述是「任务栏透明 / 亚克力」。**动态模式、Peek 按钮、底线开关不是 MVP 必须。**

#### 开源与否、许可证、语言/框架

- 开源，**GPL-3.0**。衍生作品必须 GPL。闭源合集 **不能直接链/拷其 ExplorerTAP**。
- C++ / Win32 + **XAML Islands**（UI）+ **ExplorerTAP**（注入 explorer）+ **ExplorerHooks**（detour）。
- 构建：Visual Studio 2026 + vcpkg（[CONTRIBUTING.md](https://github.com/TranslucentTB/TranslucentTB/blob/release/CONTRIBUTING.md)，核对 2026-09-05）。「At this time we have no plans of expanding this beyond the taskbar.」
- UI 用 Microsoft.UI.Xaml（动态依赖 UWP 包）。README：Win11 便携版；Store 版自动更新。

#### 实现手段摘要（源码证据）

`gh api` 文件树 + `gh search code`（2026-09-05）：

| 手段 | 证据 |
| --- | --- |
| 未文档 `SetWindowCompositionAttribute` | `TranslucentTB/dynamicloader.hpp` 从 user32 `GetProcAddress`；`taskbarattributeworker.cpp` 调用；`ExplorerHooks/swcadetour.cpp` detour |
| 未文档头 | `Common/undoc/explorer.hpp`, `user32.hpp`, `uxtheme.hpp`, `winternl.hpp`, `winuser.hpp` |
| Win11 XAML 任务栏：注入 ExplorerTAP | 目录 `ExplorerTAP/`：`InitializeXamlDiagnosticsEx`（`tapsite.cpp` / `api.cpp`）、`VisualTreeWatcher`、改 `Taskbar.TaskbarFrame` 的 brush（`XamlBlurBrush`） |
| XAML Island 探测 | 源码注释：22621+ 且存在 `Windows.UI.Composition.DesktopWindowContentBridge`、无 `WorkerW` 则视为新任务栏（commit 讨论 / DeepWiki 二次来源；**以仓库文件名为准**） |
| 2026.2 明确避开现代 XAML 任务栏上的 ExplorerHooks | [Release 2026.2 body](https://github.com/TranslucentTB/TranslucentTB/releases/tag/2026.2)（`gh api` 2026-09-05）：“Avoid ExplorerHooks in modern XAML taskbar” |
| Microsoft Q&A 旁证 | 「TranslucentTB calls SetWindowCompositionAttribute」：[learn.microsoft.com Q&A](https://learn.microsoft.com/en-us/answers/questions/1720291/taskbar-shell-traywnd-transparency-like-in-translu)（提问 2024-06-20，核对 2026-09-05） |

**没有证据**表明它用公开的 WinUI `AcrylicBrush` 去刷系统任务栏。任务栏不是自家窗口，必须改 explorer 的 visual tree 或 composition 属性。

作者 sylveon 在 [WindowsAppSDK discussion #2711](https://github.com/microsoft/WindowsAppSDK/discussions/2711) 写过：`SetWindowCompositionAttribute` 是未文档技巧，曾被 Windows 更新弄坏又修好。核对 2026-09-05。

#### 已知坑

| 坑 | 证据 |
| --- | --- |
| 未文档 API，随 Windows 更新断裂 | 仓库 topic `undocumented`；discussion #2711；Blur 在 Win11 被 OS 拿掉（市场页/旧 release 笔记，README 仍写 Blur 仅 22000） |
| 必须注入 explorer；杀软误报 | README Security：「Some antiviruses are over eager」；建议自编译 |
| 多显示器任务栏更新失败 | 2026.2 bugfix：「would not be able to update the taskbar on certain monitors」 |
| 重入崩溃 | 2026.2：「resilient against re-entrency」 |
| 与其它改任务栏工具冲突 | README 只声明兼容 RoundedTB / ExplorerPatcher，未保证全部 |
| 开机启动被策略关掉 | README 注册表 `EnableFullTrustStartupTasks` 等 |
| 便携版仅 Win11 | README |
| GPL-3.0 传染 | 不能闭源复用其注入代码 |

---

### 2.3 TrafficMonitor（桌面/任务栏网速）

仓：[https://github.com/zhongyang219/TrafficMonitor](https://github.com/zhongyang219/TrafficMonitor)

`gh api` 核对 2026-09-05：

| 字段 | 值 |
| --- | --- |
| stars | **46032** |
| forks | 3787 |
| license SPDX | **NOASSERTION**（GitHub 无法映射到 OSI 标准许可证） |
| 实际文本 | **Anti 996 License Version 1.0 (Draft)**，[LICENSE](https://github.com/zhongyang219/TrafficMonitor/blob/master/LICENSE) |
| language | **C++**（C++ 1400360 / C 42986），MFC（`CMFCColorDialogEx`、`.sln`） |
| latest release | **V1.86**，2026-03-29 |
| last push | **2026-08-31**（master 仍有提交，领先 release 68 commits） |
| open issues | 1347 |

#### 核心功能清单（应对齐的）

来自 [README.md](https://github.com/zhongyang219/TrafficMonitor/blob/master/README.md)（核对 2026-09-05）：

Joe 口述只要「桌面/状态栏网速（上传下载实时显示）」。仓库还包含大量 Joe **不必对齐** 的能力。

**应对齐：**

- 实时上传/下载速率
- 多网卡自动/手动选择
- 桌面悬浮窗（可穿透、置顶、皮肤——皮肤可降级）
- （可选）嵌入任务栏显示

**不应在 v1 对齐：**

- CPU/内存/GPU/硬盘/温度（标准版要管理员；温度依赖 LibreHardwareMonitor，作者警告崩溃/蓝屏）
- 历史流量统计、插件系统、丰富皮肤
- README 1.86 起：Lite 也有 GPU/磁盘占用；温度将迁到插件；后续可能只出 Lite

版本对照（README 表）：网速、CPU/内存 Lite 与标准版都有；温度仅标准版；**标准版要管理员，Lite 不要**。作者建议无温度需求用 Lite。

#### 开源与否、许可证、语言/框架

- 开源，但许可证是 **Anti-996**，不是 GPL（部分转载站写成 GPL，**与仓库 LICENSE 不符，以仓库为准**）。商业闭源合集使用其代码前需法务看 Anti-996 劳动合规条款；GitHub SPDX 为 `NOASSERTION`。
- C++ / **MFC** 对话框；任务栏窗口 `TaskBarDlg` / `Win11TaskbarDlg`；绘制有 GDI 与 Direct2D（`D2D1Support.cpp`、Help.md 1.85+ HDR 建议改 D2D）。

#### 实现手段摘要（源码证据）

`gh search code` + 文件内容（2026-09-05）：

| 手段 | 证据 |
| --- | --- |
| 网速 | `GetIfTable`（`TrafficMonitorDlg.cpp`）；`GetAdaptersInfo`（`AdapterCommon.cpp`）；`MIB_IFTABLE` |
| 任务栏宿主 | `FindWindow("Shell_TrayWnd")`；`SetParent(this->m_hWnd, GetParentHwnd())`（`TaskBarDlg.cpp:1017`，注释「把程序窗口设置成任务栏的子窗口」） |
| Win11 判定 | `TrafficMonitor.cpp` 注释：在 `Shell_TrayWnd` 子窗口找到 `Windows.UI.Composition.DesktopWindowContentBridge` 则视为 Win11 任务栏 |
| 分层窗口 | `WS_EX_LAYERED`；D2D 路径用 alpha 混合 |
| DPI | `CTaskBarDlg::GetDPI/SetDPI`；2026-08-19 commit「app初始化时获取当前屏幕DPI」 |
| 硬件（非网速） | LibreHardwareMonitor；WinRing0 驱动曾被 Defender 报 `VulnerableDriver:WinNT/Winring0` |

平台文档（应自己实现时用，不要抄 Anti-996 代码）：

- [GetIfTable](https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getiftable)（Win2000+，`Iphlpapi.dll`）
- [GetIfTable2](https://learn.microsoft.com/en-us/windows/win32/api/netioapi/nf-netioapi-getiftable2) / [GetIfEntry2](https://learn.microsoft.com/en-us/windows/win32/api/netioapi/nf-netioapi-getifentry2)（Vista+，含逻辑网卡）

速率 = 两次采样 `InOctets`/`OutOctets` 之差 / 间隔。不需要驱动。

#### 已知坑

来自 [Help.md](https://raw.githubusercontent.com/zhongyang219/TrafficMonitor/master/Help.md)（核对 2026-09-05）及 issues：

| 坑 | 证据 |
| --- | --- |
| Win11 与小组件重叠 | FAQ：勾选「避免与右侧小组件重叠」 |
| Win11「显示在任务栏左侧」仅居中任务栏可用 | FAQ |
| 任务栏占满后与图标重叠 | FAQ：**目前无法解决**，不要再反馈 |
| 网速一直 0 | 网卡切换；需刷新连接或关掉自动选择 |
| 睡眠恢复后丢网速 | V1.86 修复说明（release 2026-03-29） |
| 通知区宽度变化导致显示不全 | FAQ：轮询位置，跟不上 |
| Win10 HDR 下任务栏窗消失 | 关背景透明或改 Direct2D |
| 开机启动：管理员 / 安全软件 / 移动路径 | FAQ |
| **WinRing0 / TrafficMonitor.sys 被 Defender 隔离** | [#2331](https://github.com/zhongyang219/TrafficMonitor/issues/2331)（2026-05-15，1.86）；[#2156](https://github.com/zhongyang219/TrafficMonitor/issues/2156) 等。这是硬件监控驱动，**纯网速 Lite 路径可避开** |
| 副屏任务栏 | V1.86 新增；说明此前是缺口 |

---

## 3. 合集方案对比

评价维度：三功能匹配度、Win11 寿命、Grok Build 写代码友好度（Joe 主语言 Python；C#/Rust 熟悉度未知）、许可证。

### A. 原生：C# + WinUI 3 / WPF

| | |
| --- | --- |
| 适合 | 截图（`Windows.Graphics.Capture` 官方示例就是 C#/WinUI 3）；置顶贴图（WPF `Topmost` + 分层窗成熟；WinUI 3 透明分层窗有已知坑）；网速（P/Invoke `Iphlpapi`）；托盘/安装包（MSIX 或 unpackaged + bootstrapper）；ShareX 同语言可对照行为（**不能闭源拷 GPL 代码**）；Windows App SDK 文档密、Grok 训练语料多 |
| 不适合 | 任务栏亚克力：C# 也能 P/Invoke `SetWindowCompositionAttribute`、也能写注入 DLL，但 **TAP/detour 本质仍是 C++ 级 Win32**，WinUI 帮不上忙。WinUI 3 透明无边框贴图窗：[castorix/WinUI3_SwapChainPanel_Layered](https://github.com/castorix/WinUI3_SwapChainPanel_Layered) 自述 WASDK 1.1 后 layered 曾坏，Win11 还有边框/阴影问题（核对 2026-09-05，59 stars，非正式文档） |
| 匹配度 | 截图 **高**；贴图 **WPF 高 / WinUI 3 中**；网速悬浮窗 **高**；任务栏嵌入 **中（SetParent 与 UI 框架无关）**；任务栏亚克力 **低（除非另写 native 模块）** |
| Grok Build | **三档里最高。** C# + P/Invoke + WinRT 示例多；unpackaged 部署有官方指南：[Deploy unpackaged apps](https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/deploy-unpackaged-apps)（页内 ms.date 2026-05-29，核对 2026-09-05）。Joe 若只会 Python，C# 语法坡度仍小于 C++/Rust Win32。 |
| 建议切分 | **设置/托盘用 WinUI 3 或 WPF；截图与贴图用 WPF 或纯 Win32 HWND；不要让 WinUI 3 窗口去 SetParent 进任务栏。** |

### B. C++/Rust + Win32

| | |
| --- | --- |
| 适合 | 与 TranslucentTB、TrafficMonitor、Flameshot 同一世界。ExplorerTAP、`SetWindowCompositionAttribute`、分层窗、`RegisterHotKey`、低 RAM。Rust 有 `windows-rs`。 |
| 不适合 | Joe 主 Python；Grok 能写，但 **注入 explorer 的 review/调试成本** 远高于截图。VS2026 + vcpkg（TranslucentTB 现状）环境重。Rust 生命周期 + COM 对「周末合集」不友好。 |
| 匹配度 | 三功能 **都高**（能力上）。寿命仍绑在未文档任务栏上。 |
| Grok Build | **中偏低。** 能生成样板，难保证 TAP 在下一个 Windows 补丁仍活。Rust 友好度低于 C#。 |
| 何时选 | 只有在 **明确要做任务栏亚克力、且接受 GPL 或完全自写注入** 时，把该模块做成 C++ DLL；主程序仍可用 C#。 |

### C. 跨端壳：Tauri / Electron

| | |
| --- | --- |
| 适合 | 设置页、关于页、热键插件（Tauri `tauri-plugin-global-shortcut` / Electron `globalShortcut`）。Electron `desktopCapturer` 能抓屏/窗：[desktopCapturer](https://www.electronjs.org/docs/latest/api/desktop-capturer)（核对 2026-09-05）。 |
| 不适合 | **三类核心都别走壳。** ① 截图要全屏遮罩、像素级选区、不含黄框、不含 WebView 自身——Electron 的 getDisplayMedia 是会议共享模型，不是 Snipaste。② 贴图要大量轻量分层 HWND，WebView 每个贴图一张太重。③ 任务栏亚克力必须进 explorer，WebView 做不到。④ 网速可做，但为几个数字背 Chromium 不合理。Tauri 截的是 **WebView 内容** 不是桌面（`reticle-tauri` 等工具自述 Windows CapturePreview 未实测，且「renders the webview, not the screen」——二次来源，仅作旁证）。 |
| 匹配度 | 截图 **低**；贴图 **低**；网速 **中（浪费）**；任务栏 **无** |
| Grok Build | 前端高、Win32 边界低。Joe 会 Python 也不等于会 Rust（Tauri）或 Electron 主进程。 |
| 结论 | **明显不适合作为主架构。** 最多将来给设置页嵌 WebView，不建议现在引入。 |

### 附：Python（Joe 母语，不是正式三档，但必须说清）

`ctypes` / `pywin32` 调 `GetIfTable`、甚至有 [littlewhitecloud/TranslucentTB](https://github.com/littlewhitecloud/TranslucentTB) 这种 Python 调 blur 的实验仓；[DXcam](https://github.com/ra1nty/DXCam) 用 Desktop Duplication。  
**可以用来周末验证网速数字，不能当产品壳：** 打包体积、托盘、DPI、热键、代码签名、杀软、无边框贴图，Python 运行时全是负债。Grok Build 写 Python Win32 可行，但交付形态应是 C#/C++ 可执行文件。

---

### 三档对照（给拍板用）

| | A C# WinUI3/WPF | B C++/Rust Win32 | C Tauri/Electron |
| --- | --- | --- | --- |
| 截图+贴图 | 高（贴图偏 WPF） | 高 | 低 |
| 桌面网速 | 高 | 高 | 浪费 |
| 任务栏嵌入网速 | 中（SetParent） | 中 | 低 |
| 任务栏亚克力 | 低（需 native 旁路） | 高（仍脆弱） | 无 |
| 一个安装包 | 容易 | 容易 | 容易但巨大 |
| Grok + Joe | **最好** | 差 | 设置页还行，核心不行 |
| 建议 | **主路径** | 仅亚克力模块 | 不采用 |

---

## 4. 推荐栈 + MVP 边界

### 推荐栈

1. **一个 unpackaged（或 MSIX）安装包**，开机可选启动，托盘常驻。
2. **主进程：C# + .NET 8（或更新 LTS）+ WPF** 做截图选区、标注、贴图分层窗、桌面网速悬浮窗、设置。WinUI 3 可做设置页，但 **贴图窗优先 WPF/Win32 HWND**，避开 WinUI layered 坑。
3. **截图 API 顺序：** `IGraphicsCaptureItemInterop.CreateForMonitor`（静默、Win10 1903+）→ 失败再 DXGI Desktop Duplication → 最后 GDI `BitBlt`（HDR 会烂，当兼容层）。
4. **网速：** `GetIfTable2` 周期采样；**v1 只做桌面悬浮窗**，不做 `SetParent` 进任务栏。
5. **任务栏亚克力：v1 不做。** 若以后做：独立进程、独立崩溃、独立杀软说明；自写 composition 调用，**不要 fork GPL 的 ExplorerTAP 进闭源**；并接受「下一个 Dev 频道就可能挂」。
6. **不要** Electron/Tauri 做核心；**不要** Python 做发布形态。
7. **Grok Build：** 用 C# 写主程序最划算。Joe 用 Python 做网速算法/DPI 实验可以，合入时译成 C#。

目标系统：**Windows 10 1903+ / Windows 11**（Graphics Capture 下限）。不承诺 Win7（Snipaste 1.x 才管）。

### 建议第一版做哪 2–3 个能力

只做这三件（其实是两条产品线 + 一个小窗）：

1. **区域截图**：全屏遮罩、拖矩形、Esc 取消、复制到剪贴板、保存 PNG。全局热键（注意 Win11「PrtScr 打开截图工具」系统开关，ShareX 踩过）。
2. **贴图**：截完或从剪贴板钉成置顶窗；拖动、滚轮缩放、透明度、关闭、鼠标穿透。不要分组/备份/GIF/虚拟桌面。
3. **桌面网速悬浮窗**：上/下行 KB/s，置顶，可拖，选网卡。不要任务栏嵌入、不要 CPU 温度。

标注：**矩形 / 箭头 / 文字 / 马赛克** 四件套即可，不要高斯模糊全套画板。

### Out of scope（别一口吃成 Snipaste Pro）

- OCR、二维码、滚动长截图、GIF 录制、Super-snip、白板、贴图分组与自动恢复、命令行矩阵
- 任务栏 Acrylic/Blur、动态模式、改 explorer
- 网速嵌入任务栏、副屏任务栏窗、皮肤引擎
- 硬件温度 / WinRing0 / PawnIO
- 与 ExplorerPatcher/RoundedTB 联调
- 商店上架、跨 Windows 7
- 抄 TranslucentTB / TrafficMonitor / ShareX 源码进闭源包

---

## 5. 可参考的开源仓库

均为 `gh api repos/...` 核对 **2026-09-05**。许可证以 GitHub `license.spdx_id` 为准。

| owner/repo | 用途 | 许可证 | 语言 | stars | 最近活跃 | 备注 |
| --- | --- | --- | --- | --- | --- | --- |
| [ShareX/ShareX](https://github.com/ShareX/ShareX) | 开源截图+标注+**Pin to screen**（最接近 Snipaste 的开源对照） | GPL-3.0 | C# | **39447** | push 2026-09-04；release v21.0.0 于 2026-07-03 | 可对照行为与 GDI/DWM 辅助函数；**闭源产品不能拷代码**。Pin 文档：[getsharex.com/docs/pin-to-screen](https://getsharex.com/docs/pin-to-screen.html) |
| [TranslucentTB/TranslucentTB](https://github.com/TranslucentTB/TranslucentTB) | 任务栏效果怎么做的「存在证明」 | GPL-3.0 | C++ | **20260** | push 2026-08-31；release 2026.2 | 学架构，不要闭源 fork TAP |
| [zhongyang219/TrafficMonitor](https://github.com/zhongyang219/TrafficMonitor) | 网速+任务栏嵌入怎么做的 | Anti-996（SPDX NOASSERTION） | C++/MFC | **46032** | push 2026-08-31；release V1.86 2026-03-29 | 学 `GetIfTable`/`SetParent` 行为；许可证特殊 |
| [flameshot-org/flameshot](https://github.com/flameshot-org/flameshot) | 跨平台截图标注 | GPL-3.0 | C++ | **30775** | push 2026-09-02 | Windows 不是第一公民；HDR BitBlt 旁路测过它 |
| [greenshot/greenshot](https://github.com/greenshot/greenshot) | Windows 截图 | GPL-3.0 | C# | **5107** | push 2026-07-06 | 比 ShareX 小 |
| [microsoft/WindowsAppSDK](https://github.com/microsoft/WindowsAppSDK) | WinUI 3 / 部署 | MIT | C++ | **4674** | push 2026-09-03 | 官方运行时 |
| [LibreHardwareMonitor/LibreHardwareMonitor](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor) | 仅当以后做温度 | MPL-2.0 | C# | **9004** | push 2026-09-04 | TrafficMonitor 依赖源；与网速无关 |
| [valinet/ExplorerPatcher](https://github.com/valinet/ExplorerPatcher) | Win11 任务栏改版对照 | GPL-2.0 | C | **33792** | push 2026-07-06 | 高风险系统补丁，**不要做成合集依赖** |
| [RoundedTB/RoundedTB](https://github.com/RoundedTB/RoundedTB) | 圆角任务栏 | GPL-3.0 | C# | **3126** | **archived**；last push **2023-10-16** | 已停更，只作历史 |
| [tauri-apps/tauri](https://github.com/tauri-apps/tauri) | 若有人坚持壳 | Apache-2.0 | Rust | **110816** | push 2026-09-04 | 不推荐作本产品主栈 |
| [Snipaste/feedback](https://github.com/Snipaste/feedback) | 需求/坑 | 无 license | — | 3685 | push 2026-01-08 | 不是源码 |
| [Snipaste/bitblt-hdr](https://github.com/Snipaste/bitblt-hdr) | HDR+BitBlt 问题本身 | MIT | — | 0 | push 2025-12-07 | 说明 BitBlt 不够 |

**核对不了 / 不采用：** PixPin 等国内闭源截图工具本次未用 `gh api` 核到可引用的官方源码仓。`www.trafficmonitor.cn` 自称 GPL，**与 GitHub LICENSE（Anti-996）冲突，以 GitHub 为准。**

---

## 6. 下一步给产品/架构的问题清单（最多 5 个）

1. **Win11 任务栏亚克力是 v1 必须，还是「做得出算赚」？**  
   若必须，就要单独进程 + 接受 explorer 注入 + GPL 隔离 + 每个 Windows 更新回归。若不算必须，MVP 立刻可开工。

2. **网速要嵌进任务栏，还是桌面悬浮窗就够？**  
   嵌入 = `SetParent(Shell_TrayWnd)`，Win11 小组件/占满/居中都是已知死角。悬浮窗一周能做完。

3. **发行形态：闭源安装包，还是 GPL 开源？**  
   闭源就不能吃 TranslucentTB/ShareX/Flameshot 代码。开源 GPL 才能认真 fork TAP。Anti-996 的 TrafficMonitor 即使开源合集也不一定能合。

4. **最低系统：只 Win11，还是 Win10 1903+？**  
   决定能不能只用 Graphics Capture、要不要 BitBlt 回退、便携版策略（TranslucentTB 便携仅 Win11）。

5. **Joe 写不写 C#？**  
   若坚持 Python 交付，合集三件套会在打包、DPI、签名、贴图窗上反复付税。需要明确：Python 只做原型，还是甘愿产品是 `.py` 托盘。

---

## 附录：一个安装包下的建议进程图（非实现）

```
Installer
 └── Suite.exe          C# WPF：托盘、热键、截图、贴图、网速悬浮
 └── (optional) TaskbarFx.exe   以后再说：任务栏外观，崩了不影响截图
```

不要让截图热键和 explorer 注入共享一个 UI 线程。

核对完成时间：2026-09-05。链接总表见同目录 `SOURCES.md`。
