# 任务：Windows 合集 App 竞品与技术路线调研

你是远程 Grok Build 的「调研」。只做调研，不写业务代码、不定视觉、不出面试材料。

## 目标产品（Joe 口述，2026-09-05）

做一个 **Windows 桌面 App**，合集三类体验：

1. **Snipaste** 级别的截图 / 贴图（截图、置顶贴图、标注等核心能力）
2. **TranslucentTB** 级别的任务栏透明 / 亚克力效果
3. **TrafficMonitor** 级别的桌面/状态栏网速（上传下载实时显示）

约束：**最大化使用 Grok Build** 做后续实现；本轮只调研。

## 产出要求

写到：`/workspace/windows-suite-app/research/REPORT.md`

必须中文。每条结论带出处（官方站 / GitHub / 文档 URL + 你核对时的日期）。找不到就写「找不到」，禁止编造 star 数、版本号、API。

### REPORT.md 结构

1. **一句话结论**：合不合理做成一个进程/一个安装包；最大技术风险各是什么
2. **三款对标拆解**（各一节）：
   - 核心功能清单（只列 Joe 可能要对齐的）
   - 开源与否、许可证、主要语言/框架
   - 实现手段摘要（Win32 API / UWP / XAML Islands / DWM / GDI+ / DXGI 等，有证据再写）
   - 已知坑（权限、Win11 任务栏改版、高 DPI、多显示器、杀软误报等）
3. **合集方案对比**（至少 3 档）：
   - A. 原生：C# + WinUI 3 / WPF
   - B. C++/Rust + Win32
   - C. 跨端壳：Tauri / Electron（若明显不适合也要写清为什么）
   - 每档：适合点、不适合点、和三功能的匹配度、后续用 Grok Build 写代码的友好度（Joe 主要会 Python，C#/Rust 熟悉度未知）
4. **推荐栈 + MVP 边界**
   - 建议第一版先做哪 2–3 个能力
   - 明确 Out of scope（别一口吃成 Snipaste Pro）
5. **可参考的开源仓库**（用 `gh api` 或网页核对；写 owner/repo、许可证、最近活跃迹象；核对不了就说核对不了）
6. **下一步给产品/架构的问题清单**（最多 5 个阻塞问题）

## 执行方式

- 用 web search / fetch 查官方与 GitHub；有 `gh` 就用 `gh api repos/...` 核事实
- 不要 git push，不要建无关仓库
- 写完 REPORT.md 后在同目录写一页 `SOURCES.md`：链接列表即可
- 完成后在 stdout 打印 `DONE` 和 REPORT.md 路径

日期语境：2026-09-05（Asia/Shanghai）。
