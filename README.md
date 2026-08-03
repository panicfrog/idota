# idota

Rust workspace,基于 Dota 2 Game State Integration (GSI) 的工具集。

| Crate | 说明 |
| --- | --- |
| [dota-gsi](dota-gsi/README.md) | GSI 库:接收 Dota 2 推送的 HTTP 事件、完整的 JSON 数据模型、状态 diff 事件生成,附 echoslam / recall / killfeed 示例 |
| [dota2-assistant](dota2-assistant/README.md) | 应用:自动定位 Dota 2 安装目录、写入 GSI 配置文件、启动事件服务器,免去手动配置 |

## 构建

```sh
cargo build --release --workspace
```

## 快速开始

以 `dota2-assistant` 为例:

```sh
dota2-assistant setup            # 找到 Dota 2 并写入 GSI 配置
dota2-assistant serve            # 启动服务器,打印 Dota 2 推送的事件
```

详细步骤见 [dota2-assistant/README.md](dota2-assistant/README.md)。
