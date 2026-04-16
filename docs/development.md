# sproto-rust 工程开发指南

## 项目结构

```
sproto-rust/
  Cargo.toml                -- 主 crate 配置
  src/                      -- 核心库源码
    lib.rs                  -- 公共 API 导出
    error.rs                -- 错误类型定义
    types.rs                -- 模式元数据类型
    derive_traits.rs        -- SprotoEncode / SprotoDecode trait
    codec/                  -- 编解码模块
      mod.rs, wire.rs
    pack.rs                 -- 零压缩
    parser/                 -- 文本模式解析器
      mod.rs, lexer.rs, ast.rs, grammar.rs, schema_builder.rs
    binary_schema.rs        -- 二进制模式加载
    rpc/                    -- RPC 模块
      mod.rs
    serde/                  -- Serde 集成 (feature = "serde")
      mod.rs, ser.rs, de.rs, error.rs
  sproto-derive/            -- 派生宏 crate (feature = "derive")
    Cargo.toml
    src/
      lib.rs, attr.rs, encode.rs, decode.rs
  sproto-lua/               -- Lua FFI 绑定 crate
    Cargo.toml
    src/
  tests/                    -- 集成测试
    binary_schema_tests.rs  -- 二进制模式加载测试
    compatibility_tests.rs  -- 前向/后向兼容性测试
    decode_tests.rs         -- C 生成二进制文件解码测试
    encode_tests.rs         -- 编码并与 C 输出字节对比
    pack_tests.rs           -- pack/unpack 压缩测试
    roundtrip_tests.rs      -- 自洽性往返测试
    rpc_tests.rs            -- RPC 功能测试
    derive_tests.rs         -- 派生宏测试
    testdata/               -- C/Lua 生成的二进制固定文件
      generate.lua          -- 生成固定文件的 Lua 脚本
      *.bin                 -- 各种编码/打包/模式的二进制文件
  benchmark/                -- 基准测试
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
# 构建所有 crate（默认功能：derive + serde）
cargo build

# 仅核心功能
cargo build --no-default-features

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
members = ["sproto-derive", "sproto-lua"]
```

- `sproto-derive`: proc-macro crate，提供 `#[derive(SprotoEncode, SprotoDecode)]`
- `sproto-lua`: Lua FFI 绑定，将 Rust 实现暴露为 Lua 可加载模块

## 运行测试

```bash
# 运行所有测试（默认功能）
cargo test --tests

# 运行特定测试文件
cargo test --test decode_tests
cargo test --test roundtrip_tests
cargo test --test rpc_tests

# 运行特定测试函数
cargo test test_decode_person

# 仅运行核心功能的测试（不含 serde/derive）
cargo test --no-default-features

# 仅运行 serde 功能的测试
cargo test --features serde --no-default-features

# 显示测试输出
cargo test -- --nocapture
```

### 测试分类

| 测试类别 | 文件 | 数量 | 说明 |
|---------|------|------|------|
| 单元测试 | src/ 各模块内 | 37 | 组件级测试（词法分析、解析器、线格式读写、pack/unpack、RPC header 等） |
| 二进制模式 | binary_schema_tests.rs | 3 | 加载 C 生成的 .bin 模式 |
| 解码测试 | decode_tests.rs | 14 | 解码 C 和 Go 生成的二进制数据并验证字段值 |
| 编码测试 | encode_tests.rs | 14 | 编码并与 C/Go 输出字节比对 |
| 压缩测试 | pack_tests.rs | 17 | pack/unpack 交叉验证 |
| 往返测试 | roundtrip_tests.rs | 48 | 自洽性测试（serde encode/decode、pack/unpack、derive 往返、完整管线） |
| RPC 测试 | rpc_tests.rs | 15 | RPC 功能（dispatch、session、协议配置、错误处理） |
| 派生宏测试 | derive_tests.rs | 5 | 派生宏编解码功能 |
| 兼容性测试 | compatibility_tests.rs | 13 | 前向/后向兼容性（serde + derive） |
| **合计** | | **~166** | 全部通过 |

