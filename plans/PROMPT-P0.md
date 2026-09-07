# 任务：P0 实现计划（create-plan）

你是远程 Grok Build。结合 create-plan 思路，为已冻结 ADR 写出 **P0 可执行计划**。不写业务代码、不定视觉。

先读：
- /workspace/windows-suite-app/architecture/ADR.md（accepted）
- /workspace/windows-suite-app/research/REPORT.md（需要时）

## P0 范围（ADR §7，无注入）

Suite 托盘、单实例、设置 JSON、开机启动；ThemeToggle；NetSpeed 桌面窗；热键注册占位；Windows 真机 `dotnet build` 绿。不做 TaskbarFx 注入、不做截图贴图完整实现（热键可只占位）。

栈：C# .NET 10 + WPF；Grok 写 Joe 审；远程 Debian 不能真机编 WPF——计划里写清「远程出码 / Windows 真机编验」。

## 产出

写到：`/workspace/windows-suite-app/plans/P0.md`

模板：
```markdown
# Plan — Suite P0

<1–3 句：what/why/approach>

## Scope
- In:
- Out:

## Action items
- [ ] ...（6–10 条，动词开头，有文件/命令名）

## Open questions
- 最多 3 个
```

可验证标准对齐 ADR P0 退出条件。写完打印 DONE 与路径。
