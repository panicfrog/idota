# dota2-assistant

Dota 2 辅助工具:自动定位 Dota 2 安装目录、写入 Game State Integration (GSI) 配置,并启动事件服务器,省去手写 `gamestate_integration_*.cfg` 的过程。

## 功能

| 命令 | 说明 |
| --- | --- |
| `find` | 定位 Dota 2,打印游戏根目录、执行文件、GSI 配置目录 |
| `setup` | 定位 Dota 2,并把 GSI 配置文件写入游戏的 `cfg` 目录 |
| `serve` | 启动 GSI 服务器,接收并打印 Dota 2 推送的事件 |

## 构建

需要 Rust(stable,建议通过 [rustup](https://rustup.rs) 安装)。Windows 上还需安装 Microsoft C++ Build Tools(MSVC 链接器)。

```sh
cargo build --release -p dota2-assistant
```

产物位于 `target/release/`,Windows 下是可执行文件 `dota2-assistant.exe`。

## Windows 运行步骤

1. 安装 Rust 与 MSVC Build Tools,在 PowerShell 中执行:

   ```powershell
   cd <项目目录>
   cargo build --release -p dota2-assistant
   ```

2. 自动定位 Dota 2 并写入配置:

   ```powershell
   .\target\release\dota2-assistant.exe setup
   ```

   正常会打印找到的游戏根目录、执行文件和配置目录;若打印了找不到的报错,见下方「常见问题」。

3. 启动事件服务器(保持窗口不关):

   ```powershell
   .\target\release\dota2-assistant.exe serve --port 53000
   ```

4. Steam 中右键 Dota 2 → 属性 → 启动选项,填入 `-gamestateintegration`,然后重启 Dota 2。
   国服用户还要加上 `-perfectworld`,见下方「国服(完美世界)运行说明」。

5. 进入游戏后,`serve` 窗口会逐行打印事件摘要,例如:

   ```
   event: start / In Progress | hero=-
   ```

## Linux 运行步骤

与 Windows 相同,区别只是二进制名和 Steam 默认路径:

- 原生 Steam:`~/.steam/steam/steamapps/common/dota 2 beta`
- Flatpak Steam:`~/.var/app/com.valvesoftware.Steam/.local/share/Steam`

```sh
./target/release/dota2-assistant setup
./target/release/dota2-assistant serve --port 53000
```

## 国服(完美世界)运行说明

国服与默认客户端共用同一套 GSI 机制,区别只在启动参数和(可能的)游戏目录,`setup` / `serve` 的用法不变。

**通过 Steam 玩国服(推荐)**

在 Steam 启动选项里同时填入两个参数,用空格分隔:

```
-perfectworld -gamestateintegration
```

`-perfectworld` 进入国服(部分资料写作 `-perfectworld steam`,效果相同),`-gamestateintegration` 开启 GSI。游戏目录、配置路径与 `setup` / `serve` 完全不变;国际服和国服切换时,配置文件无需改动。

**完美世界独立客户端(非 Steam)**

如果仍在使用完美世界官网下载的独立客户端,需要注意两点:

1. 它的安装目录与 Steam 版不同(常见于 `C:\Program Files (x86)\Dota2`),`setup` 自动搜索不到,需要手动指定:

   ```powershell
   dota2-assistant.exe setup --path "C:\Program Files (x86)\Dota2"
   ```

   可执行文件通常在 `game\bin\win64\dota2.exe`。

2. 独立客户端无法通过 Steam 属性设置启动选项,需要在快捷方式里加参数:

   - 右键 `game\bin\win64\dota2.exe` → 创建快捷方式;
   - 右键快捷方式 → 属性 → 「目标」末尾追加参数(注意前面的空格):

     ```
     "C:\Program Files (x86)\Dota2\game\bin\win64\dota2.exe" -gamestateintegration
     ```

   - 之后用这个快捷方式启动游戏。

独立客户端的 GSI 配置文件同样写入 `<安装目录>\game\dota\cfg\gamestate_integration_dota2_assistant.cfg`,`serve` 用法不变。

## 命令参考

所有命令都支持全局参数 `--path <PATH>`,手动指定 Dota 2 的位置,可传以下任意一种:

- Dota 2 可执行文件,如 `D:\Steam\steamapps\common\dota 2 beta\game\bin\win64\dota2.exe`
- 游戏根目录,如 `D:\Steam\steamapps\common\dota 2 beta`
- GSI 配置目录,如 `D:\Steam\steamapps\common\dota 2 beta\game\dota\cfg`

不传 `--path` 时自动搜索:Windows 的 Program Files / 用户目录、Linux/macOS 的常见 Steam 路径,并解析 `steamapps\libraryfolders.vdf` 覆盖多磁盘库目录。也可用环境变量 `DOTA2_ASSISTANT_STEAM_ROOT` 指定 Steam 根目录。

```sh
# 只定位,不写配置
dota2-assistant find

# 写入配置(默认端口 53000,可选 auth token)
dota2-assistant setup --port 53000 --token <token>

# 启动服务器(端口必须与 setup 一致)
dota2-assistant serve --port 53000
```

`serve` 默认以 info 级别打印事件,可用 `RUST_LOG=debug` 查看更详细日志。

## GSI 配置说明

`setup` 写入的文件:

```
<游戏根目录>/game/dota/cfg/gamestate_integration_dota2_assistant.cfg
```

文件内容包含:

- `uri`:指向 `serve` 监听的地址,端口不一致时收不到事件
- `data`:启用的数据段(provider、map、player、hero、abilities、items、buildings、draft、wearables)
- `auth`:可选 token,传入后 Dota 2 会在每个事件中附带 `auth.token`

## 常见问题

**`setup` 找不到 Dota 2**

Steam 装在非常规位置时自动搜索会失效,用 `--path` 指定:

```powershell
dota2-assistant.exe setup --path "D:\Steam\steamapps\common\dota 2 beta"
```

**提示端口被占用**

`Address already in use` 表示已有实例在监听。换一个端口,并保证 `setup` 和 `serve` 使用同一个:

```powershell
dota2-assistant.exe setup --port 53001
dota2-assistant.exe serve --port 53001
```

**收不到事件**

- 确认 Dota 2 以 `-gamestateintegration` 启动参数运行,且配置写入后重启过游戏
- 确认 `setup` 写入的 `uri` 端口与 `serve` 的 `--port` 一致
- Windows 防火墙若弹窗,选择允许访问(服务器只监听本机 127.0.0.1)
