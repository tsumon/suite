<p align="center">
  <img src="./assets/readme/hero.png" alt="Suite — Windows 托盘工具：F1 截图选区框与任务栏网速芯片" width="1200" />
</p>

<p align="center">
  <strong>一个纯原生单进程托盘工具：F1 截图标注 / OCR / 长截图、贴图、任务栏网速、任务栏透明与系统深浅色切换</strong>
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0-blue.svg" alt="License: GPL-3.0" /></a>
  <img src="https://img.shields.io/badge/platform-Windows%2010%2F11%20x64-2077F0.svg" alt="Platform: Windows 10/11 x64" />
  <img src="https://img.shields.io/badge/language-Rust%20%2B%20Win32%2FD2D-EA580C.svg" alt="Language: Rust + Win32/D2D" />
</p>

**Suite v0.3.0** 是纯原生 **Rust（Win32 + Direct2D / DirectWrite + DXGI + WinRT OCR）** 单进程托盘宿主（仅保留 C++/WinRT XAML Diagnostics 注入模块 `TaskbarFx.Tap.dll`）。运行时内存低，启动极快，安装包仅约 2.6 MB，并完整兼容旧版 `settings.json` 与历史截图目录。

---

## 核心功能

| 能力 | 说明 |
| --- | --- |
| **F1 区域截图与标注** | DXGI / GDI 多屏抓取、窗口吸附与放大镜；内置矩形/椭圆、直线/虚线/箭头、铅笔、马克笔、**方块马赛克 & 柔和模糊**、**步骤序号圆标 ①②③**、原生中文 IME 文字输入、橡皮擦、撤销/重做 |
| **交互与快捷键增强 (v0.3.0)** | **工具栏 & 子栏悬停提示气泡（带快捷键说明）**、HiDPI 自适应缩放、滚轮调节画笔粗细/半径/字号、`Shift` 正形与 45° 吸附、方向键 1px / `Shift+方向键` 10px 选区微调、`Ctrl+Shift+S` 一键快存、记忆上次颜色与尺寸、`1–7 / I / E` 切工具、`C` 选区取色 |
| **延时 3 秒截图 (v0.3.0)** | 托盘右键菜单与设置首页一键触发「延时 3 秒截图」，轻松截取悬停菜单、Tooltip 与瞬时弹窗 |
| **WinRT 离线识字 (OCR)** | 标注工具栏一键识别 (`Ctrl+O`)，也支持在桌面**贴图**上按 `O` 或右键直接 OCR 复制文字 |
| **滚动长截图 (`F4`)** | 选区实时显示已拼接像素高度，停滚 1.5 秒自动完成拼接并复制/保存 |
| **屏幕取色 (`F2`)** | 放大镜像素取色，`Space`/`Shift` 切换 `HEX` 与 `rgb(...)` 格式复制 |
| **置顶彩虹贴图 (`F3` / `Ctrl+Shift+Q`)** | Direct2D 彩虹悬停描边、滚轮缩放、透明调节、鼠标穿透；**v0.3.0 新增双击缩略图折叠/还原、顺时针旋转 90°、水平翻转、`R` 重置 1:1、贴图直出 OCR、`Ctrl+C/S` 及托盘「隐藏/显示全部贴图」** |
| **任务栏嵌入网速** | Direct2D `UpdateLayeredWindow` **真嵌入主/副任务栏**（或独立悬浮窗），随文本行宽自动适配槽位，支持切换「自动选择」或指定物理网卡 |
| **任务栏透明外观** | Win10 `SWCA` 与 Win11 `TaskbarFx.Tap.dll`（清晰 / 亚克力 / 不透明 / 原生），带 `CrashGuard` 自动保护 |
| **一键切换深浅色** | 同步切换 Windows 应用与系统深浅色，设置、历史、贴图管理窗口全套适配 Win11 沉浸式深色标题栏与自绘导航栏 |

---

## 安装与升级

1. 打开 [Releases](https://github.com/tsumon/suite/releases)，下载 `Suite-Setup-0.3.0.exe`（安装版）或 `Suite-portable-0.3.0.zip`（便携版）。
2. 若本机已安装 0.1.x / 0.2.x 版本，直接运行 `Suite-Setup-0.3.0.exe` 即可原位升级，安装器会自动关闭旧版并清理遗留 `.NET` 运行库（如有），同时保留 `%LOCALAPPDATA%\Suite\settings.json` 与历史截图。
3. **便携版**需保持 `Suite.exe` 与 `TaskbarFx.Tap.dll` 处于同一目录。布局约定见 [`installer/PORTABLE.md`](installer/PORTABLE.md)。

---

## 杀软与 `TaskbarFx.Tap.dll`

Win11 任务栏透明路径需向 `explorer.exe` 注入 `TaskbarFx.Tap.dll`（改编自 TranslucentTB）。部分杀毒软件或 EDR 可能拦截该 DLL；即便被拦截，截图、识字、贴图、网速与深浅色切换等核心功能仍会照常工作，且 `CrashGuard` 会在连续异常退出后自动关闭任务栏透明以保护主进程稳定性。

---

## 从源码构建

需 Windows 10/11 (x64)、Rust 工具链（MSVC x64）、VS 2022 C++ Build Tools，以及可选的 [Inno Setup 6](https://jrsoftware.org/isinfo.php)。详见 [`rust/README.md`](rust/README.md)。

```powershell
# 运行静态检查与单元测试
cmd /c "cd /d rust && cargo clippy --workspace --all-targets --offline && cargo test --workspace --offline"

# 一键构建 C++ TAP + Rust Release + 便携包 + 安装包（输出到 dist-rs\）
powershell -ExecutionPolicy Bypass -File scripts\build-rust.ps1
```

---

## 许可

**GPL-3.0**。见仓库根目录 [`LICENSE`](LICENSE) 与 [`NOTICE`](NOTICE)（包含 TranslucentTB 等开源归属声明）。
