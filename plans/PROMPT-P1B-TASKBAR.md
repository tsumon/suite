# 任务：P1b — TaskbarFx（Win10/经典任务栏透明，自写，不拷 GPL）

你是远程 Grok Build 的「coding」。在已有 Suite 上加任务栏效果进程。Joe 是小白：交接用大白话。

## 必读

- `/workspace/windows-suite-app/architecture/ADR.md` §2、§3、§4.4、§5、§7 P1
- `/workspace/windows-suite-app/security/AUDIT.md` **§4.2 P1-1…P1-8**（编码前门禁，必须写入实现）
- `/workspace/windows-suite-app/research/REPORT.md` §2.2（只学行为，**禁止**拷 TranslucentTB 源码/头文件/TAP）
- 现有 `/workspace/windows-suite-app/src/`

## 本轮 In（= ADR P1 任务栏部分）

1. **新建** `TaskbarFx.Host`（C# WinExe）+ `TaskbarFx.Native`（C++ DLL，MSVC 工程）
2. **命名管道 IPC**（Suite = client，Fx = server）
   - CurrentUserOnly；DACL **拒绝** World/Everyone
   - `FirstPipeInstance`
   - 管道名不可纯猜（含用户 SID 或随机+发现机制，写进 HANDOFF）
   - 对端校验方向：至少校验对端是同目录预期 EXE（Release 不得带宽松 Debug ACL）
   - 所有调用超时（建议 2s），不得阻塞截图热键
3. 协议最小集：Enable/Disable、SetAppearance{mode,argb}、GetStatus、Shutdown、Ping、Event
   - mode：normal | opaque | clear | acrylic
4. **Native Win10 路径**：对 `Shell_TrayWnd`（及能找到的 Secondary）用 **自写** `SetWindowCompositionAttribute`（GetProcAddress），**不**拷 GPL undoc 头
   - 失败 → status unavailable，设置页可见，不假成功
5. **Win11 XAML 路径本轮不做**：探测到现代 XAML 任务栏时 GetStatus=unavailable，UI 文案「Win11 路径未交付」；**禁止** `InitializeXamlDiagnosticsEx`
6. Suite.App：设置页增加任务栏效果开关与档位/颜色；打开效果时 CreateProcess 拉起同目录 TaskbarFx.exe（不提权）；关闭时 Disable+可选 Shutdown
7. 单实例 Mutex（TaskbarFx 自己一把）；崩溃隔离：Fx 挂不影响 Suite
8. `plans/P1B-HANDOFF.md` 大白话：怎么编 Native、怎么试、一键恢复默认、杀软可能弹窗

## Out

- Win11 TAP / ExplorerHooks / 动态模式 / Blur
- 拷贝 TranslucentTB / 任何 GPL 文件进树
- 网速 SetParent 进任务栏
- git push
- 声称 Debian 上编过 WPF/C++ 为绿

## 工程

- 加入 `Suite.sln`（或说明 VS 需同时开 vcxproj——优先一个 sln 能编 Host；Native 用 vcxproj，HANDOFF 写清 VS 工作负载）
- Suite **零** 注入符号；Native **只**链进 TaskbarFx
- 生产尽量少 NuGet；Contracts 共享 DTO

## 安全注释

代码里注释标明：未文档 API、可能随 Windows 更新断裂；禁止杀 explorer 当默认恢复手段。

写完：stdout `DONE` + 路径；更新 sln；rg 自检无 TranslucentTB 路径拷贝。
