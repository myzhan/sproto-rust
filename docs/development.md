# sproto-rust 工程开发指南

## 项目结构

```
sproto-rust/
  Cargo.toml                -- 主 crate 配置
  src/                      -- 核心库源码
    lib.rs                  -- 公共 API 导出
    error.rs                -- 错误类型定义
    types.rs                -- 模式元数据类型 + Builder API
    codec/                  -- 编解码模块
      mod.rs                -- 模块导出: StructEncoder, StructDecoder, DecodedField
      wire.rs               -- 小端读写原语、常量
      encoder.rs            -- StructEncoder: tag-based 结构体编码引擎
      decoder.rs            -- StructDecoder: tag-based 结构体解码引擎
    pack.rs                 -- 零压缩
    binary_schema.rs        -- 二进制模式加载
    rpc/                    -- RPC 模块
      mod.rs
  sproto-lua/               -- Lua FFI 绑定 crate
    Cargo.toml
    src/
      lib.rs                -- Lua 模块入口
      lua_codec.rs          -- LuaTable <-> wire bytes (基于 StructEncoder/StructDecoder)
      userdata.rs           -- Lua userdata 封装
      error.rs              -- 错误转换
  tests/                    -- 集成测试
    direct_tests.rs         -- StructEncoder/StructDecoder 编解码测试（含 C 二进制对比）
    pack_tests.rs           -- pack/unpack 压缩测试
    binary_schema_tests.rs  -- 二进制模式加载测试
    rpc_tests.rs            -- RPC 功能测试
    testdata/               -- C/Lua 生成的二进制固定文件
      generate.lua          -- 生成固定文件的 Lua 脚本
      build.sh              -- 构建脚本（需要 Lua 5.3+ 和 C sproto）
      *.bin                 -- 各种编码/打包/模式的二进制文件
  benches/                  -- 基准测试
    sproto_bench.rs         -- criterion 微基准测试
    benchmark.rs            -- 跨语言对比基准（cargo example）
    benchall.sh             -- 自动化 Rust/Go 对比测试脚本
  docs/                     -- 项目文档
```

## 前置环境

- **Rust**: 稳定版工具链 (edition 2021)
- **Cargo**: Rust 包管理器（随 Rust 一起安装）
- 可选: Lua 5.3+（用于重新生成 testdata 固定文件）
- 可选: Go（用于运行跨语言基准测试对比）

## 构建

```bash
# 构建主 crate
cargo build

# 构建整个 workspace（含 sproto-lua）
cargo build --workspace

# Release 模式（启用 LTO 和单 codegen-unit 优化）
cargo build --release
```

Cargo.toml 中的 release profile 配置：

```toml
[profile.release]
lto = true
codegen-units = 1
```

## Workspace 成员

项目是一个 Cargo workspace，包含以下成员：

```toml
[workspace]
members = ["sproto-lua"]
```

- `sproto-lua`: Lua FFI 绑定，将 Rust 实现暴露为 Lua 可加载模块

## 运行测试

```bash
# 运行所有 workspace 测试
cargo test --workspace

# 运行特定测试文件
cargo test --test direct_tests
cargo test --test rpc_tests
cargo test --test pack_tests

# 运行特定测试函数
cargo test test_decode_person

# 显示测试输出
cargo test -- --nocapture
```

### 测试分类

| 测试类别 | 文件 | 数量 | 说明 |
|---------|------|------|------|
| 单元测试 | src/ 各模块内 | 29 | 组件级测试（线格式读写、pack/unpack、RPC header、StructEncoder/StructDecoder 往返等） |
| Direct 测试 | direct_tests.rs | 50 | StructEncoder/StructDecoder 编解码，含 C 生成二进制对比 |
| 压缩测试 | pack_tests.rs | 24 | pack/unpack 交叉验证 |
| 二进制模式 | binary_schema_tests.rs | 2 | 加载 C 生成的 .bin 模式 |
| RPC 测试 | rpc_tests.rs | 15 | RPC 功能（dispatch、session、协议配置、错误处理） |
| **合计** | | **120** | 全部通过 |

### 测试策略

**交叉验证测试**: 通过 `tests/testdata/generate.lua` 使用 C/Lua sproto 库生成二进制固定文件（.bin），Rust 端解码这些文件并验证字段值，或编码后与 C 输出字节精确对比，确保线格式兼容。

**自洽往返测试**: 不依赖外部实现，验证 `decode(encode(value)) == value`、`unpack(pack(data)) == data`、完整管线 `encode -> pack -> unpack -> decode`。

**测试覆盖要点**: 测试不仅验证字段值，还验证：
- 嵌套结构体的正确编解码
- 字段数量检查
- 边界值、Unicode 字符串、空数据等边界情况

## 重新生成测试固定文件

如果修改了编码逻辑或需要添加新的测试用例：

```bash
cd tests/testdata
# 需要 Lua 5.3+ 以及 C sproto 库
bash build.sh
```

`generate.lua` 使用 C/Lua sproto 库产生所有 `*.bin` 固定文件。

## 基准测试

### Criterion 微基准测试

使用 [criterion](https://crates.io/crates/criterion) 进行细粒度性能测量：

```bash
# 运行所有 criterion 基准
cargo bench --bench sproto_bench

# 运行特定基准
cargo bench --bench sproto_bench -- encode
cargo bench --bench sproto_bench -- decode
cargo bench --bench sproto_bench -- pack
```

基准测试文件位于 `benches/sproto_bench.rs`，覆盖：
- encode / decode（StructEncoder/StructDecoder API）
- pack / unpack
- 不同数据复杂度的消息（简单 Person、复杂 UserProfile、大数组 DataSet）

### 跨语言基准测试

`benches/benchmark.rs` 是一个 cargo example，支持与 Go 实现对比：

```bash
# 构建并运行 Rust 基准
cargo build --release --example benchmark
./target/release/examples/benchmark --count=1000000 --mode=encode

# 可用参数:
#   --count=N        迭代次数
#   --mode=MODE      encode | decode | encode_pack | unpack_decode
```

### 自动化对比脚本

```bash
# 运行 Rust + Go 完整对比（需要 Go 环境和 gosproto 项目）
bash benches/benchall.sh 1000000

# 仅运行 Rust 部分
bash benches/benchall.sh
```

## 代码检查

```bash
# 运行 clippy lint（全 workspace，视警告为错误）
cargo clippy --workspace -- -D warnings

# 格式化检查
cargo fmt -- --check

# 格式化
cargo fmt --all

# 完整 CI 检查
make ci
```

## 依赖管理

### 运行时依赖
- `thiserror`: 错误类型派生

### 开发依赖
- `pretty_assertions`: 可读性更好的测试失败输出
- `criterion`: 基准测试框架

## 常见开发任务

### 添加新的 sproto 类型支持

1. 在 `src/types.rs` 的 `FieldType` 枚举中添加新变体
2. 更新 `src/codec/encoder.rs` 添加对应的 `set_*` 方法
3. 更新 `src/codec/decoder.rs` 添加对应的 `as_*` 方法
4. 在 `tests/direct_tests.rs` 中添加测试

### 修改线格式

1. 修改 `src/codec/wire.rs` 中的底层读写操作
2. 更新 encoder.rs 和 decoder.rs
3. 重新生成 testdata 固定文件（`cd tests/testdata && bash build.sh`）
4. 运行全量测试确保兼容性：`cargo test --workspace`

### 添加新的 RPC 功能

1. 修改 `src/rpc/mod.rs`
2. 在 `tests/rpc_tests.rs` 中添加测试
3. 如果涉及新的包头字段，可能需要更新 tests/testdata 中的 RPC 固定文件
