# 决定：整包 GPL-3.0，允许使用 TranslucentTB

- **日期：** 2026-09-05
- **决定人：** Joe（拟合app 对话）
- **决定：** Suite **整包**以 **GPL-3.0** 开源；Win11 任务栏透明路径 **可以使用 / 改写 TranslucentTB**（同为 GPL-3.0）代码。
- **作废：** 原「闭源安装包 + 禁止拷贝 TranslucentTB / GPL」发行拍板（见旧 ADR §2.3 / 调研建议）。
- **仍自写、不掺：** TrafficMonitor（Anti-996，与 GPL 合集混用另有风险）；截图不拷 ShareX 源码也可继续自写（ShareX 亦 GPL，若要用需同样标注）。
- **单进程：** 整包 GPL 后，单进程不再有「闭源被传染」问题；注入崩溃仍会拖死截图/网速（用户已接受）。
- **合规最低动作：** 根目录 `LICENSE`（GPL-3.0）、`NOTICE`（TranslucentTB 版权与来源 URL）、衍生文件保留原版权头、提供对应源码（与安装包同版本）。
