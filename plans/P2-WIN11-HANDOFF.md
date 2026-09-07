# Suite P2 交接（Win11 任务栏透明：TAP + TranslucentTB）

- **日期：** 2026-09-05
- **远程：** Debian 只出源码。**没有**在这台 Linux 上编过 WPF / C++ / C++/WinRT。不要把远程当绿。
- **目标：** tsumon（Win11）上设置启用任务栏效果后，Clear / Opaque 肉眼可见；Acrylic 尽力。成功时 path=`win11-xaml`，不再永远「Win11 路径未交付」。
- **许可证：** 整包 GPL-3.0。Win11 TAP 改编自 TranslucentTB ExplorerTAP。见根目录 `LICENSE`、`NOTICE`、`architecture/DECISION-GPL-TRANSLUCENTTB.md`。

## 0. 你现在多了什么

| 东西 | 干什么 |
| --- | --- |
| `vendor/TranslucentTB/` | 上游 shallow snapshot（release / `d4636e439865df0a1a1419db408e055740ce5c74`）。**不编这个树。** 只作版权与对照。 |
| `src/TaskbarFx.Tap/` → `TaskbarFx.Tap.dll` | 进 **explorer.exe** 的 TAP。改 `Taskbar.TaskbarFrame` 下 `BackgroundFill` / `BackgroundStroke`。零 CLR。 |
| `TaskbarFx.Native.dll` | 经典栏仍 SWCA。现代 XAML 栏用 **绝对路径** 调 `InitializeXamlDiagnosticsEx` 把 TAP 载入 explorer。 |
| Suite 设置页 | 成功显示「任务栏效果已应用。」失败人话 +「截图和网速仍可用。」 |

**没有做（故意的）：** TranslucentTB 的 XAML 设置 UI、ExplorerHooks / Detours、Blur 档位、动态模式、TrafficMonitor 源码、默认结束 explorer。

## 1. 工具链（Windows 真机）

和 P1b 一样：

1. VS 2026 / MSBuild 18 + **.NET 10 SDK**（`Microsoft.WindowsDesktop.App`）
2. **使用 C++ 的桌面开发**
3. **Windows 10/11 SDK**（要带 C++/WinRT：`cppwinrt.exe` 和 `Include\...\cppwinrt`）
4. 平台工具集 **v143**。若 VS 弹出升级到 v145，可以升。
5. 架构 **x64**。不要 AnyCPU。

TAP 工程会在编前尝试：

```
%WindowsSdkDir%\bin\%WindowsTargetPlatformVersion%\x64\cppwinrt.exe -input sdk -output "Generated Files"
```

若这步失败：装完整 Windows SDK，或把 SDK 的 `cppwinrt` 头目录加进 `TaskbarFx.Tap.vcxproj` 的 AdditionalIncludeDirectories。

## 2. 怎么编

仓库根（本机路径以你放的为准；远程是 `/workspace/windows-suite-app/`）。

### 2.1 推荐：VS 解决方案

打开 `src\Suite.sln`。

1. 配置 **Debug**（或 Release）+ **x64**
2. 生成 → 生成解决方案
3. 这会编 C#、`TaskbarFx.Native`、`TaskbarFx.Tap`
4. `Suite.App` 编完把两个 DLL 拷到 `Suite.exe` 旁边

产出（路径以 SDK 是否按 Platform 分子目录为准）：

```
src\Suite.App\bin\x64\Debug\net10.0-windows10.0.19041.0\Suite.exe
src\Suite.App\bin\x64\Debug\net10.0-windows10.0.19041.0\TaskbarFx.Native.dll
src\Suite.App\bin\x64\Debug\net10.0-windows10.0.19041.0\TaskbarFx.Tap.dll
```

**三份必须在同一文件夹。** 缺 TAP DLL 时设置页会失败可见，不会假成功。

### 2.2 命令行（仍是 Windows）

