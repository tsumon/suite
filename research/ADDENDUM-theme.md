# 补丁：系统深浅色一键切换（Joe 2026-09-05 补充）

- 补丁作者：监控子 agent（非原调研 grok）
- 补丁日期：2026-09-05（Asia/Shanghai）
- 背景：原 `REPORT.md` **未覆盖**「一键切换 Windows 系统 Light/Dark」（仅出现 TranslucentTB 未文档头里的 `uxtheme.hpp` 文件名，与系统主题切换无关）。
- 原则：有出处才写；精确 star 等用 `gh api` 核对；实现伪代码仅作说明，不是可复制产品代码。

---

## 1. 需求界定

| 要 | 不要 |
| --- | --- |
| 切换 **系统级** Apps + System 配色（Settings → Personalization → Colors 里那两档） | 只改本 App 自己的 WPF/WinUI 主题 |
| 托盘/热键一键翻转，尽量即时生效 | 依赖重启 Explorer 才能看见变化（可作为最后手段） |
| 与任务栏透明/亚克力、桌面网速悬浮窗共存时不互踩 | 做成完整 `.theme` 包替换（壁纸+声音+鼠标）——那是另一档能力 |

---

## 2. 可用手段（按推荐顺序）

### 2.1 注册表 DWORD（社区主流、需广播）

路径（当前用户）：

`HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize`

| 值名 | 含义 | 取值 |
| --- | --- | --- |
| `AppsUseLightTheme` | 应用默认浅/深 | `1` = Light，`0` = Dark |
| `SystemUsesLightTheme` | 系统外壳（任务栏、开始、操作中心等）浅/深 | 同上 |

出处（社区/教程，**非** Microsoft 正式「Theme Toggle API」文档）：

