# Suite 便携 zip 布局

不写注册表也可跑（登录启动除外：便携默认不写 HKCU Run）。

```
Suite-portable/
  Suite.exe
  Suite.dll
  Suite.deps.json
  Suite.runtimeconfig.json
  # 以及 publish 带出的其它托管依赖
  TaskbarFx.Native.dll          # 旁路 exe；缺则任务栏效果不可用
  TaskbarFx.Tap.dll            # Win11 TAP；缺则现代任务栏路径失败
  LICENSE
  NOTICE
  vendor/                      # 可选：对应源码提示；发行 zip 可只放 NOTICE 指向仓库
```

## 用法

1. 整夹解压到任意可写目录（勿拆散 DLL）
2. 双击 `Suite.exe`
3. 设置里可勾「登录时启动」（写 HKCU Run → 指向该 exe 绝对路径）

## 不要

- 不要声称此 zip 已在 box 打出
- 不要把 TrafficMonitor 二进制塞进便携包
- 不要省略 `LICENSE` / `NOTICE`（GPL）