```bat
msbuild src\TaskbarFx.Tap\TaskbarFx.Tap.vcxproj /p:Configuration=Debug /p:Platform=x64
msbuild src\TaskbarFx.Native\TaskbarFx.Native.vcxproj /p:Configuration=Debug /p:Platform=x64
dotnet build src\Suite.sln -c Debug -p:Platform=x64
dotnet test  src\Suite.sln -c Debug -p:Platform=x64
```

`dotnet` **不会**编 vcxproj。没装 C++ 工作负载时 sln 里 Native/TAP 会红，这不是 C# 的锅。

**Debian 远程没跑上面这些命令，不能当已绿。**

## 3. 运行时路径

| 探测 | 路径 | 行为 |
| --- | --- | --- |
| `Shell_TrayWnd` 下有 `Windows.UI.Composition.DesktopWindowContentBridge` | `win11-xaml` | TAP 改 XAML brush |
| 经典 Win32 栏（Win10，或 Win11 + ExplorerPatcher 一类） | `win10-swca` | 未文档 `SetWindowCompositionAttribute` |
| 找不到任务栏 | `unavailable` | 人话失败 |

Win11 档位：

| 档位 | TAP |
| --- | --- |
| 系统默认 | 恢复原来的 Fill |
| 实色 | `SolidColorBrush`，A=255 |
| 全透明 | `SolidColorBrush`，A=0；藏底边 |
| 亚克力 | `AcrylicBrush` Backdrop。失败则实色 tint，状态仍算应用，LastMessage 会写 fallback |

注入：`InitializeXamlDiagnosticsEx`，`wszTAPDllName` = **Suite 目录下 `TaskbarFx.Tap.dll` 的绝对路径**（AUDIT F-04）。最多换几个 connection 名，总等待约数秒，不死循环。失败则再试一次 `SetWindowsHookEx` 把 DLL 载进任务栏线程（仍不杀 explorer）。**ready 只在 `AdviseVisualTreeChange` 返回后置位**（与 TranslucentTB 相同，不在 `StartPump` 里抢先 `SetEvent`）；Native `Apply` 若收到 `PendingFills` 会最多再等约 8 秒、每 200ms 重试，直到找到 `BackgroundFill`。

单进程：TAP 崩可能拖死 Suite（含截图）。Joe 已接受。见 `architecture/DECISION-SINGLE-PROCESS.md`。

## 4. 怎么试（tsumon Win11）

先编过，再运行 **Suite.exe 那个目录里的** `Suite.exe`。

| # | 动作 | 预期 |
| --- | --- | --- |
| 1 | 托盘、截图热键、网速、一键深浅色 | P1 没被弄坏 |
| 2 | 设置 → 启用任务栏效果 → **全透明** → 应用 | 任务栏看穿壁纸。状态「任务栏效果已应用。」**不是**「Win11 路径未交付」 |
| 3 | 换成 **实色**，选一个显眼颜色 | 任务栏铺满该色 |
| 4 | **亚克力** | 磨砂更好；不行则实色 tint，不要当假成功瞒过去 |
| 5 | 去掉「启用任务栏效果」→ 应用 | 回到系统默认。**explorer 还活着** |
| 6 | 双屏 | 副屏 `Shell_SecondaryTrayWnd` 尽力一起改 |
| 7 | 没编 `TaskbarFx.Tap.dll` 就开效果 | 失败可见，截图仍可用 |

## 5. 一键恢复默认（不要杀 explorer）

按这个顺序，**禁止**把「结束 explorer.exe」当成日常恢复手段：

1. Suite 设置 → **去掉**「启用任务栏效果」→ 应用。
2. 若 Suite 已经挂了，同一目录还可以：

```bat
TaskbarFx.exe --reset
```

这只是再刷一次系统默认（经典 SWCA 或 TAP 还原 Fill），**不会**去杀资源管理器。

3. 还不行：注销或重启一次。

Windows 累计更新可能改 XAML 树节点名。效果消失时设置页应显示失败，而不是「已经开了」。

## 6. 杀软可能弹窗（预期，不是你中毒了）

