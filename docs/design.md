# sproto-rust 整体设计

## 概述

sproto-rust 是 [sproto](https://github.com/cloudwu/sproto) 二进制序列化协议的纯 Rust 实现，与 C/Lua 参考实现保持线格式兼容。sproto 是一种紧凑的、模式驱动的序列化格式，设计目标是简洁高效，类似 Protocol Buffers 但特性集更精简。

## 核心模块架构

```
src/
  lib.rs                  -- 公共 API 导出
  error.rs                -- 错误类型 (ParseError, EncodeError, DecodeError, PackError, RpcError, SerdeError)
  types.rs                -- 模式元数据: Sproto, SprotoType, Field, Protocol, FieldType
  schema_traits.rs        -- SchemaEncode / SchemaDecode trait 定义 (Direct API)
  derive_traits.rs        -- SprotoEncode / SprotoDecode trait 定义
  codec/
    mod.rs                -- Direct API 入口: to_bytes(), from_bytes()
    wire.rs               -- 小端读写原语、常量定义
    encoder.rs            -- StructEncoder: tag-based 结构体编码引擎
    decoder.rs            -- StructDecoder: tag-based 结构体解码引擎
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
    encode.rs             -- SprotoEncode + SchemaEncode 派生实现
    decode.rs             -- SprotoDecode + SchemaDecode 派生实现

sproto-lua/               -- Lua FFI 绑定 crate
  src/
    lib.rs                -- Lua 模块入口
    lua_codec.rs          -- LuaTable <-> wire bytes (基于 StructEncoder/StructDecoder)
    userdata.rs           -- Lua userdata 封装 (SprotoUserData, HostUserData 等)
    error.rs              -- 错误转换
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

## 编解码引擎 (Codec Engine)

`codec/encoder.rs` 和 `codec/decoder.rs` 实现了核心编解码引擎，是 Direct API、Serde 适配器和 Lua 绑定的共享基础设施。

### StructEncoder

`StructEncoder` 是 tag-based 的结构体编码器。工作流程：

1. **创建**: `StructEncoder::new(sproto, sproto_type, output)` 在 output buffer 中预留 header 空间
2. **写入字段**: 通过类型化方法写入字段值：
   - `set_integer(tag, value)` / `set_bool(tag, value)` / `set_double(tag, value)`
   - `set_string(tag, value)` / `set_bytes(tag, value)`
   - `encode_nested(tag, closure)` — 嵌套结构体，closure 接收子编码器
   - `set_integer_array(tag, values)` / `set_bool_array` / `set_double_array` / `set_string_array` / `set_bytes_array`
   - `encode_struct_array(tag, closure)` — 结构体数组
3. **完成**: `finish()` 组装 header + data section，执行 compact（移除未使用的 header 槽位）

内部优化：
- 小结构体（<= 32 字段）使用栈上 `[Option<FieldEntry>; 32]` 数组，避免堆分配
- 跟踪 data section 写入顺序，若顺序一致（常见情况）使用 `copy_within` 原地 compact；否则回退到重排模式

### StructDecoder

`StructDecoder` 是惰性的 header 迭代器。工作流程：

1. **创建**: `StructDecoder::new(sproto, sproto_type, data)` 解析 header 校验长度
2. **迭代**: `next_field()` 依次返回 `DecodedField`，自动处理 skip gap 和 data offset 推进
3. **读取值**: `DecodedField` 提供类型化访问器：
   - `as_integer()` / `as_bool()` / `as_double()` / `as_string()` / `as_bytes()`
   - `as_struct()` — 返回嵌套子解码器
   - `as_integer_array()` / `as_bool_array()` / `as_double_array()` / `as_string_array()` / `as_bytes_array()`
   - `as_struct_iter()` — 返回结构体数组迭代器

### 分层架构

```
            编码                              解码
  ┌───────────────────┐             ┌───────────────────┐
  │   Derive Macros    │             │   Derive Macros    │
  │  (编译时内联生成)   │             │  (编译时内联生成)   │
  └───────────────────┘             └───────────────────┘
  ┌───────────────────┐             ┌───────────────────┐
  │   Direct API       │             │   Direct API       │
  │  SchemaEncode trait│─────┐       │  SchemaDecode trait│─────┐
  └───────────────────┘     │       └───────────────────┘     │
  ┌───────────────────┐     │       ┌───────────────────┐     │
  │   Serde Adapter    │     │       │   Serde Adapter    │     │
  │  SerdeEncoderState │─────┤       │  WireMapAccess     │─────┤
  └───────────────────┘     │       └───────────────────┘     │
  ┌───────────────────┐     │       ┌───────────────────┐     │
  │   Lua Binding      │     │       │   Lua Binding      │     │
  │  lua_fill_encoder  │─────┤       │  lua_decode_fields │─────┤
  └───────────────────┘     │       └───────────────────┘     │
                            ▼                                 ▼
                   ┌────────────────┐              ┌─────────────────┐
                   │ StructEncoder  │              │ StructDecoder   │
                   │ (wire format)  │              │ (wire format)   │
                   └────────────────┘              └─────────────────┘
```

四种上层适配（Derive、Direct、Serde、Lua）共享同一套 wire format 引擎，消除了代码重复。Derive 宏是唯一在编译时直接生成 wire 操作的路径，其余三种均在运行时通过 StructEncoder/StructDecoder 操作。

## Direct API

Direct API 提供 3 参数的显式类型编解码，是 StructEncoder/StructDecoder 的高层封装：

```rust
// 编码: sproto::to_bytes(&schema, sproto_type, &value)
// 解码: sproto::from_bytes::<T>(&schema, sproto_type, &data)
```

用户类型通过实现 `SchemaEncode`/`SchemaDecode` trait 与引擎对接。`#[derive(SprotoEncode)]` 同时生成 `SprotoEncode`（无模式编码）和 `SchemaEncode`（模式驱动编码）两个 trait 实现。

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

serde 模块是 StructEncoder/StructDecoder 的薄包装，处理 serde trait 的名称-based 字段派发与 tag-based 引擎之间的桥接：

- `to_bytes()`: Rust struct -> wire bytes（`SerdeEncoderState` 包装 `StructEncoder`，按字段名查找 tag 后委托写入）
- `from_bytes()`: wire bytes -> Rust struct（`WireMapAccess` 包装 `StructDecoder`，以 `DecodedField` 驱动 serde 的 MapAccess 接口）

这跳过了任何中间表示，直接在 Rust 结构体字段和 sproto 线格式之间转换，消除了中间数据结构的分配和转换开销。

与 Go 版本 (gosproto) 的架构对比：Go 版本所有编解码都是结构体直达线格式（reflect 方式通过运行时反射 + struct tag，codec 方式通过手写编解码方法）。Rust 的 Direct API 在架构上与 Go codec 方式对应（trait 方法 vs 手写方法），Serde 路径与 Go reflect 方式对应。

关键设计决策：
- 使用高阶 trait bounds (`for<'de> Deserialize<'de>`) 处理拥有所有权的数据
- Rust 结构体字段名必须与 sproto 模式中的字段名匹配
- `Option<T>` 映射为可选字段，`None` 时省略该字段

## 派生宏 (Derive Macros)

`sproto-derive` crate 提供编译时代码生成：

- `#[derive(SprotoEncode)]`: 生成 `SprotoEncode`（自包含编码）和 `SchemaEncode`（Direct API 编码）两个 trait
- `#[derive(SprotoDecode)]`: 生成 `SprotoDecode`（自包含解码）和 `SchemaDecode`（Direct API 解码）两个 trait

关键特性：
- **内联模式生成**: 宏在编译时生成模式元数据，消除运行时模式查找
- **编译时类型检测**: Rust 类型在宏展开时映射到 sproto `FieldType`
- **支持 `Option<T>`**: 可选字段自动处理
- **支持 `Vec<T>`**: 基本类型数组支持
- **非连续 tag**: 间隔标记在编译时计算
- **属性语法**: `#[sproto(tag = N)]` 指定线上 tag，`#[sproto(skip)]` 排除字段

派生宏不依赖外部模式文件，适用于 Rust 端自定义的协议结构体。

## Lua 绑定 (sproto-lua)

`sproto-lua` crate 将 Rust 实现暴露为 Lua C 模块（cdylib）。`lua_codec.rs` 是 StructEncoder/StructDecoder 的 Lua 适配层，仅处理 `LuaValue <-> Rust` 类型转换，wire format 操作完全委托给共享引擎。

- **编码**: `lua_fill_encoder()` 遍历 schema 字段，从 Lua table 取值，调用 `set_integer/set_string/encode_nested` 等方法
- **解码**: `lua_decode_fields()` 迭代 `StructDecoder::next_field()`，将 `DecodedField` 的 typed accessor 结果转为 `LuaValue` 填入 Lua table

## 错误处理

采用 `thiserror` 实现层次化错误类型：

- `ParseError`: 模式文本解析错误（语法错误、重复 tag/字段、未定义类型引用等）
- `EncodeError`: 编码错误（类型不匹配、未知类型/tag）
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
