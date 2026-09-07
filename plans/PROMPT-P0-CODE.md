# 任务：实现 Suite P0 骨架（无注入）

你是远程 Grok Build 的「coding」。写代码、出可审 diff。视觉不定稿（设置窗能用即可）。

## 必读

1. `/workspace/windows-suite-app/plans/P0.md` — 范围与 action items（照做）
2. `/workspace/windows-suite-app/architecture/ADR.md` — §3 模块边界、§7 P0
3. `/workspace/windows-suite-app/security/AUDIT.md` — §4.1 P0-1…P0-7（门禁）

## Joe 拍板摘要

- 闭源；不拷 GPL / Anti-996
- P0：**无** TaskbarFx、**无** 管道、**无** SWCA/TAP、**无** Capture/Pinboard 完整实现
- C# + .NET 10 + WPF；Grok 写 Joe 审
- 开机启动：只 `HKCU\...\Run`
- Theme：翻两 DWORD + ImmersiveColorSet；**禁止**杀 explorer
- 热键：占位，可改；默认可用 `Ctrl+Shift+F12`（避开 PrtScr），写进 settings
- 本机是 Debian：可写源码与测试；**不要**声称 WPF `dotnet build` 已在本机绿。在 `plans/P0-HANDOFF.md` 写清 Joe 在 Windows 上要跑的命令。

## 目录

代码根：`/workspace/windows-suite-app/`

按 ADR/P0 建：

```
src/
  Suite.sln
  Directory.Build.props
  Suite.Contracts/
  Suite.Platform/
  Suite.NetSpeed/
  Suite.ThemeToggle/
  Suite.App/          # WinExe UseWPF → Suite.exe
tests/
  Suite.Contracts.Tests/
  Suite.ThemeToggle.Tests/
  Suite.NetSpeed.Tests/
.gitignore
```

**禁止**创建 TaskbarFx*、Suite.Capture、Suite.Pinboard。

## 实现要点

- `Directory.Build.props`：`net10.0-windows`、nullable、TreatWarningsAsErrors、平台 x64
- Contracts：settings schema（开机启动、网卡 IfIndex、热键、网速窗位置/可见）
- Platform：Mutex 单实例；HKCU Run 写删；RegisterHotKey；SendMessageTimeoutW；Registry 读 Personalize（可给 Theme 用）
- ThemeToggle：读-翻-写 AppsUseLightTheme + SystemUsesLightTheme；广播；ThemeChanged 事件
- NetSpeed：GetIfTable2/GetIfEntry2；1s 采样；选网卡；Topmost 可拖窗；PowerModeChanged 恢复
- App：托盘菜单（翻转主题、显隐网速、设置、开机启动、退出）；设置窗简易；热键占位气泡/日志
- Tests：Contracts/ThemeToggle（假注册表）/NetSpeed 能无 UI 测的部分；本机 Linux 若跑不通 Win 专有测试，用条件编译或注明「须 Windows」
- 根 `.gitignore`：bin/ obj/ *.user 证书
- **零** 网络客户端、**零** 来路不明 NuGet；NotifyIcon 用 WinForms 互操作或 P/Invoke Shell_NotifyIcon（二选一，写进 HANDOFF）
- settings 只在 `%LOCALAPPDATA%\Suite\settings.json`，不存截图像素/密钥

## 交付

1. 源码树完整可审
2. `/workspace/windows-suite-app/plans/P0-HANDOFF.md`：Windows 验证步骤（dotnet --info / build / test / 手工点检清单）
3. stdout：`DONE` + 关键路径列表

不要 git init/push（Joe 未要求建仓）。不要写注入相关符号「以后再用」。
日期：2026-09-05。
