# sproto-rust 使用指南

## 安装

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
sproto = "0.1"
```

无可选 feature，直接使用全部功能。

## 构建模式 (Schema)

sproto 是模式驱动的序列化协议，编解码前需要先构建模式。有两种方式：

### 方式一：Builder API（纯 Rust 编程构建）

```rust
use sproto::types::{Sproto, Field, FieldType};

let mut sproto = Sproto::new();

// 添加类型
sproto.add_type("Person", vec![
    Field::new("name", 0, FieldType::String),
    Field::new("age", 1, FieldType::Integer),
    Field::new("active", 2, FieldType::Boolean),
    Field::new("score", 3, FieldType::Double),
    Field::new("data", 4, FieldType::Binary),
]);
```

嵌套结构体通过 `FieldType::Struct(type_index)` 引用：

```rust
let person_idx = sproto.add_type("Person", vec![
    Field::new("name", 0, FieldType::String),
    Field::new("age", 1, FieldType::Integer),
]);

sproto.add_type("Team", vec![
    Field::new("name", 0, FieldType::String),
    Field::array("members", 1, FieldType::Struct(person_idx)),
]);
```

数组字段用 `Field::array()`，定点小数字段用 `Field::decimal()`：

```rust
sproto.add_type("Data", vec![
    Field::array("tags", 0, FieldType::String),       // 字符串数组
    Field::array("scores", 1, FieldType::Integer),     // 整数数组
    Field::decimal("price", 2, 100),                   // integer(2)，精度 100
]);
```

添加 RPC 协议：

```rust
let req_idx = sproto.add_type("LoginRequest", vec![
    Field::new("username", 0, FieldType::String),
    Field::new("password", 1, FieldType::String),
]);
let resp_idx = sproto.add_type("LoginResponse", vec![
    Field::new("ok", 0, FieldType::Boolean),
]);

// add_protocol(name, tag, request_type_index, response_type_index, confirm)
sproto.add_protocol("login", 1, Some(req_idx), Some(resp_idx), false);
sproto.add_protocol("ping", 2, None, None, false);           // 无请求无响应
sproto.add_protocol("logout", 3, None, None, true);          // confirm 式响应(nil)
```

### 方式二：加载二进制模式（C/Lua 工具链兼容）

加载由 C/Lua 工具链 (`sprotodump`) 预编译的二进制模式：

```rust
use sproto::binary_schema;

let schema_bytes = std::fs::read("schema.bin").unwrap();
let sproto = binary_schema::load_binary(&schema_bytes).unwrap();

// 之后正常使用
let person_type = sproto.get_type("Person").unwrap();
```

## 编码 (Encoding)

使用 `StructEncoder` 进行 tag-based 编码：

```rust
use sproto::codec::StructEncoder;

let st = sproto.get_type("Person").unwrap();

let mut buf = Vec::new();
let mut enc = StructEncoder::new(&sproto, st, &mut buf);
enc.set_string(0, "Alice").unwrap();    // tag 0: name
enc.set_integer(1, 30).unwrap();        // tag 1: age
enc.set_bool(2, true).unwrap();         // tag 2: active
enc.set_double(3, 99.5).unwrap();       // tag 3: score
enc.set_bytes(4, b"raw data").unwrap(); // tag 4: data
enc.finish();
// buf 现在包含编码后的字节
```

### 编码数组

```rust
let mut buf = Vec::new();
let st = sproto.get_type("Data").unwrap();
let mut enc = StructEncoder::new(&sproto, st, &mut buf);
enc.set_string_array(0, &["a", "b", "c"]).unwrap();
enc.set_integer_array(1, &[10, 20, 30]).unwrap();
enc.finish();
```

### 编码嵌套结构体

```rust
let st = sproto.get_type("Team").unwrap();

let mut buf = Vec::new();
let mut enc = StructEncoder::new(&sproto, st, &mut buf);
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

单个嵌套结构体使用 `encode_nested`：

```rust
enc.encode_nested(tag, |nested_enc| {
    nested_enc.set_string(0, "value")?;
    Ok(())
}).unwrap();
```

## 解码 (Decoding)

使用 `StructDecoder` 进行 tag-based 解码：

```rust
use sproto::codec::StructDecoder;

let st = sproto.get_type("Person").unwrap();
let mut dec = StructDecoder::new(&sproto, st, &buf).unwrap();

while let Some(field) = dec.next_field().unwrap() {
    match field.tag() {
        0 => println!("name = {}", field.as_string().unwrap()),
        1 => println!("age = {}", field.as_integer().unwrap()),
        2 => println!("active = {}", field.as_bool().unwrap()),
        3 => println!("score = {}", field.as_double().unwrap()),
        4 => println!("data = {:?}", field.as_bytes()),
        _ => {} // 忽略未知字段
    }
}
```

### 解码数组

```rust
match field.tag() {
    0 => {
        let strings: Vec<&str> = field.as_string_array().unwrap();
    }
    1 => {
        let ints: Vec<i64> = field.as_integer_array().unwrap();
    }
    _ => {}
}
```

### 解码嵌套结构体

单个嵌套结构体：

```rust
let sub_dec = field.as_struct().unwrap();
// 像普通 StructDecoder 一样迭代 sub_dec
```

结构体数组：

