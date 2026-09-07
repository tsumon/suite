# 决策变更（Joe 2026-09-05）

**原 ADR：** Suite.exe + TaskbarFx.exe 双进程。  
**现决定：** **单进程**。任务栏透明与截图/贴图/网速同进程。

Joe 明确：接受透明挂了截图也挂。

影响：
- 后续实现把 TaskbarFx 能力收进 Suite（可仍用 Native DLL，但由主进程加载，不必独立 Host EXE + 管道）。
- 设置 UI 不再「拉起另一程序」。
- 原 AUDIT 管道对端校验等门禁降级/删除；注入/SWCA 风险仍在，只是不再隔离。

## P2 补充（2026-09-06）

仍单进程。INTERACTION-P2 §2 用 `CrashGuard` + 连续失败自动关任务栏效果做兜底（截图/网速尽量保住）。  
**独立 TaskbarFx 进程**仍是 future 选项，本轮不拆；设置里无「透明独立进程」开关。
