# UX 第二轮：光环 + 自动选窗 + 去掉土工具栏

第一轮已完成（勿回退）：松手 1:1 pin、Esc、网速 SetParent 默认嵌入。见 `plans/UX-FIX-HANDOFF.md`。

## 本轮必须（Joe 追加）

1. **贴图向外光环/光晕**（Snipaste 感）：边缘外扩柔光/阴影，从桌面可辨；标题条保持叠在图上悬停才显，**不要**厚白边土框。
2. **微信式自动选窗**：截图模式下鼠标移动自动高亮窗口（可用 WindowFromPoint / GetWindowRect；控件级用 UI Automation 能做就做，做不到至少顶层窗）；单击即截该矩形并走默认「立刻 1:1 pin」；仍可拖拽手选；Esc 取消。禁止逆向 Snipaste。
3. **不要**截完后强制弹出红框那种厚白图标工具栏当主流程。默认松手即 pin；标注仅 Shift+松手或贴图右键轻量入口。若 AnnotationWindow 仍是厚工具条，改成极简或弱化。
4. 参考反例图：`plans/joe-hate-toolbar-ref.png` 与 `plans/UX-ADD-GLOW-WINDOW.md`。

## 交付

- 更新 `plans/UX-FIX-HANDOFF.md`（追加第二节）
- 可测则补测试（选窗矩形纯函数等）
- 不改坏 TaskbarFx；不 git push；DONE + 路径
