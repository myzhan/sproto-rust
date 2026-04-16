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

local function hexdump(s)
    local t = {}
    for i = 1, #s do
        t[#t+1] = string.format("%02x", string.byte(s, i))
    end
    return table.concat(t, " ")
end

-- =============================================================================
-- Schema 1: Person + Data (wire protocol examples from README)
-- =============================================================================

local person_data_schema_text = [[
.Person {
    name 0 : string
    age 1 : integer
    marital 2 : boolean
    children 3 : *Person
}

.Data {
    numbers 0 : *integer
    bools 1 : *boolean
    number 2 : integer
    bignumber 3 : integer
    double 4 : double
    doubles 5 : *double
    fpn 6 : integer(2)
}
]]

local person_data_schema_bin = sprotoparser.parse(person_data_schema_text)
write_file("person_data_schema.bin", person_data_schema_bin)

local sp = sproto.new(person_data_schema_bin)

-- Example 1: simple_struct -- Person { name="Alice", age=13, marital=false }
local ex1 = sp:encode("Person", { name = "Alice", age = 13, marital = false })
write_file("simple_struct_encoded.bin", ex1)
print("  simple_struct: " .. hexdump(ex1))

-- Example 2: struct_array -- Person with children
local ex2 = sp:encode("Person", {
    name = "Bob",
    age = 40,
    children = {
        { name = "Alice", age = 13 },
        { name = "Carol", age = 5 },
    }
})
write_file("struct_array_encoded.bin", ex2)
print("  struct_array: " .. hexdump(ex2))

-- Example 3: number_array -- Data { numbers = {1,2,3,4,5} }
local ex3 = sp:encode("Data", { numbers = { 1, 2, 3, 4, 5 } })
write_file("number_array_encoded.bin", ex3)
print("  number_array: " .. hexdump(ex3))

-- Example 4: big_number_array -- Data { numbers = {(1<<32)+1, (1<<32)+2, (1<<32)+3} }
local ex4 = sp:encode("Data", {
    numbers = { (1<<32)+1, (1<<32)+2, (1<<32)+3 }
})
write_file("big_number_array_encoded.bin", ex4)
print("  big_number_array: " .. hexdump(ex4))

-- Example 5: bool_array -- Data { bools = {false, true, false} }
local ex5 = sp:encode("Data", { bools = { false, true, false } })
write_file("bool_array_encoded.bin", ex5)
print("  bool_array: " .. hexdump(ex5))

-- Example 6: number -- Data { number=100000, bignumber=-10000000000 }
local ex6 = sp:encode("Data", { number = 100000, bignumber = -10000000000 })
write_file("number_encoded.bin", ex6)
print("  number: " .. hexdump(ex6))

-- Example 7: double -- Data { double=0.01171875, doubles={0.01171875, 23, 4} }
local ex7 = sp:encode("Data", {
    double = 0.01171875,
    doubles = { 0.01171875, 23, 4 }
})
write_file("double_encoded.bin", ex7)
print("  double: " .. hexdump(ex7))

-- Example 8: fixed_point -- Data { fpn = 1.82 }
local ex8 = sp:encode("Data", { fpn = 1.82 })
write_file("fixed_point_encoded.bin", ex8)
print("  fixed_point: " .. hexdump(ex8))

-- Packed versions
write_file("simple_struct_packed.bin", sp:pencode("Person", { name = "Alice", age = 13, marital = false }))
write_file("struct_array_packed.bin", sp:pencode("Person", {
    name = "Bob", age = 40,
    children = {
        { name = "Alice", age = 13 },
        { name = "Carol", age = 5 },
    }
}))
write_file("number_array_packed.bin", sp:pencode("Data", { numbers = { 1, 2, 3, 4, 5 } }))
write_file("big_number_array_packed.bin", sp:pencode("Data", { numbers = { (1<<32)+1, (1<<32)+2, (1<<32)+3 } }))
write_file("bool_array_packed.bin", sp:pencode("Data", { bools = { false, true, false } }))
write_file("number_packed.bin", sp:pencode("Data", { number = 100000, bignumber = -10000000000 }))
write_file("double_packed.bin", sp:pencode("Data", { double = 0.01171875, doubles = { 0.01171875, 23, 4 } }))
write_file("fixed_point_packed.bin", sp:pencode("Data", { fpn = 1.82 }))

-- =============================================================================
-- Schema 2: AddressBook (map / indexed array)
-- =============================================================================

local addressbook_schema_text = [[
.Person {
    name 0 : string
    id 1 : integer
    email 2 : string

    .PhoneNumber {
        number 0 : string
        type 1 : integer
    }

    phone 3 : *PhoneNumber
}

.AddressBook {
    person 0 : *Person(id)
    others 1 : *Person
}
]]

local addressbook_schema_bin = sprotoparser.parse(addressbook_schema_text)
write_file("addressbook_schema.bin", addressbook_schema_bin)

local sp2 = sproto.new(addressbook_schema_bin)

local ab = {
    person = {
        { name = "Alice", id = 10000, phone = {
            { number = "123456789", type = 1 },
            { number = "87654321", type = 2 },
        }},
        { name = "Bob", id = 20000, phone = {
            { number = "01234567890", type = 3 },
        }},
    },
    others = {
        { name = "Carol", id = 30000 },
    },
}

local ab_encoded = sp2:encode("AddressBook", ab)
write_file("addressbook_encoded.bin", ab_encoded)
write_file("addressbook_packed.bin", sp2:pencode("AddressBook", ab))

-- =============================================================================
-- Schema 3: RPC (server + client)
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
