# Suite P1b 交接（任务栏效果：经典 Win10 路径）

- **日期：** 2026-09-05
- **远程：** Debian 只出源码。**没有**在这台 Linux 上编过 WPF，也**没有**编过 C++ DLL。不要把远程当绿。
- **目标：** 另开 `TaskbarFx.exe` 改经典任务栏外观。截图还在 Suite 里。Win11 现代任务栏本轮只提示「Win11 路径未交付」，**不会**往 explorer 塞 DLL。

## 0. 你现在多了什么

| 东西 | 干什么 |
| --- | --- |
| `TaskbarFx.exe` | 小助手进程。听 Suite 的管道命令。自己崩了，截图/贴图/网速还在。 |
| `TaskbarFx.Native.dll` | C++ 小库。找到 `Shell_TrayWnd`（以及副屏 `Shell_SecondaryTrayWnd`），调用未文档的 `SetWindowCompositionAttribute`。 |
| 设置页「任务栏效果」 | 开关 + 档位（系统默认 / 实色 / 全透明 / 亚克力）+ 颜色。 |

**没有做（故意的）：** Win11 TAP、`InitializeXamlDiagnosticsEx`、ExplorerHooks、动态模式、Blur、把网速 `SetParent` 进任务栏、拷 TranslucentTB。

## 1. 先装 Visual Studio 工作负载（Windows 真机）

和 P0/P1a 一样需要 **VS 2026 / MSBuild 18** + **.NET 10 SDK**（带 `Microsoft.WindowsDesktop.App`）。

**本轮额外要：**

1. Visual Studio Installer → **使用 C++ 的桌面开发**（Desktop development with C++）
2. 勾上 **Windows 10/11 SDK**（跟 VS 推荐的最新即可）
3. 平台工具集：工程写的是 **v143**。若 VS 2026 弹出「升级到 v145」，**可以点升级**，只是编译器版本，不是功能。

架构：**x64**。不要 AnyCPU。

## 2. 怎么编

仓库根（本机路径以你放的为准；远程是 `/workspace/windows-suite-app/`）。

### 2.1 推荐：一个 VS 解决方案编完

打开 `src\Suite.sln`。

1. 上面配置选 **Debug**、**x64**（或 Release + x64）
2. 生成 → 生成解决方案
3. 这会编所有 C# 项目，**以及** `TaskbarFx.Native`（vcxproj）
4. `Suite.App` 编完会把 `TaskbarFx.exe` + `TaskbarFx.Native.dll` **拷到 Suite.exe 旁边**

产出大概在：

```
src\Suite.App\bin\x64\Debug\net10.0-windows10.0.19041.0\Suite.exe
src\Suite.App\bin\x64\Debug\net10.0-windows10.0.19041.0\TaskbarFx.exe
src\Suite.App\bin\x64\Debug\net10.0-windows10.0.19041.0\TaskbarFx.Native.dll
```

