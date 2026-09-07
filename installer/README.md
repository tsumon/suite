# Suite 安装包骨架

**未在 Windows 上编译 Inno Setup。** 本目录只有脚本草稿与便携布局说明。  
tsumon 开机后用 [Inno Setup 6](https://jrsoftware.org/isinfo.php) 打开 `Suite.iss` 编译。

## 文件

| 文件 | 用途 |
| --- | --- |
| `Suite.iss` | Inno Setup 草稿：Program Files、开始菜单、可选登录启动、旁路 DLL、GPL |
| `PORTABLE.md` | 便携 zip 目录约定 |
| `README.md` | 本说明 |

## 编译前（Windows）

1. `dotnet publish src\Suite.App\Suite.App.csproj -c Release -p:Platform=x64 -r win-x64 --self-contained false`
2. 编出 `TaskbarFx.Native.dll` / `TaskbarFx.Tap.dll`（MSBuild x64）并拷到 publish 目录旁路 `Suite.exe`
3. 按 `Suite.iss` 里 `Source` 路径改成你的 publish 输出
4. 杀软可能拦 TAP DLL — 见设置页任务栏效果失败句；发行说明需写明

## 许可

Suite 与改编自 TranslucentTB 的 TAP 路径均为 **GPL-3.0**。安装结束页应能打开 `LICENSE` / `NOTICE`。
