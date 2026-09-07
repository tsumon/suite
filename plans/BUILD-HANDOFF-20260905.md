# Suite → Grok Build 交接（2026-09-05）

> Joe：Grok Bot 额度紧，后续专业活优先走 **Grok Build**（`grok --agent`，远程机可见终端）。本文件是给 Build 的完整上下文。

## 1. 产品是什么

Windows 合集 App（单进程托盘 `Suite.exe`）：

1. Snipaste 风格截图 / 标注 / 钉图  
2. TranslucentTB 风格任务栏透明（Win11 TAP 优先）  
3. TrafficMonitor 风格任务栏网速（自写，**不**掺 TrafficMonitor Anti-996 源码）  
4. 系统深浅色一键切换（托盘右键「切换系统深浅色」）

许可：整包 **GPL-3.0**（为合法使用/改编 TranslucentTB）。见根目录 `LICENSE`、`NOTICE`、`architecture/DECISION-GPL-TRANSLUCENTTB.md`。

## 2. 路径与机器

| 角色 | 路径 |
| --- | --- |
| Bot/共享机源码权威 | `/workspace/windows-suite-app/`（box） |
| Windows 真机编测 | tsumon：`C:\Users\Administrator\suite-app-fresh\windows-suite-app\` |
| 旧目录（可能过期） | `C:\Users\Administrator\suite-app\windows-suite-app\` |
| 设置 | `%LOCALAPPDATA%\Suite\settings.json` |
| 截图日志 | `%LOCALAPPDATA%\Suite\capture.log` |

- 最低系统：Win10 1903+；真机 tsumon 是 **Win11**。  
- 栈：C# / .NET 10 / WPF；C++ 仅 `TaskbarFx.Native` / `TaskbarFx.Tap`（需 MSBuild/VS，**不要**用 `dotnet build Suite.sln` 编 C++，编 `Suite.App.csproj`）。  
- Joe 自称小白：汇报用大白话；他只提需求与点效果，少让他碰命令行。  
- 「你控制吧」：可在 tsumon 上装工具、编译、杀进程、重启 Suite。

## 3. 编 / 跑（Windows）

```bat
cd C:\Users\Administrator\suite-app-fresh\windows-suite-app
dotnet build src\Suite.App\Suite.App.csproj -c Debug -p:Platform=x64
REM 输出：src\Suite.App\bin\x64\Debug\net10.0-windows10.0.19041.0\Suite.exe
REM TaskbarFx.Tap.dll / TaskbarFx.Native.dll 需在同目录（可从旧成功构建拷）