若 SDK 没按 Platform 分子目录，就在 `bin\Debug\net10.0-windows10.0.19041.0\`。以实际输出为准。

**三份文件必须在同一文件夹**，Suite 才找得到助手。

### 2.2 也可以命令行（仍是 Windows）

```bat
dotnet build src\Suite.sln -c Debug -p:Platform=x64
dotnet test  src\Suite.sln -c Debug -p:Platform=x64
```

- 只编 C# 时：`dotnet build src\TaskbarFx.Host\TaskbarFx.Host.csproj -c Debug -p:Platform=x64` 可以出 `TaskbarFx.exe`。
- **C++ DLL 必须用 VS / MSBuild 编 vcxproj。** `dotnet` 不会变出 `TaskbarFx.Native.dll`。没这 DLL 时，设置页会说 Native 找不到，**这不是假成功**。
- 若 `dotnet build` 整份 sln 时抱怨不会编 `.vcxproj`：正常。用 VS 生成 Native，或：

```bat
msbuild src\TaskbarFx.Native\TaskbarFx.Native.vcxproj /p:Configuration=Debug /p:Platform=x64
```

然后再编一次 Host / App，让拷贝目标把 DLL 带上。

**Debian 远程没跑上面这些命令，不能当已绿。**

## 3. 管道名（不是 `Suite-TaskbarFx` 这么好猜）

助手是管道 **服务端**，Suite 是 **客户端**。

- 名字：`Suite.TaskbarFx.{当前用户SID}.s{会话号}`  
  例：`Suite.TaskbarFx.S-1-5-21-….s1`（不含 `\\.\pipe\` 前缀，.NET 自己加）
- 同一用户才能连（`CurrentUserOnly`）
- 权限里 **拒绝** Everyone / World / Anonymous
- `FirstPipeInstance`：同名管道不能被别人先占
- 连上后还要看对端 EXE：必须是 **同一目录** 的 `Suite.exe` ↔ `TaskbarFx.exe`。Release **没有**「给调试器放开 Everyone」的开关
- 每条命令最多等 **2 秒**。截图热键 **不会** 卡在这条管道上

看助手是否起来：任务管理器找 `TaskbarFx.exe`。日志：`%LOCALAPPDATA%\Suite\taskbarfx.log`（没有图片、没有密钥）。

## 4. 怎么试（Windows 真机）

先编过，再运行 **Suite.exe 那个目录里的** `Suite.exe`（不要只拷一个 exe）。

| # | 动作 | 预期 |
| --- | --- | --- |
| 1 | 托盘还在，截图热键还能框选 | P1a 没被弄坏 |
| 2 | 设置 → 勾上「启用任务栏效果」，档位亚克力，应用 | **Win10 经典任务栏**（或 Win11 + ExplorerPatcher 一类「看起来像 Win10」的栏）：任务栏变磨砂。任务管理器里多一个 `TaskbarFx.exe`，**没有**提权 |
| 3 | 换成实色 / 全透明 / 系统默认，每次应用 | 能看出差别。系统默认 = 恢复，**explorer 还活着** |
| 4 | 关掉开关，应用 | 任务栏回到系统样；`TaskbarFx.exe` 应退出 |
| 5 | 再打开效果，然后任务管理器结束 `TaskbarFx.exe` | 截图热键、贴图、网速还在。设置可再应用把助手拉起来 |
| 6 | 打开效果后退出 Suite | 默认 **任务栏外观还在**（设置项 KeepAppearance；助手可以继续活着） |
| 7 | **Win11 默认现代任务栏** | 设置状态变成 **「Win11 路径未交付」**。不要指望变透明。这不是 bug |
| 8 | 没编 Native DLL 就开效果 | 文案说找不到 `TaskbarFx.Native.dll`，不会假装成功 |
| 9 | 以管理员运行 Suite | 仍会提示用标准用户。不要用管理员去开任务栏效果 |

双屏：副屏任务栏类名是 `Shell_SecondaryTrayWnd`，能找到就会一起改。某一屏失败不应谎称全部成功。

## 5. 一键恢复默认（不要杀 explorer）

按这个顺序，**禁止**把「结束 explorer.exe」当成日常恢复手段：

1. 打开 Suite 设置 → **去掉**「启用任务栏效果」→ 应用。这会发 Disable，再可选关掉助手。
2. 若 Suite 已经挂了：任务管理器结束 `TaskbarFx.exe`，再在同一目录运行：

```bat
TaskbarFx.exe --reset
```

这只是再刷一次「系统默认」合成，**不会**去杀资源管理器。

3. 还不行：注销或重启一次。仍不要把结束 explorer 写成默认步骤。

未文档 API 可能被 Windows 更新弄坏：效果消失或没变化时，设置页应显示失败/不可用，而不是「已经开了」。

## 6. 杀软可能弹窗（预期，不是你中毒了）

本轮 **没有** 往 explorer 加载 DLL（那是 P2）。但独立进程 + 改任务栏窗口属性，个别杀软仍可能多嘴。

- 不要关 Defender 来「修好」
- 不要给安装程序加排除项
- 对外试玩包离开这台机器前，至少签 EXE（SmartScreen 仍可能告警，见安全审计）
- 误报走 Microsoft 安全情报提交，而不是教用户关实时保护

`TaskbarFx.Native.dll` 里用的 `SetWindowCompositionAttribute` **没有官方文档**，以后系统一更新就可能失效。

## 7. 门禁对照（AUDIT §4.2）

| ID | 落地 |
| --- | --- |
| P1-1 | `CurrentUserOnly` + Deny World/Anonymous；对端校验同目录 EXE；`#if DEBUG` **没有**放宽到 Everyone |
| P1-2 | `FirstPipeInstance`；管道名带用户 SID + 会话号 |
| P1-3 | Native **只**链进 TaskbarFx.Host。Suite.App / Capture **零** SWCA / TAP 符号 |
| P1-4 | 探测到 XAML 桥接窗 → 文案「Win11 路径未交付」。全仓库 **没有** `InitializeXamlDiagnosticsEx` |
| P1-5 | SWCA 失败 / 导出不存在 → `unavailable`，不假成功 |
| P1-6 | 截图仍尊重显示亲和（P1a 已做，本轮未改捕获） |
| P1-7 | 对外离开本机再签；开发自签/未签可在本机跑 |
| P1-8 | 截图临时文件约定仍是 P1a（本轮不改） |

## 8. 许可证

Native 是自写的。**不要**从 TranslucentTB 拷 `undoc/`、`ExplorerTAP/`、`ExplorerHooks/` 或任何 GPL 头。审 diff 时按许可证审查看 C++。

## 9. 出了问题看哪

| 现象 | 先看 |
| --- | --- |
| 勾了没变化，状态「Win11 路径未交付」 | 你是现代 XAML 任务栏，本轮本来就不做 |
| 状态说找不到 Native | VS 没编 vcxproj，或 DLL 不在 Suite.exe 旁边 |
| 状态说连不上 TaskbarFx | 助手没起来；看 `taskbarfx.log`；管道被占用（FirstPipeInstance 失败） |
| 任务栏花了 / 一直半透明 | 用第 5 节恢复；不要结束 explorer |
| 截图热键失灵 | 那是 Suite，不是 TaskbarFx；本轮不应把热键和管道绑在一起 |

卸载（仍无安装包，手工）：

1. 设置里关掉任务栏效果并应用，或 `TaskbarFx.exe --reset` 后结束该进程
2. 退出 Suite
3. `reg delete HKCU\Software\Microsoft\Windows\CurrentVersion\Run /v Suite /f`
4. 可选删除 `%LOCALAPPDATA%\Suite`
