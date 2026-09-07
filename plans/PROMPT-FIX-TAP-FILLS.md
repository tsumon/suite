# fix：Win11 TAP「background node was not found」

## 现象
Joe：无法应用任务栏效果 — `win11 taskbar background node was not found the visual tree may have changed`

## 根因（先核实再改）
高概率不是节点改名，而是时序：
1. `TAPSite::StartPump()` 里**立刻** `SetEvent(ready)`，早于 `AdviseVisualTreeChange` 走完现有树的 Add 回调。
2. Native `EnsureInjected` 一等到 ready 就 `Apply`，此时 `m_taskbars` 还没有 `BackgroundFill` → `TaskbarFxTapResultPendingFills` → 这句英文错误。
3. `BackgroundFill` 若已不是 `Rectangle`（变成 Border 等）也要兼容；对照 `vendor/TranslucentTB/ExplorerTAP/visualtreewatcher.cpp`。

## 必改
1. **ready 只能在 VisualTreeWatcher 已 Advise 之后**再置位（跟 TT 一样：Advise 线程里 SetEvent）。删掉 StartPump 里过早的 SetEvent，或改成「site 已挂」与「可 Apply」两个信号。
2. `ApplyWin11Xaml`：若 result=PendingFills，**重试/等待**（例如最多 5–8 秒，每 200ms 再 SetEvent apply），直到 fillCount>0 或超时；超时中文：`暂时找不到任务栏背景，请再点一次应用。`
3. `OnVisualTreeChange`：除 `Rectangle` 外，对名为 BackgroundFill/BackgroundStroke 的 `Shape`/`Border` 背景也注册；`RegisterTaskbar` 后可用 VisualTreeHelper 主动在 TaskbarFrame 下找 BackgroundFill（不依赖后续 mutation）。
4. 不要杀 explorer。保留 GPL/TT 版权头。

## 验收（tsumon）
清 → 任务栏肉眼变透；状态「任务栏效果已应用。」；截图钉图仍正常。

改 `TaskbarFx.Tap` + `taskbarfx_win11.cpp`（必要时文案）。写完更新 `plans/P2-WIN11-HANDOFF.md` 一句时序说明。DONE。
