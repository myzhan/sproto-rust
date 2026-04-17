# sproto-rust 使用指南

## 安装

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
sproto = "0.1"
```

默认同时启用 `serde` 和 `derive` 功能。如只需核心功能：

```toml
[dependencies]
sproto = { version = "0.1", default-features = false }
```

可选功能组合：

```toml
# 仅 serde 集成（无派生宏）
sproto = { version = "0.1", default-features = false, features = ["serde"] }

# 完整功能（默认，等价于 features = ["derive"]）
sproto = "0.1"
```

## 模式语法 (Schema Syntax)

sproto 使用文本模式定义数据结构和 RPC 协议：

```sproto
# 类型定义
.Person {
    name 0 : string
    age 1 : integer
    email 2 : string
    active 3 : boolean
    score 4 : double
    data 5 : binary

    # 嵌套类型
    .Address {
        city 0 : string
        zip 1 : integer
    }

    address 6 : Address
    friends 7 : *Person        # Person 数组
    tags 8 : *string           # 字符串数组
}

# RPC 协议定义
login 1 {
    request LoginRequest
    response LoginResponse
}

ping 2 {
    response PingResponse      # 无请求体
}

logout 3 {
    response nil               # 确认式响应，无响应数据
}

notify 4 {
    # 单向，无响应
}
```

每个字段格式为：`字段名 tag : 类型`，tag 为 u16 整数，在同一类型内必须唯一。`*` 前缀表示数组类型。

## 方式一：Serde 集成

Serde 集成使用标准 `#[derive(Serialize, Deserialize)]`，需要运行时模式（schema）驱动编解码。字段名必须与模式中定义的字段名一致。

### 基本使用

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Person {
    name: String,
    age: i64,
}

