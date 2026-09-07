# Plan — Win11 任务栏透明（P2，GPL + TranslucentTB）

Joe 已改拍板：整包 GPL-3.0，可用 TranslucentTB。目标：tsumon（Win11）上任务栏 Clear/Opaque/Acrylic 肉眼生效。

## Scope
- In: 接入/改写 TranslucentTB 的 Win11 XAML TAP 路径；Suite 设置档位驱动；LICENSE/NOTICE；失败人话；主屏（副屏尽力）
- Out: 动态模式全集；TrafficMonitor 源码；设置页美化（让路）

## Action items
- [x] 落 LICENSE(GPL-3.0) + NOTICE；记 DECISION-GPL-TRANSLUCENTTB.md
- [x] 取得 TranslucentTB 源码（git clone 到 vendor/ 或 sparse 所需目录），保留版权头
- [x] 把 ExplorerTAP / 任务栏 brush 能力接到现有 TaskbarFx Apply/Reset（或明确子模块边界）
- [x] 去掉「Win11 路径未交付」死胡同；成功 path=win11-xaml
- [ ] tsumon MSBuild/编测：Clear 肉眼透明（须 Windows 真机；Debian 未编）
- [x] 写 P2-WIN11-HANDOFF.md（怎么编、怎么恢复、杀软提示）