### 测试策略

**交叉验证测试**: 通过 `tests/testdata/generate.lua` 使用 C/Lua sproto 库生成二进制固定文件（.bin），Rust 端解码这些文件并验证字段值，或编码后与 C 输出字节精确对比，确保线格式兼容。

**自洽往返测试**: 不依赖外部实现，验证 `decode(encode(value)) == value`、`unpack(pack(data)) == data`、完整管线 `encode -> pack -> unpack -> decode`、serde/derive 往返等。

**测试覆盖要点**: 测试不仅验证字段值，还验证：
- 嵌套结构体的正确编解码
- 字段数量检查
- 边界值、Unicode 字符串、空数据等边界情况
- 前向/后向兼容性（V1 编码 V2 解码、V2 编码 V1 解码）

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
cargo bench

# 运行特定基准
cargo bench -- encode
cargo bench -- decode
cargo bench -- pack
```

基准测试文件位于 `benchmark/sproto_bench.rs`，覆盖：
- encode / decode（Serde API 和 Derive API）
- pack / unpack
- 不同数据复杂度的消息（简单 Person、复杂 UserProfile、大数组 DataSet）

### 跨语言基准测试

`benchmark/benchmark.rs` 是一个 cargo example，支持与 Go 实现对比：

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
bash benchmark/benchall.sh 1000000

# 仅运行 Rust 部分
bash benchmark/benchall.sh
```

脚本输出格式化的对比表格，包含 Rust Serde API 与 Go (reflect/codec) 的跨语言对比。

## Feature 开关开发

开发时注意 feature 开关的影响：

```toml
[features]
default = ["derive"]
serde = ["dep:serde"]
derive = ["dep:sproto-derive", "serde"]
```

- `derive` 依赖 `serde`，启用 `derive` 会自动启用 `serde`
- `serde` 模块在 `src/serde/` 下，受 `#[cfg(feature = "serde")]` 守护
- 核心功能（codec::wire、pack、parser、binary_schema、rpc）不依赖任何 feature

验证各 feature 组合：

```bash
cargo test --no-default-features          # 核心功能
cargo test --no-default-features --features serde   # 核心 + serde
cargo test                                 # 完整功能（默认 derive）
```

## 代码检查

```bash
# 运行 clippy lint
cargo clippy

# 格式化检查
cargo fmt --check

# 格式化
cargo fmt
```

## 依赖管理

### 运行时依赖
- `thiserror`: 错误类型派生
- `serde` (可选): serde 框架
- `sproto-derive` (可选): 派生宏 proc-macro

### 开发依赖
- `pretty_assertions`: 可读性更好的测试失败输出
- `serde` (with "derive"): 用于集成测试
- `serde_bytes`: 用于测试中的 binary 字段序列化
- `criterion`: 基准测试框架

### sproto-derive 依赖
- `syn`: Rust 语法解析
- `quote`: 代码生成
- `proc-macro2`: proc-macro 工具

## 常见开发任务

### 添加新的 sproto 类型支持

1. 在 `src/types.rs` 的 `FieldType` 枚举中添加新变体
2. 更新 serde 模块中 `ser.rs` 和 `de.rs` 的序列化/反序列化逻辑
3. 更新 sproto-derive 中的编解码生成逻辑
4. 添加对应测试

### 修改线格式

1. 修改 `src/codec/wire.rs` 中的底层读写操作
2. 更新 serde ser.rs 和 de.rs
3. 重新生成 testdata 固定文件（`cd tests/testdata && bash build.sh`）
4. 运行全量测试确保兼容性

### 添加新的 RPC 功能

1. 修改 `src/rpc/mod.rs`
2. 在 `tests/rpc_tests.rs` 中添加测试
3. 如果涉及新的包头字段，可能需要更新 tests/testdata 中的 RPC 固定文件
