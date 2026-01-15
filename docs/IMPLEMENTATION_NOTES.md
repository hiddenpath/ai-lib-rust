# 实现说明：从 ai-lib 学习的改进

## 已完成的改进

### 1. JSONPath 工具改进 (`src/utils/json_path.rs`)

借鉴了 `ai-lib` 的 `PathMapper` 实现：

- ✅ **PathMapper**：支持嵌套路径访问（`a.b.c`）和数组索引（`choices[0].delta.content`）
- ✅ **JsonPathEvaluator**：实现了完整的条件匹配，支持：
  - `exists($.path)` - 路径存在检查
  - `$.path == "value"` - 相等性检查
  - `$.path != "value"` - 不等性检查
  - `$.path in ['value1', 'value2']` - 列表成员检查
  - `$.path != null` / `$.path == null` - null 检查
  - `&&` 和 `||` - 逻辑组合

### 2. 事件映射器改进 (`src/pipeline/event_map.rs`)

- ✅ 使用 `PathMapper::get_string` 和 `PathMapper::get_path` 替代简单的 `pointer` 方法
- ✅ 支持更复杂的路径表达式（包括数组索引）

## 待改进项

### 1. 解码器实现 (`src/pipeline/decode.rs`)

**建议**：使用 `tokio_util::codec::Decoder` trait，参考 `ai-lib/src/streaming/decoder.rs`

当前实现使用自定义的 stream unfold，可以改为：
```rust
use tokio_util::codec::Decoder;

impl Decoder for SseDecoder {
    type Item = DecodedFrame;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // 实现解码逻辑
    }
}
```

### 2. 累积器改进 (`src/pipeline/accumulate.rs`)

**建议**：参考 `ai-lib/src/streaming/pipeline.rs` 的 `StreamProcessor`

- 使用简单的 `String` 累积器（当前实现）
- 支持多个工具调用的并发累积（需要 `HashMap<ToolCallId, String>`）
- 改进 `flush_on` 条件的处理

### 3. 条件匹配完善

**当前状态**：已实现基本的条件匹配

**待完善**：
- 支持更复杂的表达式解析
- 支持数值比较（`>`, `<`, `>=`, `<=`）
- 支持正则表达式匹配

### 4. 路径设置功能

**建议**：添加 `PathMapper::set_path` 方法，用于在编译请求时设置嵌套路径

参考 `ai-lib/src/utils/path_mapper.rs` 的实现。

## 关键学习点

1. **使用标准库 trait**：`tokio_util::codec::Decoder` 比自定义实现更可靠
2. **状态管理**：累积器需要维护状态，使用 `Arc<Mutex<>>` 或简单的字段
3. **路径处理**：支持数组索引和嵌套路径是必需的
4. **条件匹配**：需要支持复杂的逻辑表达式

## 下一步行动

1. ✅ 更新 JSONPath 工具（已完成）
2. ⏳ 更新解码器使用 `tokio_util::codec::Decoder`
3. ⏳ 改进累积器支持多工具调用
4. ⏳ 添加路径设置功能
5. ⏳ 完善条件匹配表达式
