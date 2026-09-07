# 任务：Windows 合集 App 架构方案（ADR 级）

你是远程 Grok Build 的「架构师」。只做架构：模块边界、数据流、进程模型、技术选型约束、ADR。不写业务代码，不定视觉，不出面试材料，不做安全审计全文（可列安全触点交 安全审计）。

先读：
- /workspace/windows-suite-app/research/REPORT.md
- /workspace/windows-suite-app/research/ADDENDUM-theme.md
- /workspace/windows-suite-app/research/SOURCES.md
- /home/box/TEAM.md（协议与可见跑）

## Joe 已拍板（2026-09-05，拟合app 对话）

1. **v1 必须**有任务栏透明/亚克力；接受**单独进程 + 注入 explorer** 风险。
2. **闭源安装包**；任务栏效果**自写**，**不拷 GPL**（TranslucentTB/ShareX/Auto Dark Mode 等）代码；不拷 Anti-996（TrafficMonitor）代码。可学行为与 Microsoft 文档 API。
3. 最低 **Windows 10 1903+**，并支持 **Windows 11**（任务栏路径必须分 Win10 Win32 / Win11 XAML 两套策略）。
4. 网速：**桌面悬浮窗为主**，任务栏嵌入为**可选**（可 v1.x）。
5. 另有：**一键系统深浅色**（HKCU AppsUseLightTheme/SystemUsesLightTheme + ImmersiveColorSet 广播）。
6. 实现：主推 **C# + .NET LTS + WPF**；**Grok Build 写，Joe 审**。
7. 功能合集：Snipaste 式截图/贴图、TranslucentTB 式任务栏透明/亚克力、TrafficMonitor 式网速、系统深浅切换。

## 产出

写到：`/workspace/windows-suite-app/architecture/ADR.md`

中文。每个决策写：**为什么选、为什么不选、贸易代价**。引用调研报告路径作出处；涉及外部事实标链接/日期；推断标「推断」。

### ADR.md 必含

1. **目标与非目标**（v1 In / Out）
2. **进程与部署模型**  
   - Suite 主进程 vs TaskbarFx 进程  
   - IPC 方式（命名管道 / 其他）与崩溃隔离  
   - 安装包形态（unpackaged vs MSIX）推荐与代价
3. **模块边界**（包/项目建议）  
   - Capture / Pinboard / NetSpeed / ThemeToggle / TrayShell / TaskbarFx  
   - 哪些必须 native C++/C# PInvoke，哪些纯托管
4. **关键数据流**（简图即可，可用 mermaid）  
   - 热键 → 截图 → 剪贴板/贴图  
   - 网速采样 → UI  
   - 主题切换 → 广播 → 各窗刷新  
   - 主进程 ↔ TaskbarFx（启用/禁用效果、色值、动态模式是否 v1）
5. **Win10 vs Win11 任务栏策略**（自写、不拷 GPL）  
   - v1 最低可用效果（透明/opaque/acrylic 哪几档）  
   - 未文档 API 的风险与降级  
   - 杀软/签名注意点（列触点，细节可交安全审计）
6. **仓库与解决方案草图**（目录树，尚不要建 GitHub repo 除非必要）
7. **分阶段交付**（P0/P1/P2，每阶段可验证标准）
8. **开放风险**（最多 8 条）与建议下一步（是否立刻开 create-plan / 安全审计 / 建空仓）

## 约束

- 不要 git push；不要建无关仓库（本机目录可以写文档）
- 不要抄写 GPL/Anti-996 源码进文档当「可实现片段」；只描述公开 API 与进程职责
- 写完 stdout 打印 `DONE` 和 ADR.md 路径

日期：2026-09-05 Asia/Shanghai。