- Windows Central：用 Task Scheduler + PowerShell 写上述两 DWORD（[How to switch between Windows 10 light and dark modes on schedule](https://www.windowscentral.com/how-switch-between-light-and-dark-colors-schedule-automatically-windows-10)）
- Stack Overflow：Win11 仅写注册表时 Explorer/任务栏钟点颜色等 **不会完整刷新**（[Toggle Light/Dark Mode Programmatically in Windows 11](https://stackoverflow.com/questions/75107056/toogle-light-dark-mode-programmatically-in-windows-11)）
- Mike van Oorschot：写完后必须 `SendMessageTimeout(HWND_BROADCAST, WM_SETTINGCHANGE, …, "ImmersiveColorSet")`，否则任务栏/托盘字体等不可靠（[Easily toggle between Windows light/dark mode](https://mikevanoo.co.uk/blog/toggle-windows-dark-mode/)）

**即时生效要点：**

1. 同时写 `AppsUseLightTheme` 与 `SystemUsesLightTheme`（若只要「系统壳」或只要「应用」，可分写；合集「一键」通常两档一起翻）。
2. 广播 `WM_SETTINGCHANGE`（数值 `0x001A`），`lParam = "ImmersiveColorSet"`。只改注册表、不广播 → Win11 上常见「半切换」（资源管理器仍浅、任务栏文字对比度错乱）。
3. 杀 `explorer.exe` 强制刷新是粗暴回退，会闪任务栏、打断正在进行的拖放；合集 App **不应作为默认路径**（SO / 社区脚本有人这么做，体验差）。

**权限：** `HKCU` 即可，**不需要管理员**。企业策略若锁 Personalization，写入可能失败或被策略覆盖——需人工在目标机再核。

### 2.2 `IThemeManager2`（未文档 COM，完整 .theme 场景）

- COM 接口在 `themeui.dll` 一侧，社区逆向整理见 namazso gist / SecureUxTheme `ThemeLib`（[gist: Setting themes with IThemeManager2](https://gist.github.com/namazso/0fde102c2fc56049c7c37f7fdf9ac3cd)；[SecureUxTheme ThemeLib/theme.cpp](https://github.com/namazso/SecureUxTheme/blob/master/ThemeLib/theme.cpp)）。
- `SetCurrentTheme(...)` 可应用整套 `.theme`（含 `AppMode`/`SystemMode`），比只翻两个 DWORD 更「像设置面板」。
- **风险：** 未文档；Auto Dark Mode 维护者讨论过 Win11 上仅改 Apps/System 时 UI 刷新不稳定（约「50–80%」为讨论中的经验估计，**需人工再核**，不当成硬指标）：[AutoDarkMode issue #567](https://github.com/AutoDarkMode/Windows-Auto-Night-Mode/issues/567)。
- MVP **不必**上这条；若以后要「一键切完整主题包（壁纸+强调色）再考虑。

### 2.3 对照产品：Auto Dark Mode

| 项 | 值 | 出处 |
| --- | --- | --- |
| 仓 | https://github.com/AutoDarkMode/Windows-Auto-Night-Mode | `gh api` 2026-09-05 |
| 描述 | Automatically switches between the dark and light theme of Windows 10 and Windows 11 | 同上 |
| license | **GPL-3.0** | 同上 |
| stars | **9677** | 同上 |
| last push | 2026-09-04T08:53:48Z | 同上 |

闭源合集：**可学行为，不可拷代码**（与 REPORT 对 TranslucentTB/ShareX 同一红线）。

### 2.4 WinRT / 官方文档缺口

本轮 **找不到** 面向第三方的、文档化的「ToggleSystemColorMode()」一类公开 WinRT API（Settings 自己走内部路径）。公开侧更多是 **读** 用户偏好（如应用跟随系统主题），而不是 **写** 系统偏好。因此合集实现应预期走 **HKCU + ImmersiveColorSet 广播**（事实标准），并在发布说明写明「依赖未文档广播约定，Windows 大版本需回归」。

---

## 3. 与任务栏透明 / 网速窗的联动注意

| 点 | 说明 |
| --- | --- |
| 任务栏亚克力（TranslucentTB 类） | 深浅切换会改任务栏底色与文字对比度；若以后做注入/刷 brush，必须订阅 `WM_SETTINGCHANGE`/`ImmersiveColorSet` 或注册表监听后 **重算 brush/透明度**，否则会出现「深色字压在深色亚克力上」。REPORT 已建议 v1 不做亚克力——若 MVP 仍只做悬浮网速，联动面小很多。 |
| 桌面网速悬浮窗 | 跟随 `AppsUseLightTheme` 或自备浅/深两套前景色；切换后刷新一次绘制即可。 |
| 截图遮罩 / 贴图窗 | 与系统主题弱相关；贴图窗自管背景即可。 |
| 同时跑 Auto Dark Mode | 两程序抢写同一注册表键会「打架」；设置页应提示互斥或提供「外部托管主题」开关。 |

---

## 4. MVP 边界建议（合并进拍板）

**建议：第一版放入「一键深浅切换」，但做成最小切片。**

| 放入 v1 | 理由 |
| --- | --- |
| 托盘菜单 / 热键：同时翻转 Apps+System 两 DWORD + `ImmersiveColorSet` 广播 | 无管理员、无注入、与「合集小工具」定位匹配；实现量远小于截图选区 |
| 切换后刷新本 App 悬浮窗配色 | 用户立刻感到「有效」 |

| 仍放 out-of-scope | 理由 |
| --- | --- |
| 按日出日落自动切换、壁纸随主题换 | Auto Dark Mode 主场；GPL 且范围膨胀 |
| `IThemeManager2` 整包 `.theme` | 未文档、调试贵 |
| 与 TranslucentTB 级任务栏效果的联合主题配置 | 亚克力本身就不在 v1 |

**相对原 REPORT MVP（截图 + 贴图 + 桌面网速）：** 深浅切换是第 4 个小能力，技术风险 **低于** 任务栏亚克力与 HDR 截图，可并行由 C# P/Invoke 完成。

---

## 5. 实现草图（说明用，非交付代码）

```text
1. 读 HKCU\...\Personalize\SystemUsesLightTheme（缺省可当 1）
2. new = 1 - old
3. 写 AppsUseLightTheme = new，SystemUsesLightTheme = new（REG_DWORD）
4. SendMessageTimeoutW(HWND_BROADCAST=0xFFFF, WM_SETTINGCHANGE=0x1A,
      wParam=0, lParam=L"ImmersiveColorSet", SMTO_ABORTIFHUNG, timeout≈5000ms)
5. 本进程重读主题，刷新悬浮窗/托盘图标
```

语言：与 REPORT 推荐栈一致时用 **C# + P/Invoke**（`Microsoft.Win32.Registry` + `user32.SendMessageTimeout`）。

---

## 6. 出处缺口 / 需人工再核

1. Microsoft Learn **是否存在**正式「程序化切换系统浅深色」页面——本轮检索未找到对等官方 API 文；若有新 Win11 SDK API，需再搜一次。
2. SO 所述 Win11「只写注册表不全刷新」的复现范围（哪几个 build）——需在目标 Win11 机实测。
3. Auto Dark Mode issue #567 中「50–80%」刷新失败率为讨论经验值，**非基准测试**。
4. 企业 MDM / 教育版锁主题后的行为未测。
5. 与现网 TranslucentTB / ExplorerPatcher 同机切换时的视觉回归未测。

## 补丁来源链接（本文件用到的）

- https://mikevanoo.co.uk/blog/toggle-windows-dark-mode/
- https://stackoverflow.com/questions/75107056/toogle-light-dark-mode-programmatically-in-windows-11
- https://stackoverflow.com/questions/77179391/instantly-switch-between-dark-and-light-mode-for-windows
- https://www.windowscentral.com/how-switch-between-light-and-dark-colors-schedule-automatically-windows-10
- https://dev.to/vast-cow/toggle-windows-light-and-dark-mode-with-python-277g
- https://gist.github.com/namazso/0fde102c2fc56049c7c37f7fdf9ac3cd
- https://github.com/namazso/SecureUxTheme
- https://github.com/AutoDarkMode/Windows-Auto-Night-Mode
- https://github.com/AutoDarkMode/Windows-Auto-Night-Mode/issues/567
- https://github.com/atefalshehri/WinThemeSwitcher（二次来源，描述 IThemeManager2 用法；未做源码审计）
