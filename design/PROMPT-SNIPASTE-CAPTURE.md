# design：截图完完全全按 Snipaste

## 权威材料（只认这些）
Joe 附图目录 `design/snipaste-ref/`：
- `01-snip-display.png` 边框宽3、遮罩、显示锚点、放大镜选项、快捷键提示
- `02-snip-behavior.png` 自动检测窗口/界面元素、HDR、历史
- `03-snip-hotkeys.png` Esc确认、左键双击=复制并退出、中键=贴图并退出、回车=复制并退出
- `04-pin-display.png` 贴图阴影等
- `05-toolbar-live.png` / `07-toolbar-live-2.png` **松手后工具栏实况（主视觉目标）**
- `06-control-hotkeys.png` 全局 F1=截屏、F3=贴图

旧 `CAPTURE-PIN-SPEC.md` 把工具栏当反例 → **整份推翻重写**。禁逆向 Snipaste。

## 主路径（必须）
F1 → 遮罩+悬停选窗/拖选 → **松手立刻**出现浅色图标工具栏（贴选区下方，对照 07）→ 可标注/复制/钉/存/关。
不是文字按钮条，不是先钉再标。

## 工具栏图标顺序（对照 07）
形状(矩形/椭圆) · 曲线 · 铅笔 · 马克笔 · 马赛克 · 文字 · 橡皮 | 撤销 · 重做 | 关闭 · 钉图 · 保存 · 复制
选中工具可有红点。选区：尺寸片、描边、**八个锚点**、辅助线（按显示设置）。

## 产出
1. 重写 `design/CAPTURE-PIN-SPEC.md`
2. 新建 `design/SNIPASTE-TOOLBAR-SPEC.md`（像素级对照 07）
3. 更新 `DESIGN.md` 截图相关 token
若存在 `plans/SKILL-PICK-SNIPASTE-CAPTURE.md` 严格按其 skill 切片。禁 Taste 等。不改 src。DONE。