```rust
let mut iter = field.as_struct_iter().unwrap();
while let Some(sub_dec) = iter.next().unwrap() {
    // 像普通 StructDecoder 一样迭代 sub_dec
}
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

完整管线示例（编码 + 压缩 + 解压 + 解码）：

```rust
use sproto::codec::{StructEncoder, StructDecoder};
use sproto::pack;

let st = sproto.get_type("Person").unwrap();

// 发送端: 编码 + 压缩
let mut buf = Vec::new();
let mut enc = StructEncoder::new(&sproto, st, &mut buf);
enc.set_string(0, "Alice").unwrap();
enc.set_integer(1, 30).unwrap();
enc.finish();
let packed = pack::pack(&buf);
// ... 发送 packed 数据 ...

// 接收端: 解压 + 解码
let unpacked = pack::unpack(&packed).unwrap();
let mut dec = StructDecoder::new(&sproto, st, &unpacked).unwrap();
while let Some(field) = dec.next_field().unwrap() {
    match field.tag() {
        0 => println!("name = {}", field.as_string().unwrap()),
        1 => println!("age = {}", field.as_integer().unwrap()),
        _ => {}
    }
}
```

## RPC 使用

RPC 层操作原始字节，调用者负责使用 `StructEncoder`/`StructDecoder` 对消息体进行编解码。

### 创建 Host 和处理请求

```rust
use sproto::rpc::{Host, DispatchResult};

// 加载 RPC 模式（需包含协议定义）
let sproto = binary_schema::load_binary(&schema_bytes).unwrap();

// 创建 Host
let mut host = Host::new(sproto.clone());

// 分发接收到的数据
match host.dispatch(&packed_data).unwrap() {
    DispatchResult::Request { name, body, responder, ud } => {
        println!("收到请求: {}", name);
        // 用 StructDecoder 解码请求体 body...

        // 处理请求并发送响应
        if let Some(resp) = responder {
            // 用 StructEncoder 编码响应体
            let mut resp_buf = Vec::new();
            // ... 编码 ...
            let response_packed = resp.respond(&resp_buf, None).unwrap();
            // ... 发送 response_packed ...
        }
    }
    DispatchResult::Response { session, body, ud } => {
        println!("收到响应, session: {}", session);
        // 用 StructDecoder 解码响应体 body...
    }
}
```

### 发送请求

```rust
// 创建 RequestSender，attach 远端模式
let mut sender = host.attach(remote_sproto.clone());

// 用 StructEncoder 编码请求体
let mut body_buf = Vec::new();
// ... 编码 ...

// 发送请求（body 为已编码的字节）
let packed = sender.request("login", &body_buf, Some(1), None).unwrap();

// 注册 session 以跟踪响应
host.register_session(1);
```

## StructEncoder 方法速查

| 方法 | 说明 |
|------|------|
| `set_integer(tag, value)` | 写入 `i64` 整数 |
| `set_bool(tag, value)` | 写入 `bool` 布尔值 |
| `set_double(tag, value)` | 写入 `f64` 浮点数 |
| `set_string(tag, value)` | 写入 `&str` 字符串 |
| `set_bytes(tag, value)` | 写入 `&[u8]` 二进制 |
| `encode_nested(tag, closure)` | 写入嵌套结构体 |
| `set_integer_array(tag, values)` | 写入 `&[i64]` 整数数组 |
| `set_bool_array(tag, values)` | 写入 `&[bool]` 布尔数组 |
| `set_double_array(tag, values)` | 写入 `&[f64]` 浮点数组 |
| `set_string_array(tag, values)` | 写入字符串数组 |
| `set_bytes_array(tag, values)` | 写入二进制数组 |
| `encode_struct_array(tag, closure)` | 写入结构体数组 |
| `finish()` | 组装最终字节 |

## StructDecoder / DecodedField 方法速查

| 方法 | 说明 |
|------|------|
| `next_field()` | 迭代下一个字段，返回 `DecodedField` |
| `field.tag()` | 获取字段 tag |
| `field.as_integer()` | 读取 `i64` |
| `field.as_bool()` | 读取 `bool` |
| `field.as_double()` | 读取 `f64` |
| `field.as_string()` | 读取 `&str` |
| `field.as_bytes()` | 读取 `&[u8]` |
| `field.as_struct()` | 返回嵌套子解码器 |
| `field.as_integer_array()` | 读取 `Vec<i64>` |
| `field.as_bool_array()` | 读取 `Vec<bool>` |
| `field.as_double_array()` | 读取 `Vec<f64>` |
| `field.as_string_array()` | 读取 `Vec<&str>` |
| `field.as_bytes_array()` | 读取 `Vec<&[u8]>` |
| `field.as_struct_iter()` | 返回结构体数组迭代器 |

## 类型映射速查表

| Sproto 类型 | Rust 类型 | 编码方法 | 解码方法 |
|-------------|-----------|----------|----------|
| `integer` | `i64` | `set_integer` | `as_integer` |
| `boolean` | `bool` | `set_bool` | `as_bool` |
| `string` | `String` / `&str` | `set_string` | `as_string` |
| `binary` | `Vec<u8>` / `&[u8]` | `set_bytes` | `as_bytes` |
| `double` | `f64` | `set_double` | `as_double` |
| `*type` | `Vec<T>` | `set_*_array` / `encode_struct_array` | `as_*_array` / `as_struct_iter` |
| `.Type` | 嵌套结构体 | `encode_nested` | `as_struct` |