let sproto = sproto::parser::parse(r#"
    .Person { name 0 : string  age 1 : integer }
"#).unwrap();
let person_type = sproto.get_type("Person").unwrap();

let person = Person { name: "Alice".into(), age: 30 };

// 序列化为 sproto 二进制
let bytes = sproto::serde::to_bytes(&sproto, person_type, &person).unwrap();

// 从 sproto 二进制反序列化
let decoded: Person = sproto::serde::from_bytes(&sproto, person_type, &bytes).unwrap();
assert_eq!(person, decoded);
```

### 可选字段

```rust
#[derive(Serialize, Deserialize)]
struct UserProfile {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<String>,  // 可选字段，None 时不编码
    #[serde(skip_serializing_if = "Option::is_none")]
    age: Option<i64>,
}
```

## 方式二：派生宏 (Derive Macros)

派生宏在编译时生成编解码逻辑，无需外部模式文件。每个字段通过 `#[sproto(tag = N)]` 指定 tag。

### 基本使用

```rust
use sproto::{SprotoEncode, SprotoDecode};

#[derive(Debug, PartialEq, SprotoEncode, SprotoDecode)]
struct Person {
    #[sproto(tag = 0)]
    name: String,
    #[sproto(tag = 1)]
    age: i64,
    #[sproto(tag = 2)]
    active: bool,
}

let person = Person {
    name: "Alice".into(),
    age: 30,
    active: true,
};

// 编码（不需要外部模式）
let bytes = person.sproto_encode().unwrap();

// 解码（不需要外部模式）
let decoded = Person::sproto_decode(&bytes).unwrap();
assert_eq!(person, decoded);
```

### 可选字段和数组

```rust
#[derive(SprotoEncode, SprotoDecode)]
struct Message {
    #[sproto(tag = 0)]
    content: String,
    #[sproto(tag = 1)]
    priority: Option<i64>,       // 可选字段
    #[sproto(tag = 2)]
    recipients: Vec<String>,     // 数组字段
}
```

### 非连续 tag

```rust
#[derive(SprotoEncode, SprotoDecode)]
struct Sparse {
    #[sproto(tag = 0)]
    first: String,
    #[sproto(tag = 5)]       // tag 可以不连续
    middle: i64,
    #[sproto(tag = 10)]
    last: bool,
}
```

## 方式三：Direct API

Direct API 使用 `StructEncoder`/`StructDecoder` 进行 tag-based 的模式驱动编解码。调用者提供 `schema` 和 `sproto_type`，保持完全的运行时灵活性。

### 配合 Derive 使用

`#[derive(SprotoEncode, SprotoDecode)]` 同时生成 `SchemaEncode`/`SchemaDecode` trait 实现，可直接用于 Direct API：

```rust
use sproto::{SprotoEncode, SprotoDecode};

#[derive(Debug, PartialEq, SprotoEncode, SprotoDecode)]
struct Person {
    #[sproto(tag = 0)]
    name: String,
    #[sproto(tag = 1)]
    age: i64,
}

let schema = sproto::parser::parse(r#"
    .Person { name 0 : string  age 1 : integer }
"#).unwrap();
let person_type = schema.get_type("Person").unwrap();

let person = Person { name: "Alice".into(), age: 30 };

// 编码: 3 参数 (schema, type, value)
let bytes = sproto::to_bytes(&schema, person_type, &person).unwrap();

// 解码: 3 参数 (schema, type, data)
let decoded: Person = sproto::from_bytes(&schema, person_type, &bytes).unwrap();
assert_eq!(person, decoded);
```

### 直接使用 StructEncoder/StructDecoder

不需要 derive 宏，可直接操作底层编解码器：

```rust
use sproto::codec::{StructEncoder, StructDecoder};

let schema = sproto::parser::parse(r#"
    .Person { name 0 : string  age 1 : integer  active 2 : boolean }
"#).unwrap();
let st = schema.get_type("Person").unwrap();

// 编码
let mut buf = Vec::new();
let mut enc = StructEncoder::new(&schema, st, &mut buf);
enc.set_string(0, "Alice").unwrap();
enc.set_integer(1, 30).unwrap();
enc.set_bool(2, true).unwrap();
enc.finish();

// 解码
let mut dec = StructDecoder::new(&schema, st, &buf).unwrap();
while let Some(field) = dec.next_field().unwrap() {
    match field.tag() {
        0 => println!("name = {}", field.as_string().unwrap()),
        1 => println!("age = {}", field.as_integer().unwrap()),
        2 => println!("active = {}", field.as_bool().unwrap()),
        _ => {}
    }
}
```

### 嵌套结构体和数组

```rust
use sproto::codec::StructEncoder;

let schema = sproto::parser::parse(r#"
    .Person { name 0 : string  age 1 : integer }
    .Team { name 0 : string  members 1 : *Person }
"#).unwrap();
let st = schema.get_type("Team").unwrap();

let mut buf = Vec::new();
let mut enc = StructEncoder::new(&schema, st, &mut buf);
enc.set_string(0, "TeamA").unwrap();
enc.encode_struct_array(1, |arr| {
    arr.encode_element(|e| {
        e.set_string(0, "Alice")?;
        e.set_integer(1, 30)?;
        Ok(())
    })?;
    arr.encode_element(|e| {
        e.set_string(0, "Bob")?;
        e.set_integer(1, 25)?;
        Ok(())
    })?;
    Ok(())
}).unwrap();
enc.finish();
```

## Pack/Unpack 压缩

sproto 提供零压缩算法，用于减少传输数据量：

```rust
use sproto::pack;

// 对已编码的数据进行压缩（用于传输）
let packed = pack::pack(&encoded);

// 解压（收到后）
let unpacked = pack::unpack(&packed).unwrap();
assert_eq!(encoded, unpacked);
```

完整管线示例（Serde 编码 + 压缩 + 解压 + 解码）：

```rust
use serde::{Serialize, Deserialize};
use sproto::pack;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Person {
    name: String,
    age: i64,
}

let sproto = sproto::parser::parse(r#"
    .Person { name 0 : string  age 1 : integer }
"#).unwrap();
let person_type = sproto.get_type("Person").unwrap();

// 发送端: 编码 + 压缩
let person = Person { name: "Alice".into(), age: 30 };
let encoded = sproto::serde::to_bytes(&sproto, person_type, &person).unwrap();
let packed = pack::pack(&encoded);
// ... 发送 packed 数据 ...

// 接收端: 解压 + 解码
let unpacked = pack::unpack(&packed).unwrap();
let decoded: Person = sproto::serde::from_bytes(&sproto, person_type, &unpacked).unwrap();
assert_eq!(person, decoded);
```

## 二进制模式加载

可加载由 C/Lua 工具链 (`sprotodump`) 预编译的二进制模式：

```rust
use sproto::binary_schema;

let schema_bytes = std::fs::read("schema.bin").unwrap();
let sproto = binary_schema::load_binary(&schema_bytes).unwrap();

// 之后正常使用
let person_type = sproto.get_type("Person").unwrap();
```

## RPC 使用

RPC 层操作原始字节，调用者负责使用 serde 或 derive 对消息体进行编解码。

### 创建 Host 和处理请求

```rust
use serde::{Serialize, Deserialize};
use sproto::rpc::{Host, DispatchResult};

#[derive(Serialize)]
struct LoginRequest { username: String, password: String }

#[derive(Deserialize)]
struct LoginResponse {
    #[serde(default)]
    ok: Option<bool>,
    #[serde(default)]
    message: Option<String>,
}

// 加载 RPC 模式
let sproto = sproto::binary_schema::load_binary(&schema_bytes).unwrap();

// 创建 Host
let mut host = Host::new(sproto.clone());

// 分发接收到的数据
match host.dispatch(&packed_data).unwrap() {
    DispatchResult::Request { name, body, responder, ud } => {
        println!("收到请求: {}", name);

        // 用 serde 解码请求体
        let req_type = sproto.get_type("login_request").unwrap();
        // let req: LoginRequest = sproto::serde::from_bytes(&sproto, req_type, &body).unwrap();

        // 处理请求并发送响应
        if let Some(resp) = responder {
            let resp_type = sproto.get_type("login_response").unwrap();
            let response_body = sproto::serde::to_bytes(&sproto, resp_type, &LoginResponse {
                ok: Some(true), message: Some("Welcome!".into()),
            }).unwrap();
            let response_packed = resp.respond(&response_body, None).unwrap();
            // ... 发送 response_packed ...
        }
    }
    DispatchResult::Response { session, body, ud } => {
        println!("收到响应, session: {}", session);
        // 用 serde 解码响应体
    }
}
```

### 发送请求

```rust
// 创建 RequestSender，attach 远端模式
let mut sender = host.attach(remote_sproto.clone());

// 编码请求体
let req_type = remote_sproto.get_type("login_request").unwrap();
let body = sproto::serde::to_bytes(&remote_sproto, req_type, &LoginRequest {
    username: "alice".into(), password: "secret".into(),
}).unwrap();

// 发送请求（body 为已编码的字节）
let packed = sender.request("login", &body, Some(1), None).unwrap();

// 注册 session 以跟踪响应
host.register_session(1);
```

## 三种 API 对比

| 特性 | Direct API | Serde | Derive |
|------|-----------|-------|--------|
| 需要模式文件 | 是 | 是 | 否 |
| 需要定义结构体 | 可选（可直接操作 tag） | 是 | 是 |
| 编译时类型安全 | 是（配合 derive） | 是 | 是 |
| 性能 | 最高（tag-based 直接操作） | 高（名称查找开销） | 高（编译时生成） |
| 适用场景 | 需要最大灵活性或最高性能 | 已有 serde 生态的项目 | Rust 端自定义协议 |

### 与 Go 版本的对应关系

Go 版本 (gosproto) 所有 API 都是结构体直达线格式：

- **Go reflect 方式** (`sproto.Encode()/Decode()`): 通过 Go 的 reflect 包 + struct tag 注解在运行时反射编解码，类似 Rust 的 Serde 方式
- **Go codec 方式** (手写 `MarshalSproto()/UnmarshalSproto()`): 每个结构体手动实现编解码方法，类似 Rust 的 Direct API（trait 方法 vs 手写方法）

Rust 的 Direct API (`sproto::to_bytes`/`sproto::from_bytes`) 在架构上与 Go codec 方式对应。Serde 路径与 Go reflect 方式对应。Derive 宏在编译时生成内联编解码逻辑，无需运行时模式。

## 类型映射速查表

| Sproto 类型 | Rust 类型 |
|-------------|-----------|
| `integer` | `i64` (及 i8/i16/i32/u8/u16/u32/u64) |
| `boolean` | `bool` |
| `string` | `String` / `&str` |
| `binary` | `Vec<u8>` |
| `double` | `f64` (及 f32) |
| `*type` | `Vec<T>` |
| `.Type` | 结构体 |
