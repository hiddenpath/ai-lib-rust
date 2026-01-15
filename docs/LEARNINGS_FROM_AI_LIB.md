# 从 ai-lib 项目学习的实现模式

本文档总结了从 `ai-lib` 项目中可以借鉴到 `ai-lib-rust` 的关键实现模式。

## 1. 解码器实现 (`streaming/decoder.rs`)

### 关键点：
- **使用 `tokio_util::codec::Decoder` trait**：这是标准的 tokio 解码器接口，比自定义实现更可靠
- **支持多种格式**：SSE, Anthropic SSE, Gemini JSON, Cohere NDJSON
- **`SseEventDecoder`**：专门处理多行 SSE 事件的解码器，支持 `event:` 和 `data:` 配对

### 可借鉴的代码模式：
```rust
impl Decoder for StreamingFrameDecoder {
    type Item = DecodedFrame;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // 查找分隔符
        // 解析行
        // 返回 DecodedFrame::Json, DecodedFrame::Done, 或 DecodedFrame::Skip
    }
}
```

## 2. 流式处理管线 (`streaming/pipeline.rs`)

### 关键点：
- **状态累积器**：使用 `accumulated: String` 来累积工具调用参数
- **条件匹配函数**：`evaluate_match` 支持复杂的表达式（`exists()`, `in`, `==`, `!=`, `&&`, `||`）
- **路径提取函数**：`get_value_by_path` 和 `get_string_by_path` 用于从 JSON 中提取值

### 可借鉴的代码模式：
```rust
pub struct StreamProcessor {
    cfg: StreamingConfig,
    accumulated: String,  // 状态累积
}

impl StreamProcessor {
    pub fn process(&mut self, root: &Value) -> Option<StreamingEvent> {
        // 1. 帧过滤
        // 2. 累积器处理
        // 3. 事件映射规则匹配
        // 4. stop_condition 检查
        // 5. flush_on 处理
    }
}
```

## 3. SSE 解析器 (`sse/parser.rs`)

### 关键点：
- **事件边界查找**：`find_event_boundary` 支持 LF+LF 和 CRLF+CRLF
- **配置驱动解析器**：`ConfigDrivenParser` 完全由配置驱动，支持多种格式
- **格式特定转换**：每种格式都有专门的转换函数（`anthropic_to_chunk`, `gemini_to_chunk` 等）

### 可借鉴的代码模式：
```rust
pub fn find_event_boundary_with_delim(buffer: &[u8], delimiter: Option<&str>) -> Option<usize> {
    // 查找精确分隔符匹配
    // 回退到 CRLF+CRLF
}
```

## 4. 映射引擎 (`mapping/engine.rs`)

### 关键点：
- **参数映射规则**：支持直接映射、条件映射、转换映射
- **负载格式转换**：针对不同 payload format（OpenAI, Anthropic, Gemini）的特定转换
- **路径设置**：`set_path_value` 支持嵌套路径设置

### 可借鉴的代码模式：
```rust
pub struct MappingEngine {
    parameter_mappings: HashMap<String, MappingRule>,
    payload_format: PayloadFormat,
}

impl MappingEngine {
    pub fn transform_request(&mut self, request: &ChatCompletionRequest) -> MappingResult<Value> {
        // 应用参数映射
        // 应用负载格式特定转换
    }
}
```

## 5. PathMapper 工具 (`utils/path_mapper.rs`)

### 关键点：
- **嵌套路径支持**：支持 `a.b.c` 这样的点分路径
- **数组索引支持**：支持 `choices[0].delta.content` 这样的数组访问
- **路径设置和获取**：`set_path` 和 `get_path` 方法

### 可借鉴的代码模式：
```rust
impl PathMapper {
    pub fn set_path(obj: &mut Value, path: &str, value: Value) -> Result<(), PathMapperError> {
        // 分割路径
        // 递归创建嵌套对象
        // 设置值
    }

    pub fn get_path<'a>(obj: &'a Value, path: &str) -> Option<&'a Value> {
        // 支持点分路径和数组索引
    }
}
```

## 6. 条件匹配实现 (`streaming/pipeline.rs` 中的 `evaluate_match`)

### 关键点：
- **支持多种操作符**：`exists()`, `in`, `==`, `!=`, `&&`, `||`
- **路径提取**：从 JSON 中提取值进行比较
- **逻辑组合**：支持 AND/OR 逻辑组合

### 可借鉴的代码模式：
```rust
fn evaluate_match(expr: &str, root: &Value) -> bool {
    // 分割 OR 部分
    // 对每个 OR 部分，分割 AND 部分
    // 评估每个条件
    // 返回结果
}
```

## 建议的改进方向

1. **使用 `tokio_util::codec::Decoder`**：替换当前的自定义解码器实现
2. **实现完整的条件匹配**：借鉴 `evaluate_match` 的实现
3. **添加 PathMapper**：用于路径提取和设置
4. **改进累积器**：使用状态机模式，支持多个工具调用的并发累积
5. **格式特定转换**：为不同格式添加专门的转换逻辑

## 注意事项

- ai-lib 使用的是单文件 manifest，而 ai-lib-rust 使用的是独立的 ai-protocol 项目
- ai-lib 有一些硬编码的逻辑，ai-lib-rust 应该完全由协议驱动
- 需要将 ai-lib 的实现模式适配到 ai-lib-rust 的算子架构中
