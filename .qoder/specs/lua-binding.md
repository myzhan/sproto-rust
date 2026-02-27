# Sproto Lua Binding 实现方案

## 概述

为 sproto-rust 创建 Lua 5.4 binding，生成可被 `require` 的 `.so` 文件。

## 项目结构

```
sproto-rust/
├── Cargo.toml                 # 添加 workspace member
├── sproto-lua/                # 新建子 crate
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs             # 模块入口
│   │   ├── conversion.rs      # Lua <-> SprotoValue 转换
│   │   ├── userdata.rs        # Sproto/Host/Sender userdata
│   │   └── error.rs           # 错误转换
│   └── tests/
│       └── test.lua           # Lua 集成测试
```

## Lua API 设计

```lua
local sproto = require "sproto_lua"

-- 解析 schema
local sp = sproto.parse([[
.Person {
    name 0 : string
    age 1 : integer
}
]])

-- 编码/解码 (直接使用 Lua table)
local encoded = sp:encode("Person", {name = "Alice", age = 30})
local decoded = sp:decode("Person", encoded)

-- 压缩/解压
local packed = sproto.pack(encoded)
local unpacked = sproto.unpack(packed)

-- RPC
local host = sp:host("package")
local result = host:dispatch(data)
local sender = host:attach(remote_sp)
local req = sender:request("hello", message, session)
```

## 类型映射

| Sproto | Lua |
|--------|-----|
| integer | number (integer) |
| boolean | boolean |
| string | string |
| binary | string |
| double | number |
| struct | table (字典) |
| array | table (1-indexed 数组) |

## 实现步骤

### 1. 项目配置

**修改根 Cargo.toml:**
```toml
[workspace]
members = ["sproto-derive", "sproto-lua"]
```

**创建 sproto-lua/Cargo.toml:**
```toml
[package]
name = "sproto-lua"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
sproto = { path = "..", default-features = false }
mlua = { version = "0.10", features = ["lua54", "vendored", "module"] }
```

### 2. 模块入口 (lib.rs)

- 使用 `#[mlua::lua_module]` 宏导出
- 注册全局函数: `parse`, `load_binary`, `pack`, `unpack`
- 注册 userdata: `SprotoUserData`, `HostUserData`, `SenderUserData`

### 3. 类型转换 (conversion.rs)

核心函数:
- `lua_to_sproto_value(LuaValue) -> SprotoValue`
- `sproto_value_to_lua(SprotoValue) -> LuaValue`
- `table_to_sproto_value(LuaTable) -> SprotoValue` (自动判断 array/struct)

### 4. Userdata 实现 (userdata.rs)

**SprotoUserData:**
- `encode(type_name, table) -> string`
- `decode(type_name, data) -> table`
- `host(package_name) -> Host`
- `get_type(name) -> table|nil`

**HostUserData:**
- `dispatch(data) -> table`
- `attach(remote_sproto) -> Sender`

**SenderUserData:**
- `request(proto, message, session?, ud?) -> string`

### 5. 错误处理 (error.rs)

将 `SprotoError` 转换为 `LuaError::RuntimeError`，保留详细错误信息。

## 关键文件

| 文件 | 说明 |
|------|------|
| `sproto-lua/Cargo.toml` | 项目配置，cdylib 输出 |
| `sproto-lua/src/lib.rs` | Lua 模块入口 |
| `sproto-lua/src/conversion.rs` | 类型转换核心 |
| `sproto-lua/src/userdata.rs` | Userdata 定义 |
| `sproto-lua/tests/test.lua` | Lua 测试脚本 |

## 构建和使用

```bash
# 构建
cd sproto-lua
cargo build --release

# 使用 (Linux/macOS)
cp target/release/libsproto_lua.so ./sproto_lua.so
lua -e "local sp = require 'sproto_lua'; print(sp)"

# 或设置 LUA_CPATH
export LUA_CPATH="./target/release/lib?.so;;"
```

## 验证方案

1. **编译测试**: `cargo build --release` 成功生成 `.so`
2. **加载测试**: Lua 能 `require "sproto_lua"`
3. **功能测试**: 运行 `tests/test.lua` 验证 encode/decode/pack/unpack
4. **兼容性测试**: 使用 `testdata/` 中的二进制文件验证与 C 实现兼容
5. **RPC 测试**: 验证 Host/Sender 功能

## 预计工作量

| 阶段 | 内容 |
|------|------|
| Phase 1 | 项目结构 + 基础 API (parse/encode/decode) |
| Phase 2 | pack/unpack + 复杂类型 + binary schema |
| Phase 3 | RPC 功能 (Host/Sender) |
| Phase 4 | 测试完善 + 文档 |
