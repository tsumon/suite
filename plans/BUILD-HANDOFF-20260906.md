# Suite → Build 交接（2026-09-06）

接续 `plans/BUILD-HANDOFF-20260905.md`。权威源码仍在 box：`/workspace/windows-suite-app/`。

## 状态一句话

**代码已在 box 写完**（网速外观 + 设置页分组 + TAP 时序核对 + OCR/滚动/恢复默认/内存释帧）。Windows（tsumon）未开机，**未编、未跑、UI 未绿**。详见 `BOX-DONE-20260906.md`。

## 本轮交付

1. **P0 网速抛光** — 见 `plans/POLISH-HANDOFF.md`
2. **Win11 TAP 时序** — 源码已满足 PROMPT-FIX-TAP-FILLS；见 `plans/P2-WIN11-HANDOFF.md` §10（verified on box source 2026-09-06; awaiting tsumon boot）
3. schema 仍为 **3**；新外观字段缺省即默认，反序列化 `Normalize()`

## tsumon 开机后

```bat
cd C:\Users\Administrator\suite-app-fresh\windows-suite-app
REM 先把 box 树同步过来（tar / 手工均可）

dotnet build src\Suite.App\Suite.App.csproj -c Debug -p:Platform=x64
dotnet test tests\Suite.Contracts.Tests\Suite.Contracts.Tests.csproj -c Debug -p:Platform=x64
dotnet test tests\Suite.NetSpeed.Tests\Suite.NetSpeed.Tests.csproj -c Debug -p:Platform=x64

REM TAP 本轮源码无改动；若本地 DLL 旧/缺，仍建议 MSBuild 重编一次：
msbuild src\TaskbarFx.Tap\TaskbarFx.Tap.vcxproj /p:Configuration=Debug /p:Platform=x64
msbuild src\TaskbarFx.Native\TaskbarFx.Native.vcxproj /p:Configuration=Debug /p:Platform=x64

taskkill /IM Suite.exe /F
start "" "src\Suite.App\bin\x64\Debug\net10.0-windows10.0.19041.0\Suite.exe"
```

### 冒烟清单

| # | 动作 | 预期 |
| --- | --- | --- |
| 1 | 托盘可见；F1 截图工具栏 | 截图钉图未被本轮弄坏 |
| 2 | 任务栏网速 | 字约 13、垂直居中、默认无框；对照勿再现 ugly 图 |
| 3 | 设置五组 | 常规/截图/网速/任务栏效果/关于 |
| 4 | 网速字号/颜色/显隐 → 应用 | 槽变；不点应用不变 |
| 5 | 上下行都关 → 应用 | 「至少显示上行或下行。」不写盘 |
| 6 | 启用任务栏效果 · 全透明 | 「任务栏效果已应用。」；失败则 PendingFills 中文句 |
| 7 | 关任务栏效果 → 应用 | 系统默认；explorer 仍在 |

## 不要做

- 引入 TrafficMonitor 源码 / 逆向 Snipaste
- 在 Debian 上声称 UI 已绿
- 默认改回松手即钉或 Ctrl+Shift+F12

## 残余风险

- Win11 布局挤槽 → 悬浮回退（预期）
- 杀软拦 TAP（已知）
- box 无 `dotnet`，单测未在此机执行

## 附录 — P1 改进 backlog（2026-09-06 续）

Joe 点名六项的优先序、DoD、多屏残余见：

**`plans/BACKLOG-P1-20260906.md`**

本轮 box 已落：热键冲突处理、安装包骨架（`installer/`）、多屏最大重叠 + 贴图 DPI、OCR/滚动合约与 stub、设置窗 120ms 淡入。  
**仍未在 Windows 编跑。** 不要声称安装包已打出或 UI 已绿。
