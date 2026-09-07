# 按设计规格实现（功能+UI）

你是 coding。先读再改，禁止发明另一套视觉：

1. `/workspace/windows-suite-app/DESIGN.md`
2. `/workspace/windows-suite-app/design/CAPTURE-PIN-SPEC.md`
3. `/workspace/windows-suite-app/design/NETSPEED-TASKBAR-SPEC.md`
4. `/workspace/windows-suite-app/architecture/DECISION-SINGLE-PROCESS.md`
5. `/workspace/windows-suite-app/plans/SKILL-PICK-UX.md`（Operate；diagnosing-bugs：能测先测）
6. 反例图 `plans/joe-hate-toolbar-ref.png`

## 架构

- **单进程 Suite**：任务栏透明/亚克力作为同进程模块（可 Native DLL 由 Suite 加载）。**不要**再依赖独立 TaskbarFx.exe + 命名管道作为主路径；若旧 Host 工程还在，改为可选/废弃说明，主路径是 Suite 内调用。
- 闭源；不拷 GPL/Anti-996。

## 必须落地

截图/贴图：松手即 1:1 pin+剪贴板；Esc/右键整次取消；向外光环；微信式自动选窗+手拖；默认无厚白工具栏（标注仅 Shift 或规格里的轻量入口）。
网速：默认钉任务栏；失败人话+回退悬浮。
透明：设置四档中文；Win10 SWCA；Win11 暂不可用要人话；同进程。

## 交付

- 更新 `plans/IMPLEMENT-HANDOFF.md` 大白话怎么试
- 不 git push；Debian 不假绿
- DONE + 路径
