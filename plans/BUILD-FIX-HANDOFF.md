# Suite 编译修复交接（TreatWarningsAsErrors）

- **日期：** 2026-09-05
- **远程：** Debian **没有** .NET 10 / Windows Desktop，也 **没有** `dotnet`。本机 **不能** 当绿。请在 Windows 上编。
- **没 push。** 没改 DESIGN / CAPTURE-PIN-SPEC 行为。没加 `System.IO.AccessControl` 包。
- **目标：** 清掉 tsumon 上 `TreatWarningsAsErrors` 拦下的 C# 错误，让

  `dotnet build src\Suite.App\Suite.App.csproj -c Debug -p:Platform=x64`

  在概念上能过。`TaskbarFx.Native.vcxproj` 仍要 VS C++；缺 DLL 时 C# 走已有 `TryLoad` 降级。

## 0. 改了什么（审 diff 时先看）

| 项 | 处理 |
| --- | --- |
| 缺 `using System.IO;` | Capture / Platform / App / Host / Contracts 里用到 `Directory`/`File`/`Path`/`FileStream` 的文件补了显式 using。 |
| `AnnotationWindow.TextInput` 隐藏 `UIElement.TextInput`（CS0108） | XAML + code-behind 控件改名为 `TextEntry`。标注「文字」工具、Enter 提交、Esc 取消不变。 |
| `DxgiDuplicationGrabber` CS8500 | `DxgiOutputDesc` 不再用托管 `string`。改成 `fixed char[32]` + `int` BOOL，内存布局对齐 DXGI_OUTPUT_DESC。`AllowUnsafeBlocks` 本来就是 true，光开不够。 |
| `GraphicsCaptureGrabber` 运算符 `?` 用于方法组 | 去掉 `??=` 和 `frame?.Dispose()`。帧用 `if (captured is null)` 取；释放走 `IDisposable` 再 `Dispose()`。`IsSupported` 仍按 CsWinRT **属性** 读。 |
| `CaptureService` 未使用 `ex` | 剪贴板 `catch (Exception)` 不再声明 `ex`。错误文案仍是「无法写入剪贴板。」 |
| Platform Pipe ACL | `TightenAcl` 继续 stub。删掉未调用的 `CreateServerSecurity`（`PipeSecurity` / `PipeAccessRule`）。**没有** 加 AccessControl 包（NU1510）。管道仍 `CurrentUserOnly` + `FirstPipeInstance`。 |
| X509 `CreateFromSignedFile` | pragma SYSLIB0057 **保持**。 |

## 1. 工具链（Windows）

和 P0/P1 一样，在 **VS 2026 / MSBuild 18.0+** 的开发者命令行，或已装 **.NET 10 SDK** 的 PowerShell：

```bat
dotnet --info
```

须看到 **.NET 10 SDK**，runtime 含 **Microsoft.WindowsDesktop.App**（WPF / WinForms）。架构 **x64**。

Native DLL 另要 **使用 C++ 的桌面开发**。没有 DLL 时 `TaskbarFxNativeBridge.TryLoad` 失败，任务栏效果不可用，截图/贴图/网速仍应能开。

## 2. 编 / 测（只在 Windows 跑）

仓库根：

```bat
dotnet build src\Suite.App\Suite.App.csproj -c Debug -p:Platform=x64
dotnet build src\Suite.sln -c Debug -p:Platform=x64
dotnet test  src\Suite.sln -c Debug -p:Platform=x64
```

预期：

- C# 项目不再报：缺 `System.IO`、CS0108 `TextInput`、CS8500 托管类型取地址、`?.`/`??=` 方法组、未使用 `ex`、NU1510 AccessControl。
- 产出 `src\Suite.App\bin\...\Suite.exe`（路径以 SDK 是否按 Platform 分子目录为准）。
- `Suite.sln` 里 `TaskbarFx.Native` 若没装 C++ 工作负载会红。这 **不是** 本轮 C# 修复范围。编 `Suite.App.csproj` 时 Copy 目标对缺 DLL 是 skip。
- 测试至少：`Suite.Contracts.Tests`、`Suite.ThemeToggle.Tests`、`Suite.NetSpeed.Tests`、`Suite.Capture.Tests`（选区数学、马赛克、跨屏裁切；**不**真截屏）。

**Debian 远程未执行上述命令，不能当已绿。**

## 3. 手工点检（行为不能坏）

截图/标注/贴图仍按 `design/CAPTURE-PIN-SPEC.md`：

| # | 动作 | 预期 |
| --- | --- | --- |
| 1 | `Ctrl+Shift+F12` → 拖框松手 | 默认钉图 + 剪贴板（`PinAfterCapture`） |
| 2 | Shift+松手 → 点「文字」→ 输入 → Enter | 字画在图上。Esc 关标注框不提交这段字 |
| 3 | 标注里画矩形/箭头/马赛克再复制 | 粘贴看得到 |
| 4 | Esc / 右键取消选区 | 不钉、不写剪贴板 |
| 5 | 无 `TaskbarFx.Native.dll` 启动 | 不崩；任务栏效果失败可见；截图仍可用 |

## 4. 若 Windows 上还红

| 现象 | 怎么办 |
| --- | --- |
| `GraphicsCaptureSession.IsSupported` 报方法组 / 不能当方法用 | 19041 CsWinRT 是**属性**（现在这样）。若本机 SDK 投影成方法，改成 `IsSupported()`。不能写成同时兼容属性和方法的 C#。 |
| `PipeSecurity` 找不到 | 不要加 AccessControl 包。`CreateServerSecurity` 已删；`CreateServer` 只用 `CurrentUserOnly`。 |
| SYSLIB0057 `CreateFromSignedFile` | 已有 pragma，不要删。 |
| `TaskbarFx.Native.vcxproj` 要 CL.exe | 装 C++ 桌面工作负载，或只编 `Suite.App.csproj` 先跑 C#。 |

## 5. 改了哪些文件

| 路径 | 干什么 |
| --- | --- |
| `src/Suite.Capture/CaptureService.cs` | `using System.IO;`；剪贴板 catch 去掉未用 `ex` |
| `src/Suite.Capture/PngFileSaver.cs` | `using System.IO;` |
| `src/Suite.Capture/AnnotationWindow.xaml` | `TextInput` → `TextEntry` |
| `src/Suite.Capture/AnnotationWindow.xaml.cs` | 同上 |
| `src/Suite.Capture/Native/CaptureNative.cs` | `DxgiOutputDesc` 改非托管布局 |
| `src/Suite.Capture/Grabbers/GraphicsCaptureGrabber.cs` | 去掉 `??=` / `?.Dispose()` |
| `src/Suite.Platform/TaskbarFxPipe.cs` | 显式 `System.IO`；删 `CreateServerSecurity`；保留 TightenAcl stub + X509 pragma |
| `src/Suite.Platform/RunKeyService.cs` | `using System.IO;` |
| `src/Suite.Contracts/SettingsPaths.cs` | `using System.IO;` |
| `src/Suite.App/SettingsStore.cs` | `using System.IO;` |
| `src/Suite.App/TaskbarFxNativeBridge.cs` | `using System.IO;` |
| `src/TaskbarFx.Host/HostLog.cs` | `using System.IO;` |
| `src/TaskbarFx.Host/NativeBridge.cs` | `using System.IO;` |
| `src/TaskbarFx.Host/PipeServer.cs` | `using System.IO;`（`IOException`） |
