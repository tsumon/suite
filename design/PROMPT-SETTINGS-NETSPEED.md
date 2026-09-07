# design：设置页 + 任务栏网速（必须用 skill）

严格按 `plans/SKILL-PICK-SETTINGS-NETSPEED.md` 执行。先读并遵守 `design-md-workflow`。

## Joe 问题
1. 整个设置页丑
2. 网速缺 TrafficMonitor 级外观设置（字号/颜色/显隐行/背景等）
3. 任务栏网速字小、偏上、底下空（反例 `design/joe-netspeed-ugly.png`）
4. **新增：** 托盘右键就能切系统深浅色，不必进设置。代码已有菜单项，文案改为「切换系统深浅色」；规格里写清：主入口在托盘，设置页保留次要按钮。

## 必开（按路由顺序，具名切片）
- design-md-workflow
- intended-vs-implemented（三列）
- Impeccable **Operate** + `/shape`（压缩 brief 不问卷）+ `/layout` + `/typeset` + `/distill` + `/clarify`
- design-engineering 节点：using-design-md, marketing-vs-product-ui, ai-default-tells, cards-design（反用）, forms-validation（切片）, copy-voice, animation-decision-framework（不要动效）, border-radius, states-are-the-work
- ponytail full

## 禁止
Taste / Frontend Design / UI Brain / UI/UX Pro Max；`/document` 当第一步；`/bolder` `/delight` `/animate` `/colorize`；GSAP / transitions；从丑代码倒抽 DESIGN。

## 产出（三份齐才 DONE）
1. 更新根目录 `DESIGN.md`（改掉把丑写死的 rate-taskbar / 槽高 / padding token）
2. 更新 `design/NETSPEED-TASKBAR-SPEC.md`
3. 新建 `design/SETTINGS-PAGE-SPEC.md`（含托盘主题入口说明）

不要写 `src/` 业务代码。中文。DONE。