P2 **会**往 `explorer.exe` 加载 `TaskbarFx.Tap.dll`。这是文档化的 XAML 诊断 API 拿来改任务栏（off-label）。Defender / EDR / Smart App Control 会按注入画像处理（AUDIT F-01）。

- 不要关 Defender 来「修好」
- 不要给安装程序加排除项（排除本身是攻击者常用持久化）
- 对外离开本机前签 **Suite.exe、TaskbarFx.Native.dll、TaskbarFx.Tap.dll**（同一出版者）
- 误报走 [Microsoft 安全情报提交](https://www.microsoft.com/en-us/wdsi/filesubmission)
- 发布说明用用户语言：任务栏效果会向资源管理器加载本产品的辅助模块

TranslucentTB README 也写过 “Some antiviruses are over eager”。

## 7. 许可证审 diff

- 根目录 `LICENSE` = GPL-3.0 全文
- `NOTICE` 点名 TranslucentTB URL + commit
- `src/TaskbarFx.Tap/` 文件头保留上游版权
- **不要**把 TrafficMonitor（Anti-996）源码合进来
- 不要整份搬 TranslucentTB 的 `Xaml/` 设置 UI

## 8. 出了问题看哪

| 现象 | 怎么办 |
| --- | --- |
| 状态仍像失败、任务栏没变 | 确认 `TaskbarFx.Tap.dll` 和 Native 在 Suite.exe 旁；看设置状态全文 |
| `cppwinrt.exe` 找不到 | 装 Windows SDK；或手工把 SDK `cppwinrt` 头加入 Include |
| 平台工具集 v143 没有 | 升 v145 |
| 杀软隔离了 TAP DLL | 不要关实时保护；本机开发可暂时还原文件后再编 |
| Acrylic 看起来像实色 | 已知尽力；Clear/Opaque 才是 P2 硬验收 |
| 和现网 TranslucentTB 同时开 | 两套都会改任务栏，可能抢。先退掉其中一个 |

## 9. 本轮没在 Debian 上跑的命令

```bat
msbuild src\TaskbarFx.Tap\TaskbarFx.Tap.vcxproj /p:Configuration=Debug /p:Platform=x64
msbuild src\TaskbarFx.Native\TaskbarFx.Native.vcxproj /p:Configuration=Debug /p:Platform=x64
dotnet build src\Suite.sln -c Debug -p:Platform=x64
dotnet test  src\Suite.sln -c Debug -p:Platform=x64
```

## 10. TAP 时序核对（2026-09-06 box 源码）

**verified on box source 2026-09-06; awaiting tsumon boot**

对照 `plans/PROMPT-FIX-TAP-FILLS.md`，源码已具备，本轮**无缺口可改**：

| 要求 | 源码位置 | 结论 |
| --- | --- | --- |
| Ready 仅在 `AdviseVisualTreeChange` 之后 `SetEvent` | `TaskbarFx.Tap/src/visualtreewatcher.cpp` 构造里 Advise 返回后 SetEvent；`dllmain.cpp` `StartPump` **不再**抢先 SetEvent ready | ✅ |
| `SetSite` 把 ready 句柄交给 Watcher | `tapsite.cpp` | ✅ |
| Native PendingFills 重试 ~8s / 200ms | `TaskbarFx.Native/src/taskbarfx_win11.cpp` `kPendingWaitMs=8000` `kPendingRetryMs=200` | ✅ |
| 超时中文 | Native `SetLast(L"暂时找不到…")`；`TaskbarFxCopy.PendingFills` 同句 | ✅ |
| BackgroundFill 兼容 Shape/Border；TaskbarFrame 下主动搜 | `visualtreewatcher.cpp` `TryRegisterNamedBackground` + `DiscoverFills` | ✅ |

真机仍须：MSBuild 编 TAP/Native → Suite 旁三 DLL 齐全 → 设置启用全透明，状态「任务栏效果已应用。」，**不要**杀 explorer。