taskkill /IM Suite.exe /F
start "" "src\Suite.App\bin\x64\Debug\net10.0-windows10.0.19041.0\Suite.exe"
```

设置要点（schema 3）：

- 热键：**裸 F1**（`virtualKey=112`，无 Ctrl/Shift/Alt/Win）  
- `capture.pinAfterCapture=false`（默认松手出工具栏，不自动钉）  
- F3 = 剪贴板贴图（`PinClipboardVirtualKey=0x72`）

## 4. 已完成（截至 2026-09-05 傍晚）

### 截图 / 标注（Snipaste 对齐，公开行为，不逆向）

- F1 区域截图；松手出 **13 键浅色图标工具栏**（非文字条）  
- 实线蓝选区框 + 八锚点 + 尺寸 `宽 × 高 px`（**无虚线、无三分辅助线**；Joe 明确不要虚线框、不要红框）  
- 悬停 **窗口吸附**（`WindowPicker` / `HoverAt`）  
- 选区可 **拖移**（无画笔工具时拖预览）；锚点缩放  
- 复制 / 钉 / 存 / 关；中键钉；Enter/双击复制；Esc/右键取消  
- **冻屏修复**：标注前 `CloseOverlays()`；标注窗自带可点压暗层；关钮/右键/点暗约 400ms 防误触  
- 参照图：`design/snipaste-ref/`（尤其 `07-toolbar-live-2.png`）  
- 规格：`design/CAPTURE-PIN-SPEC.md`、`design/SNIPASTE-TOOLBAR-SPEC.md`、`DESIGN.md`

### 贴图

- 钉图彩虹向外光环（非白底硬框）；浅/深桌面都要看得见  
- 当前透明度约：Dark far/near rest **0.32/0.42**；Light **0.48/0.58**；色偏柔和粉彩  
- 拖移、滚轮缩放、Alt+滚轮透明度、右键菜单  
- **鼠标穿透**：开后贴图点不着 → 托盘 **「取消贴图穿透」**

### 其它已接

- 托盘主题切换文案：「切换系统深浅色」  
- 单进程架构（透明崩溃会拖死截图/网速；Joe 已接受）  
- Win11 TAP DLL 曾编入并拷到输出旁；透明「背景节点找不到 / 就绪信号过早」**未完全修好**

## 5. 未做 / 下一步（Build 优先顺序）

### P0 — Joe 已排队

1. **任务栏网速显示美化**  
   - 字小、偏上、缺 TrafficMonitor 级字号/颜色等设置  
   - 规格草稿：`plans/PROMPT-POLISH-SETTINGS-NETSPEED.md`、`plans/SKILL-PICK-SETTINGS-NETSPEED.md`  
   - 网速必须 **嵌在任务栏**，不要只靠悬浮窗  
   - **禁止**引入 TrafficMonitor 源码（Anti-996）

2. **Win11 任务栏透明 TAP 时序**  
   - 就绪信号发太早 → 树未扫完就应用 → 背景节点找不到  
   - 材料：`plans/PROMPT-FIX-TAP-FILLS.md`、`plans/P2-WIN11-HANDOFF.md`、`plans/PROMPT-P2-WIN11-TAP.md`  
   - 杀软告警、闪烁、系统更新失效：Joe 已知晓并接受

### P1 — 设置页

- Joe 说过设置页丑；曾让路给 Win11/截图。网速美化时可一并按 design skill 改。

### 不要做

- 逆向 Snipaste / 抄 ShareX 二进制资源  
- 把 TrafficMonitor 源码掺进树  
- 默认改回「松手即钉」或默认 Ctrl+Shift+F12  
- 把选区框改回红色/虚线（除非 Joe 新说）

## 6. 已知坑

| 坑 | 处理 |
| --- | --- |
| `dotnet build Suite.sln` 编 C++ 失败 | 只编 `Suite.App.csproj`；TAP/Native 用已有 DLL |
| CopyFromBox 到 tsumon 常掉线 | 用本机已有树 + PowerShell 补丁，或重新解 tar |
| 标注窗透明点穿曾盖住工具栏 | 已关遮罩后出标注；勿再只留透明洞 |
| `cancel-annotate` 日志 | 误关/右键/点暗；有 400ms arm |
| 穿透后无法点贴图 | 托盘「取消贴图穿透」 |
| Debian/box **无** .NET 10 Windows Desktop | 不能在 Linux 声称 UI 已绿 |

## 7. 关键文件地图

```
src/Suite.App/          托盘、热键、设置窗、组装
src/Suite.Capture/      截图遮罩、吸附、标注工具栏
src/Suite.Pinboard/     贴图 + 彩虹光环
src/Suite.NetSpeed/     网速（待美化）
src/Suite.ThemeToggle/  深浅色
src/TaskbarFx.*/        透明（TAP/Native）
design/                 规格与 snipaste-ref 截图
plans/                  历史 PROMPT / HANDOFF（以本文件为最新总览）
tests/                  CaptureUx 等
```

## 8. 给 Build 的开场指令（可直接粘贴）

```
你在 Windows 合集 Suite 上工作。源码：C:\Users\Administrator\suite-app-fresh\windows-suite-app\
（权威也可从 /workspace/windows-suite-app/ 同步）。先读 plans/BUILD-HANDOFF-20260905.md。

当前截图/贴图主路径已可用（F1、蓝实线框、工具栏、吸附、选区拖移、彩虹光环）。
下一步按 Joe 排队：
1) 任务栏网速显示美化（字号/颜色/垂直对齐 + 设置项；自写，禁 TrafficMonitor 源码）
2) Win11 TAP 透明时序（PROMPT-FIX-TAP-FILLS.md）

编：dotnet build src\Suite.App\Suite.App.csproj -c Debug -p:Platform=x64
跑：重启 Suite.exe。汇报用中文大白话。不要逆向 Snipaste。
```

## 9. 打包

同日产物：`suite-build-handoff-20260905.tar`（源码，无 bin/obj），内含本交接文件。

---

## 附录：2026-09-06 续

P0 网速/设置抛光与 TAP 时序核对已在 box 完成，见：

- `plans/BUILD-HANDOFF-20260906.md`
- `plans/POLISH-HANDOFF.md`
- `plans/P2-WIN11-HANDOFF.md` §10

Windows 真机验收仍待 tsumon 开机。

