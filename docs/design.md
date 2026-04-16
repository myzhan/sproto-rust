# sproto-rust 整体设计

## 概述

sproto-rust 是 [sproto](https://github.com/cloudwu/sproto) 二进制序列化协议的纯 Rust 实现，与 C/Lua 参考实现保持线格式兼容。sproto 是一种紧凑的、模式驱动的序列化格式，设计目标是简洁高效，类似 Protocol Buffers 但特性集更精简。

## 核心模块架构

```
src/
  lib.rs                  -- 公共 API 导出
  error.rs                -- 错误类型 (ParseError, DecodeError, PackError, RpcError, SerdeError)
  types.rs                -- 模式元数据: Sproto, SprotoType, Field, Protocol, FieldType
  derive_traits.rs        -- SprotoEncode / SprotoDecode trait 定义
  codec/
    mod.rs                -- codec 模块入口
    wire.rs               -- 小端读写原语、常量定义
  pack.rs                 -- pack() / unpack() 零压缩
  parser/
    mod.rs                -- parse(text) -> Sproto
    lexer.rs              -- 词法分析器
    ast.rs                -- AST 节点类型
    grammar.rs            -- 递归下降解析器
    schema_builder.rs     -- AST -> Sproto 转换（类型解析、验证）
  binary_schema.rs        -- load_binary(bytes) -> Sproto
  rpc/
    mod.rs                -- Host, RequestSender, Responder, DispatchResult
  serde/                  -- (feature = "serde")
    mod.rs                -- to_bytes(), from_bytes()
    ser.rs                -- Rust struct -> wire bytes 的 Serializer
    de.rs                 -- wire bytes -> Rust struct 的 Deserializer
    error.rs              -- SerdeError 类型

sproto-derive/            -- Proc-macro 派生宏 crate (feature = "derive")
  src/
    lib.rs                -- 宏入口
    attr.rs               -- 属性解析 (#[sproto(tag = N)])
    encode.rs             -- SprotoEncode 派生实现
    decode.rs             -- SprotoDecode 派生实现
```

## 线格式 (Wire Protocol)

sproto 采用基于 tag 的二进制格式。每个编码后的结构体包含：

1. **头部 (Header)**: `u16` 字段数量，后跟相应数量的 `u16` 字段描述符
2. **数据部分 (Data Part)**: 无法内联的字段的变长数据

字段描述符编码规则：
- **偶数值 `v`**: 内联值 = `v / 2 - 1`。用于小整数 (0..0x7ffe) 和布尔值
- **奇数值 `v`**: 跳过标记 = `(v - 1) / 2` 个 tag。编码非连续 tag 之间的间隔
- **零值**: 字段数据在数据部分（以 `u32` 长度前缀）

所有整数均为小端序。整数字段根据值范围自动选择 32 位或 64 位编码。数组在整数元素前包含一个大小标记字节（4 或 8）。

## 零压缩 (Zero-Packing)

pack 算法按 8 字节块处理数据：

- 对每个 8 字节块，计算一个 **tag 字节**，其中第 `i` 位表示第 `i` 个字节是否非零
- 输出：tag 字节 + 仅非零字节
- **0xFF 优化**: 当一个块的全部 8 字节都非零时，启动原始批次。连续的全非零块（以及在已有批次中有 6-7 个非零字节的块）被写为：`0xFF | count-1 | raw_bytes...`。最大批次：256 个块

这种压缩对于 sproto 典型输出（头部和长度前缀中包含大量零字节）效果良好。

## 模式解析器 (Schema Parser)

手写的递归下降解析器（无外部解析器组合子依赖），分三个阶段：

1. **词法分析 (`lexer.rs`)**: 将模式文本分词为 `Token` 变体（Name, Number, 标点符号），跟踪行号用于错误报告，跳过 `#` 注释

2. **语法分析 (`grammar.rs`)**: 将 token 流解析为 AST。处理类型定义 (`.TypeName { fields... }`)、协议定义 (`ProtocolName tag { request/response }`)、嵌套类型、数组字段 (`*type`) 以及 map/decimal 扩展

3. **模式构建 (`schema_builder.rs`)**: 三遍解析：
   - 收集所有类型和协议，展平嵌套类型为点分隔名称（如 `Person.PhoneNumber`）
   - 按字母序排列类型名称（与 Lua 参考实现一致，保证二进制兼容）
   - 解析类型引用，验证 tag/名称唯一性，计算优化查找结构

## 字段查找优化

`SprotoType` 存储按 tag 排序的字段，并计算 `base_tag` 和 `maxn`：

- 若 tag 从 `base_tag` 开始连续，字段查找为 O(1) 直接索引：`fields[tag - base_tag]`
- 否则回退到排序字段数组的二分查找

按名称查找使用分支策略：
- 字段数 <= 8 时，使用线性扫描（对小结构体更快）
- 字段数 > 8 时，使用 `HashMap<Rc<str>, usize>` O(1) 查找

这与 C 参考实现的优化策略一致。

## 二进制模式加载 (Binary Schema Loading)

`binary_schema.rs` 实现了引导解码器，无需"模式的模式"即可加载预编译的二进制模式。它硬编码了元模式结构（`.type`、`.field`、`.protocol` 定义）的知识，将二进制分组消息解码为 `Sproto` 元数据。

这实现了与 C/Lua 工具链的互操作：由 `sprotodump` 编译的模式可以直接加载。

## RPC 层

RPC 模块实现 sproto RPC 模式，操作原始字节（不绑定特定编解码方式）：

- **`Host`**: RPC 端点。`dispatch(packed_data)` 解码传入的数据包头，返回 `Request`（附带原始 body 字节和 `Responder`）或 `Response`（通过 session ID 匹配先前的出站请求，附带原始 body 字节）。调用者负责使用 serde 或 derive 对 body 进行编解码。

- **`RequestSender`**: 通过 `Host::attach(remote_sproto)` 创建。接受已编码的 body 字节，构建包含协议 tag 的请求包。

- **`Responder`**: 接受已编码的 body 字节，构建包含 session ID 的响应包。

包头使用 `codec::wire` 原语直接编码，为固定三字段整数结构：`type`（tag=0）、`session`（tag=1）、`ud`（tag=2）。

## Serde 集成

serde 模块提供 Rust 结构体与 sproto 线格式之间的直接序列化/反序列化：

- `to_bytes()`: Rust struct -> wire bytes（通过 `ser.rs`，按 sproto 线格式直接编码）
- `from_bytes()`: wire bytes -> Rust struct（通过 `de.rs`，从线格式直接解码）

这跳过了任何中间表示，直接在 Rust 结构体字段和 sproto 线格式之间转换，消除了中间数据结构的分配和转换开销。

与 Go 版本 (gosproto) 的架构对比：Go 版本所有编解码都是结构体直达线格式（reflect 方式通过运行时反射 + struct tag，codec 方式通过手写编解码方法）。Rust 的 Serde 路径在架构上与 Go reflect 方式对应。

关键设计决策：
- 使用高阶 trait bounds (`for<'de> Deserialize<'de>`) 处理拥有所有权的数据
- Rust 结构体字段名必须与 sproto 模式中的字段名匹配
- `Option<T>` 映射为可选字段，`None` 时省略该字段

## 派生宏 (Derive Macros)

`sproto-derive` crate 提供编译时代码生成：

- `#[derive(SprotoEncode)]`: 在编译时生成内联模式元数据和编码逻辑
- `#[derive(SprotoDecode)]`: 在编译时生成内联模式元数据和解码逻辑

关键特性：
- **内联模式生成**: 宏在编译时生成模式元数据，消除运行时模式查找
- **编译时类型检测**: Rust 类型在宏展开时映射到 sproto `FieldType`
- **支持 `Option<T>`**: 可选字段自动处理
- **支持 `Vec<T>`**: 基本类型数组支持
- **非连续 tag**: 间隔标记在编译时计算
- **属性语法**: `#[sproto(tag = N)]` 指定线上 tag，`#[sproto(skip)]` 排除字段

派生宏不依赖外部模式文件，适用于 Rust 端自定义的协议结构体。

## 错误处理

采用 `thiserror` 实现层次化错误类型：

- `ParseError`: 模式文本解析错误（语法错误、重复 tag/字段、未定义类型引用等）
- `DecodeError`: 解码错误（数据截断、无效数据、UTF-8 错误）
- `PackError`: 压缩/解压错误
- `RpcError`: RPC 层错误（封装了解码/压缩错误）
- `SprotoError`: 顶层错误，统一封装上述所有子错误
- `SerdeError`: Serde 集成层特有的错误（类型不匹配、缺失字段等）

## Feature 开关

```toml
[features]
default = ["derive"]
serde = ["dep:serde"]
derive = ["dep:sproto-derive", "serde"]
```

- **`serde`**: 启用模式驱动的 serde 集成
- **`derive`**: 启用派生宏，自动包含 `serde`
- 默认开启 `derive`，提供完整功能
- 可通过 `default-features = false` 仅使用核心功能（pack/unpack、模式解析）

## 类型映射

| Sproto 类型 | Rust 类型 | 线格式 |
|-------------|-----------|--------|
| `integer` | `i64` | 内联或 4/8 字节 |
| `boolean` | `bool` | 内联 0/1 |
| `string` | `String` | 长度前缀 UTF-8 |
| `binary` | `Vec<u8>` | 长度前缀字节 |
| `double` | `f64` | 8 字节 IEEE 754 |
| `*type` | `Vec<T>` | 元素前缀数组 |
| `.Type` | struct | tag-based 结构 |

## 线格式兼容性

本实现与 [C/Lua 参考实现](https://github.com/cloudwu/sproto) 保持线格式兼容。任一实现编码的二进制数据均可被另一实现解码。测试套件通过 C/Lua 生成的二进制固定文件（tests/testdata/*.bin）验证字节级兼容性。
