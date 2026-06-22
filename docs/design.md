# sproto-rust 整体设计

## 概述

sproto-rust 是 [sproto](https://github.com/cloudwu/sproto) 二进制序列化协议的纯 Rust 实现，与 C/Lua 参考实现保持线格式兼容。sproto 是一种紧凑的、模式驱动的序列化格式，设计目标是简洁高效，类似 Protocol Buffers 但特性集更精简。

## 核心模块架构

```
src/
  lib.rs                  -- 公共 API 导出
  error.rs                -- 错误类型 (EncodeError, DecodeError, PackError, RpcError, SprotoError)
  types.rs                -- 模式元数据: Sproto, SprotoType, Field, Protocol, FieldType (+ Builder API)
  derive_traits.rs        -- SprotoEncode/SprotoDecode trait + to_bytes/from_bytes 便捷函数
  codec/
    mod.rs                -- 编解码模块导出
    wire.rs               -- 小端读写原语、常量定义
    encoder.rs            -- StructEncoder: tag-based 结构体编码引擎
    decoder.rs            -- StructDecoder: tag-based 结构体解码引擎
  pack.rs                 -- pack() / unpack() 零压缩
  binary_schema.rs        -- 二进制模式加载器 (C 工具链兼容)
  rpc/
    mod.rs                -- Host, RequestSender, Responder, DispatchResult

sproto-derive/            -- proc-macro crate（feature "derive" 控制）
  src/
    lib.rs                -- #[proc_macro_derive] 入口
    parse.rs              -- 属性解析: #[sproto(tag, decimal, key, value, key_field)]
    classify.rs           -- syn::Type → FieldKind 分类（Integer/Bool/String/Vec/HashMap/Option/Box/Struct...）
    encode_gen.rs         -- 生成 SprotoEncode impl（按 tag 排序，编码各字段）
    decode_gen.rs         -- 生成 SprotoDecode impl（变量声明、match arm、struct 构建）

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

`codec/encoder.rs` 和 `codec/decoder.rs` 实现了核心编解码引擎，是 Direct API 和 Lua 绑定的共享基础设施。

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
  │   Lua Binding      │             │   Lua Binding      │
  │  lua_fill_encoder  │─────┐       │  lua_decode_fields │─────┐
  └───────────────────┘     │       └───────────────────┘     │
  ┌───────────────────┐     │       ┌───────────────────┐     │
  │   User Code        │     │       │   User Code        │     │
  │  StructEncoder API │─────┤       │  StructDecoder API │─────┤
  └───────────────────┘     │       └───────────────────┘     │
                            ▼                                 ▼
                   ┌────────────────┐              ┌─────────────────┐
                   │ StructEncoder  │              │ StructDecoder   │
                   │ (wire format)  │              │ (wire format)   │
                   └────────────────┘              └─────────────────┘
```

用户代码和 Lua 绑定共享同一套 wire format 引擎，消除了代码重复。

## Derive API (sproto-derive)

`sproto-derive` 是一个 proc-macro crate，通过 `#[derive(SprotoEncode, SprotoDecode)]` 自动为 Rust struct 生成编解码代码。

### 设计原则

1. **运行时 schema**: schema 在调用 `to_bytes`/`from_bytes` 时传入，不在编译时嵌入。这保持了 schema 可热更新的特性。
2. **零开销抽象**: 生成的代码与手写 Direct API 调用结构相同，编码性能差异 <10%。
3. **编译时字段映射**: proc-macro 解析 `#[sproto(tag = N)]` 属性，在编译时建立 field→tag 映射关系。

### 代码生成流程

```
#[derive(SprotoEncode)]         #[derive(SprotoDecode)]
        │                               │
        ▼                               ▼
    parse.rs                        parse.rs
    (解析属性)                      (解析属性)
        │                               │
        ▼                               ▼
   classify.rs                     classify.rs
   (类型分类)                      (类型分类)
        │                               │
        ▼                               ▼
  encode_gen.rs                   decode_gen.rs
  (生成编码代码)                   (生成解码代码)
```

### 类型分类 (classify.rs)

`classify_type()` 将 `syn::Type` 映射到 `FieldKind` 枚举：

- 路径匹配: `String` → StringField, `bool` → Bool, `f64` → Double
- 整数类型: `i64`, `i32`, `i16`, `i8`, `u32`, `u16`, `u8` → Integer
- 容器拆包: `Option<T>` → is_optional=true + 递归分类 T
- Box 拆包: `Box<T>` → is_boxed=true + 递归分类 T
- HashMap 分类: `HashMap<K, V>` + `value` 属性 → AnonymousMap, `HashMap<K, V>` + `key` 属性 → IndexedMap
- Vec 分类: `Vec<u8>` → Binary, `Vec<i64>` → IntegerArray, `Vec<T>` → StructArray(T)
- 其他类型 → NestedStruct(Type)

### 编码生成 (encode_gen.rs)

生成的 `SprotoEncode::sproto_encode()` 实现：

1. 字段按 tag 排序（命中 StructEncoder 的快速路径）
2. 对每个字段生成对应的 `enc.set_*()` 调用
3. `Option<T>` 字段: `if let Some(ref __v) = self.field { ... }`
4. `Vec<T>` 字段: `if !self.field.is_empty() { ... }` 跳过空数组
5. `HashMap<K, V>` 索引 map: 遍历 `.values()` 编码为 struct array（struct 自带 key 字段）
6. `HashMap<K, V>` 匿名 map: 遍历 entries，每对 (k,v) 编码为两字段 struct
7. 嵌套结构体: `enc.encode_nested(tag, |sub| self.field.sproto_encode(sub))`

### 解码生成 (decode_gen.rs)

生成的 `SprotoDecode::sproto_decode()` 实现：

1. 为每个字段声明带默认值的 `let mut` 变量
2. 循环 `dec.next_field()` 并 match tag
3. 每个 tag 调用对应的 `field.as_*()` 方法赋值
4. `HashMap` 索引 map: 解码为 struct 后通过 `item.{key_field}` 提取 key 插入 HashMap
5. `HashMap` 匿名 map: 遍历 struct array 元素，按 key_tag/value_tag 提取原始字段插入 HashMap
6. 构造并返回 struct

### 分层关系

```
  ┌───────────────────┐
  │  Derive API        │
  │  to_bytes/from_bytes│
  └─────────┬─────────┘
            │ 调用
            ▼
  ┌───────────────────┐
  │  Direct API        │
  │ StructEncoder/Decoder│
  └─────────┬─────────┘
            │ 操作
            ▼
  ┌───────────────────┐
  │  Wire Format       │
  │  (codec/wire.rs)   │
  └───────────────────┘
```

Derive API 是 Direct API 的薄封装层，生成的代码直接调用 StructEncoder/StructDecoder 方法。

## Builder API

`types.rs` 提供程序化构建模式的 Builder API：

- `Sproto::new()` — 创建空模式
- `Sproto::add_type(name, fields) -> usize` — 添加类型，返回类型索引
- `Sproto::add_protocol(name, tag, request, response, confirm) -> usize` — 添加协议
- `Field::new(name, tag, field_type)` — 创建标量字段
- `Field::array(name, tag, field_type)` — 创建数组字段
- `Field::decimal(name, tag, precision)` — 创建定点小数字段

嵌套结构体通过 `FieldType::Struct(type_index)` 引用先前添加的类型索引。

## 零压缩 (Zero-Packing)

pack 算法按 8 字节块处理数据：

- 对每个 8 字节块，计算一个 **tag 字节**，其中第 `i` 位表示第 `i` 个字节是否非零
- 输出：tag 字节 + 仅非零字节
- **0xFF 优化**: 当一个块的全部 8 字节都非零时，启动原始批次。连续的全非零块（以及在已有批次中有 6-7 个非零字节的块）被写为：`0xFF | count-1 | raw_bytes...`。最大批次：256 个块

这种压缩对于 sproto 典型输出（头部和长度前缀中包含大量零字节）效果良好。

## 二进制模式加载 (Binary Schema Loading)

`binary_schema.rs` 实现了引导解码器，无需"模式的模式"即可加载预编译的二进制模式。它硬编码了元模式结构（`.type`、`.field`、`.protocol` 定义）的知识，将二进制分组消息解码为 `Sproto` 元数据。

这实现了与 C/Lua 工具链的互操作：由 `sprotodump` 编译的模式可以直接加载。

## 字段查找优化

`SprotoType` 存储按 tag 排序的字段，并计算 `base_tag` 和 `maxn`：

- 若 tag 从 `base_tag` 开始连续，字段查找为 O(1) 直接索引：`fields[tag - base_tag]`
- 否则回退到排序字段数组的二分查找

按名称查找使用分支策略：
- 字段数 <= 8 时，使用线性扫描（对小结构体更快）
- 字段数 > 8 时，使用 `HashMap<Rc<str>, usize>` O(1) 查找

这与 C 参考实现的优化策略一致。

## RPC 层

RPC 模块实现 sproto RPC 模式，操作原始字节（不绑定特定编解码方式）：

- **`Host`**: RPC 端点。`dispatch(packed_data)` 解码传入的数据包头，返回 `Request`（附带原始 body 字节和 `Responder`）或 `Response`（通过 session ID 匹配先前的出站请求，附带原始 body 字节）。调用者负责使用 StructEncoder/StructDecoder 对 body 进行编解码。

- **`RequestSender`**: 通过 `Host::attach(remote_sproto)` 创建。接受已编码的 body 字节，构建包含协议 tag 的请求包。

- **`Responder`**: 接受已编码的 body 字节，构建包含 session ID 的响应包。

包头使用 `codec::wire` 原语直接编码，为固定三字段整数结构：`type`（tag=0）、`session`（tag=1）、`ud`（tag=2）。

## Lua 绑定 (sproto-lua)

`sproto-lua` crate 将 Rust 实现暴露为 Lua C 模块（cdylib）。`lua_codec.rs` 是 StructEncoder/StructDecoder 的 Lua 适配层，仅处理 `LuaValue <-> Rust` 类型转换，wire format 操作完全委托给共享引擎。

- **编码**: `lua_fill_encoder()` 遍历 schema 字段，从 Lua table 取值，调用 `set_integer/set_string/encode_nested` 等方法
- **解码**: `lua_decode_fields()` 迭代 `StructDecoder::next_field()`，将 `DecodedField` 的 typed accessor 结果转为 `LuaValue` 填入 Lua table

## 错误处理

采用 `thiserror` 实现层次化错误类型：

- `EncodeError`: 编码错误（类型不匹配、未知类型/tag）
- `DecodeError`: 解码错误（数据截断、无效数据、UTF-8 错误）
- `PackError`: 压缩/解压错误
- `RpcError`: RPC 层错误（封装了解码/压缩错误）
- `SprotoError`: 顶层错误，统一封装上述所有子错误

## 类型映射

| Sproto 类型 | Rust 类型 | 线格式 |
|-------------|-----------|--------|
| `integer` | `i64` | 内联或 4/8 字节 |
| `boolean` | `bool` | 内联 0/1 |
| `string` | `String` / `&str` | 长度前缀 UTF-8 |
| `binary` | `Vec<u8>` / `&[u8]` | 长度前缀字节 |
| `double` | `f64` | 8 字节 IEEE 754 |
| `*type` | `Vec<T>` | 元素前缀数组 |
| `*type(key)` | `HashMap<K, Struct>` | 元素前缀数组（线上同 `*type`） |
| `*type()` | `HashMap<K, V>` | 元素前缀数组（线上同 `*type`） |
| `.Type` | 嵌套结构体 | tag-based 结构 |

## 线格式兼容性

本实现与 [C/Lua 参考实现](https://github.com/cloudwu/sproto) 保持线格式兼容。任一实现编码的二进制数据均可被另一实现解码。测试套件通过 C/Lua 生成的二进制固定文件（tests/testdata/*.bin）验证字节级兼容性。
