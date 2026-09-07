# coding：截图完完全全按 Snipaste

## 权威
先读（design 产出后必齐）：
- `design/CAPTURE-PIN-SPEC.md`（新）
- `design/SNIPASTE-TOOLBAR-SPEC.md`
- `DESIGN.md`
- 附图 `design/snipaste-ref/07-toolbar-live-2.png`（工具栏长相）
- `design/snipaste-ref/06-control-hotkeys.png`（F1/F3）

若 design 文件尚未更新：仍以附图为准实现，并同步改规格。

## 硬需求（Joe）
1. 全局热键 **F1** 截屏，**F3** 从剪贴板贴图
2. 框选/点窗松手后 **立刻** 出浅色 **图标** 工具栏（禁止纯文字条当主 UI）
3. 工具：矩形/椭圆、线/曲线、笔、马克笔、马赛克、文字、橡皮、撤销重做、关闭、钉、存、复制
4. 选区：尺寸标签、边框、八锚点可调；自动检测窗口
5. 双击选区=复制并退出；中键=钉图并退出；Enter=复制并退出；Esc 取消（可先不做确认框）
6. 钉图：用截屏位置、置顶、外阴影（Snipaste 贴图感）
7. 不逆向 Snipaste；可自绘简单几何图标

## 验收
tsumon：F1 → 拖选 → 松手看到与 07 同结构的图标栏 → 复制/钉/关可用。

写 `plans/SNIPASTE-CAPTURE-HANDOFF.md`。改 `Suite.Capture` / `Suite.Pinboard` / App 热键。DONE。

## Skill
按 `plans/SKILL-PICK-SNIPASTE-CAPTURE.md`。design 已 DONE：跟 `CAPTURE-PIN-SPEC` + `SNIPASTE-TOOLBAR-SPEC` + `DESIGN.md`，禁止再执行「去工具栏」。
附图权威：`design/snipaste-ref/07-toolbar-live-2.png`。
