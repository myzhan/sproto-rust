-- Test data generator for sproto-rust cross-validation.
--
-- This script uses the reference C/Lua sproto implementation to produce
-- binary fixture files that the Rust tests compare against.
--
-- Usage: LUA_CPATH="$HOME/github/sproto/?.so" LUA_PATH="$HOME/github/sproto/?.lua" lua generate.lua

local sproto = require "sproto"
local sprotoparser = require "sprotoparser"

local script_dir = arg[0]:match("(.-)[^/]*$")
if script_dir == "" then script_dir = "./" end

local function write_file(path, data)
    local f = assert(io.open(script_dir .. path, "wb"))
    f:write(data)
    f:close()
    print(string.format("wrote %s (%d bytes)", path, #data))
end

local function read_file(path)
    local f = assert(io.open(script_dir .. path, "r"))
    local data = f:read("*all")
    f:close()
    return data
end

local function hexdump(s)
    local t = {}
    for i = 1, #s do
        t[#t+1] = string.format("%02x", string.byte(s, i))
    end
    return table.concat(t, " ")
end

-- =============================================================================
-- Unified data schema: covers all sproto type mappings
--
-- Scalar types: string, integer, boolean, double, binary, integer(N)
-- Struct types: nested struct, struct array, recursive struct array
-- Array types: *string, *integer, *boolean, *double
-- =============================================================================

local schema_text = read_file("schema.sproto")

local schema_bin = sprotoparser.parse(schema_text)
write_file("schema.bin", schema_bin)

local sp = sproto.new(schema_bin)

-- Fixture 1: simple_struct -- basic scalars (string, integer, boolean)
local simple_struct = { name = "Alice", age = 13, active = false }
write_file("simple_struct_encoded.bin", sp:encode("Person", simple_struct))
write_file("simple_struct_packed.bin", sp:pencode("Person", simple_struct))
print("  simple_struct: " .. hexdump(sp:encode("Person", simple_struct)))

-- Fixture 2: all_scalars -- all 6 scalar types
local all_scalars = {
    name = "Alice",
    age = 30,
    active = true,
    score = 0.01171875,
    photo = "\x28\x29\x30\x31",
    fpn = 1.82,
}
write_file("all_scalars_encoded.bin", sp:encode("Person", all_scalars))
write_file("all_scalars_packed.bin", sp:pencode("Person", all_scalars))
print("  all_scalars: " .. hexdump(sp:encode("Person", all_scalars)))

-- Fixture 3: nested_struct -- single nested struct
local nested_struct = {
    name = "Alice",
    phone = { number = "123456789", type = 1 },
}
write_file("nested_struct_encoded.bin", sp:encode("Person", nested_struct))
write_file("nested_struct_packed.bin", sp:pencode("Person", nested_struct))
print("  nested_struct: " .. hexdump(sp:encode("Person", nested_struct)))

-- Fixture 4: struct_array -- recursive struct array (children)
local struct_array = {
    name = "Bob",
    age = 40,
    children = {
        { name = "Alice", age = 13 },
        { name = "Carol", age = 5 },
    },
}
write_file("struct_array_encoded.bin", sp:encode("Person", struct_array))
write_file("struct_array_packed.bin", sp:pencode("Person", struct_array))
print("  struct_array: " .. hexdump(sp:encode("Person", struct_array)))

-- Fixture 5: int_array -- integer array (4-byte elements)
local int_array = { numbers = { 1, 2, 3, 4, 5 } }
write_file("int_array_encoded.bin", sp:encode("Person", int_array))
write_file("int_array_packed.bin", sp:pencode("Person", int_array))
print("  int_array: " .. hexdump(sp:encode("Person", int_array)))

-- Fixture 6: big_int_array -- integer array (8-byte elements)
local big_int_array = { numbers = { (1<<32)+1, (1<<32)+2, (1<<32)+3 } }
write_file("big_int_array_encoded.bin", sp:encode("Person", big_int_array))
write_file("big_int_array_packed.bin", sp:pencode("Person", big_int_array))
print("  big_int_array: " .. hexdump(sp:encode("Person", big_int_array)))

-- Fixture 7: bool_array -- boolean array
local bool_array = { flags = { false, true, false } }
write_file("bool_array_encoded.bin", sp:encode("Person", bool_array))
write_file("bool_array_packed.bin", sp:pencode("Person", bool_array))
print("  bool_array: " .. hexdump(sp:encode("Person", bool_array)))

-- Fixture 8: number -- large integers (4-byte and 8-byte)
local number = { age = 100000, id = -10000000000 }
write_file("number_encoded.bin", sp:encode("Person", number))
write_file("number_packed.bin", sp:pencode("Person", number))
print("  number: " .. hexdump(sp:encode("Person", number)))

-- Fixture 9: double -- double scalar and double array
local double = { score = 0.01171875, values = { 0.01171875, 23, 4 } }
write_file("double_encoded.bin", sp:encode("Person", double))
write_file("double_packed.bin", sp:pencode("Person", double))
print("  double: " .. hexdump(sp:encode("Person", double)))

-- Fixture 10: string_array -- string array (including UTF-8)
local string_array = { tags = { "hello", "world", "\xe4\xbd\xa0\xe5\xa5\xbd" } }
write_file("string_array_encoded.bin", sp:encode("Person", string_array))
write_file("string_array_packed.bin", sp:pencode("Person", string_array))
print("  string_array: " .. hexdump(sp:encode("Person", string_array)))

-- Fixture 11: fixed_point -- integer(N) type
local fixed_point = { fpn = 1.82 }
write_file("fixed_point_encoded.bin", sp:encode("Person", fixed_point))
write_file("fixed_point_packed.bin", sp:pencode("Person", fixed_point))
print("  fixed_point: " .. hexdump(sp:encode("Person", fixed_point)))

-- Fixture 12: full -- all 14 fields populated
local full = {
    name = "Alice",
    age = 30,
    active = true,
    score = 0.01171875,
    photo = "\xDE\xAD\xBE\xEF",
    fpn = 1.82,
    id = 10000,
    phone = { number = "123456789", type = 1 },
    phones = {
        { number = "123456789", type = 1 },
        { number = "87654321", type = 2 },
    },
    children = {
        { name = "Bob", age = 5 },
    },
    tags = { "hello", "world", "\xe4\xbd\xa0\xe5\xa5\xbd" },
    numbers = { 1, 2, 3, 4, 5 },
    flags = { false, true, false },
    values = { 0.01171875, 23, 4 },
}
write_file("full_encoded.bin", sp:encode("Person", full))
write_file("full_packed.bin", sp:pencode("Person", full))
print("  full: " .. hexdump(sp:encode("Person", full)))

-- =============================================================================
-- RPC schema (unchanged)
-- =============================================================================

local rpc_schema_text = [[
.package {
    type 0 : integer
    session 1 : integer
    ud 2 : integer
}

.foobar_request {
    what 0 : string
}

.foobar_response {
    ok 0 : boolean
}

.foo_response {
    ok 0 : boolean
}

foobar 1 {
    request foobar_request
    response foobar_response
}

foo 2 {
    response foo_response
}

bar 3 {
    response nil
}

blackhole 4 {
}
]]

local rpc_schema_bin = sprotoparser.parse(rpc_schema_text)
write_file("rpc_schema.bin", rpc_schema_bin)

local server = sproto.new(rpc_schema_bin)
local server_host = server:host("package")

-- Client uses same schema for simplicity
local client = sproto.new(rpc_schema_bin)
local client_host = client:host("package")
local client_request = client_host:attach(server)

-- Generate RPC request for foobar
local req_foobar = client_request("foobar", { what = "hello" }, 1)
write_file("rpc_foobar_request.bin", req_foobar)

-- Dispatch and generate response
local type_, name, request_msg, response_fn = server_host:dispatch(req_foobar)
assert(type_ == "REQUEST" and name == "foobar")
print("  rpc foobar request decoded: what=" .. tostring(request_msg and request_msg.what))

local resp_foobar = response_fn({ ok = true })
write_file("rpc_foobar_response.bin", resp_foobar)

-- Generate RPC request for foo (no request body)
local req_foo = client_request("foo", nil, 2)
write_file("rpc_foo_request.bin", req_foo)

-- Generate RPC request for bar (response nil)
local req_bar = client_request("bar", nil, 3)
write_file("rpc_bar_request.bin", req_bar)

-- Dispatch bar and get response (nil response)
local type3, name3, _, response_fn3 = server_host:dispatch(req_bar)
assert(type3 == "REQUEST" and name3 == "bar")
local resp_bar = response_fn3()
write_file("rpc_bar_response.bin", resp_bar)

print("\nAll test fixtures generated successfully.")
