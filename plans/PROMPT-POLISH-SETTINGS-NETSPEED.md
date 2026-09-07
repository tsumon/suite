# 实现：设置页美化 + 任务栏网速像 TrafficMonitor

你是 coding。必读：
- DESIGN.md（若本轮 design 已更新则用新版；否则仍读现有）
- design/PROMPT-SETTINGS-NETSPEED.md 的产出（SETTINGS-PAGE-SPEC / 更新的 NETSPEED-TASKBAR-SPEC）；若文件尚未出现，先按下方 Joe 硬需求做，后补对齐
- 反例 design/joe-netspeed-ugly.png

## Joe 硬需求
1. 设置页不要丑：分组清晰、间距对齐、控件别挤成一坨默认 WinForms 感；WPF 按 DESIGN Operate
2. 网速设置补 TrafficMonitor 常用项：字号、文字颜色（可上/下行分开）、显示上行/下行开关、背景透明或着色、网卡、钉栏/悬浮
3. 任务栏网速 widget：字更大、**垂直居中**（勿偏上留底空）、边框更克制或可关；嵌入高度贴合任务栏

## 约束
- 单进程；不抄 TrafficMonitor 源码
- 改完更新 plans/POLISH-HANDOFF.md 大白话
- 同步注意：Windows 真机路径 suite-app-fresh；缺 Native 仍可跑
- DONE

若 design 规格文件在开始时不存在：等最多 3 分钟再读一次；仍无则按硬需求实现并在 HANDOFF 注明「规格后补」。
