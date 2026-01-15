# Schema 验证实现说明

## 概述

`ai-lib-rust` 使用 **GitHub 作为 schema 验证的唯一标准来源**，确保所有运行时使用相同的验证标准。

## 标准 Schema URL

```rust
const SCHEMA_GITHUB_URL: &str = 
    "https://raw.githubusercontent.com/hiddenpath/ai-protocol/main/schemas/v1.json";
```

## 加载策略

`ProtocolValidator::new()` 采用以下加载策略：

### 1. 优先：GitHub URL（标准来源）

- 从标准 GitHub URL 加载 schema
- 确保所有运行时使用相同的标准 schema
- 适用于生产环境和 CI/CD

### 2. Fallback：本地文件系统

- 如果 GitHub 不可用，从本地文件系统加载
- 支持离线开发
- 通过 `AI_PROTOCOL_DIR` 或 `AI_PROTOCOL_PATH` 环境变量指定路径

## 实现细节

### 代码结构

```rust
impl ProtocolValidator {
    /// 标准 GitHub URL（唯一标准来源）
    const SCHEMA_GITHUB_URL: &'static str = "...";
    
    pub fn new() -> Result<Self, ProtocolError> {
        // 1. 尝试从 GitHub 加载
        let schema_content = if let Ok(content) = Self::fetch_schema_from_github() {
            Some(content)
        } else {
            // 2. Fallback 到本地文件
            Self::load_schema_from_local()
        }?;
        
        // 编译并返回验证器
    }
}
```

### 网络请求

- 使用 `reqwest` 异步 HTTP 客户端
- 超时设置：10 秒
- 错误处理：网络错误时自动 fallback 到本地文件

### 本地文件查找顺序

1. `AI_PROTOCOL_DIR/schemas/v1.json`
2. `AI_PROTOCOL_PATH/schemas/v1.json`
3. `ai-protocol/schemas/v1.json`（相对路径）
4. `../ai-protocol/schemas/v1.json`
5. `../../ai-protocol/schemas/v1.json`

## 使用场景

### 生产环境

- 自动从 GitHub 加载最新 schema
- 确保使用标准验证规则
- 无需手动更新代码

### 本地开发

- 设置 `AI_PROTOCOL_DIR` 环境变量指向本地 ai-protocol 仓库
- 支持离线开发
- 可以测试本地 schema 变更

### CI/CD

- GitHub Actions 自动验证所有协议文件
- 使用标准 GitHub schema URL
- 确保所有变更符合规范

## 错误处理

### GitHub 不可用

- 自动 fallback 到本地文件
- 如果本地文件也不存在，返回明确的错误信息

### Schema 格式错误

- 返回详细的验证错误
- 包含 JSON Schema 验证失败的具体信息

## 性能考虑

### 缓存策略（未来优化）

可以考虑添加 schema 缓存：

1. **内存缓存**: 在应用生命周期内缓存编译后的 schema
2. **文件缓存**: 缓存下载的 schema 到本地文件系统
3. **版本检查**: 定期检查 GitHub 上的 schema 是否有更新

### 当前实现

- 每次创建 `ProtocolValidator` 时都会尝试从 GitHub 加载
- 如果 GitHub 可用，优先使用（确保使用最新标准）
- 如果 GitHub 不可用，使用本地文件（支持离线开发）

## 与 ai-protocol 的协作

### Schema 变更流程

1. **ai-protocol 仓库**:
   - 修改 `schemas/v1.json`
   - 提交 PR 并经过审查
   - 合并到 `main` 分支

2. **自动生效**:
   - `ai-lib-rust` 会在下次启动时自动获取最新 schema
   - 无需手动更新代码

3. **版本固定**（如需要）:
   - 可以使用 Git tag URL 固定版本
   - 例如: `https://raw.githubusercontent.com/hiddenpath/ai-protocol/v1.1.0/schemas/v1.json`

## 测试

### 验证工具

```bash
# 验证所有协议文件
cargo run --bin validate_protocols
```

### 单元测试

```rust
#[test]
fn test_schema_loading_from_github() {
    // 测试从 GitHub 加载 schema
}

#[test]
fn test_schema_loading_from_local() {
    // 测试从本地文件加载 schema
}
```

## 环境变量

- `AI_PROTOCOL_DIR`: 本地 ai-protocol 仓库路径（用于 fallback）
- `AI_PROTOCOL_PATH`: 同 `AI_PROTOCOL_DIR`（别名）

## 注意事项

- ⚠️ **网络依赖**: 生产环境需要网络访问 GitHub（或使用本地缓存）
- ⚠️ **超时设置**: 默认 10 秒超时，如果网络较慢可能需要调整
- ⚠️ **错误处理**: 如果 GitHub 不可用且无本地文件，验证会失败（这是预期的，确保使用标准 schema）

---

**最后更新**: 2026-01-06  
**相关文档**: [ai-protocol/SCHEMA_VALIDATION.md](../../../ai-protocol/SCHEMA_VALIDATION.md)
