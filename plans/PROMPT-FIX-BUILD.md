# 修到 Windows 能编过 Suite.App

机器 tsumon 上 `dotnet build` 报错（TreatWarningsAsErrors）。修好 `/workspace/windows-suite-app` 源码。

## 已知错误（Suite.Capture）

1. 缺 `using System.IO;` → Directory/File/Path/FileStream/FileMode…
2. `AnnotationWindow.TextInput` 隐藏 `UIElement.TextInput` → 控件改名
3. `DxgiDuplicationGrabber` CS8500 托管类型取地址 → AllowUnsafeBlocks 或改结构
4. `GraphicsCaptureGrabber.cs` 运算符 `?` 用于方法组
5. `CaptureService` 未使用变量 `ex`
6. Platform 的 PipeStreamAcl：已 stub TightenAcl，勿再加 AccessControl 包（NU1510）
7. X509 CreateFromSignedFile：已 pragma，保持

## 约束

- 不破坏 DESIGN / CAPTURE-PIN-SPEC 行为
- 单进程；不 push
- 尽量让 `dotnet build src/Suite.App/Suite.App.csproj -c Debug -p:Platform=x64` 在概念上能过（远程无 Windows Desktop 可只改到无这类错误）
- TaskbarFx.Native.vcxproj 可仍需 VS C++，C# 侧缺 DLL 时应优雅降级（已有 TryLoad）

写 `plans/BUILD-FIX-HANDOFF.md`，DONE。
