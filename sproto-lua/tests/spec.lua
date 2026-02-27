-- Busted test suite for sproto-lua
--
-- Run with: cd sproto-lua && busted tests/spec.lua
-- Or:       make lua-test

local sproto = require "sproto_lua"

-- Helper to read binary file
local function read_file(path)
    local f = assert(io.open(path, "rb"))
    local data = f:read("*all")
    f:close()
    return data
end

-- Path to testdata directory (relative to sproto-lua)
local testdata = "../testdata/"

-- =============================================================================
-- Basic Functionality Tests
-- =============================================================================

describe("sproto.parse", function()
    it("parses valid schema", function()
        local sp = sproto.parse([[
.Person {
    name 0 : string
    age 1 : integer
}
]])
        assert.is_not_nil(sp)
    end)

    it("fails on invalid schema", function()
        assert.has_error(function()
            sproto.parse("invalid schema {{{")
        end)
    end)
end)

describe("sproto.load_binary", function()
    it("loads binary schema from testdata", function()
        local data = read_file(testdata .. "person_data_schema.bin")
        local sp = sproto.load_binary(data)
        assert.is_not_nil(sp)
    end)

    it("loads addressbook schema", function()
        local data = read_file(testdata .. "addressbook_schema.bin")
        local sp = sproto.load_binary(data)
        assert.is_not_nil(sp)
    end)

    it("loads rpc schema", function()
        local data = read_file(testdata .. "rpc_schema.bin")
        local sp = sproto.load_binary(data)
        assert.is_not_nil(sp)
    end)
end)

describe("encode/decode", function()
    local sp

    before_each(function()
        sp = sproto.parse([[
.Person {
    name 0 : string
    age 1 : integer
    marital 2 : boolean
}
]])
    end)

    it("encodes simple struct", function()
        local data = {name = "Alice", age = 30}
        local encoded = sp:encode("Person", data)
        assert.is_string(encoded)
        assert.is_true(#encoded > 0)
    end)

    it("decodes to original values", function()
        local data = {name = "Bob", age = 25, marital = true}
        local encoded = sp:encode("Person", data)
        local decoded = sp:decode("Person", encoded)
        
        assert.are.equal("Bob", decoded.name)
        assert.are.equal(25, decoded.age)
        assert.are.equal(true, decoded.marital)
    end)

    it("handles boolean false correctly", function()
        local data = {name = "Carol", age = 20, marital = false}
        local encoded = sp:encode("Person", data)
        local decoded = sp:decode("Person", encoded)
        
        assert.are.equal(false, decoded.marital)
    end)

    it("fails on unknown type", function()
        assert.has_error(function()
            sp:encode("Unknown", {})
        end)
    end)
end)

describe("double type", function()
    local sp

    before_each(function()
        sp = sproto.parse([[
.Point {
    x 0 : double
    y 1 : double
}
]])
    end)

    it("encodes and decodes doubles", function()
        local data = {x = 3.14159, y = -2.71828}
        local encoded = sp:encode("Point", data)
        local decoded = sp:decode("Point", encoded)
        
        assert.is_near(3.14159, decoded.x, 0.00001)
        assert.is_near(-2.71828, decoded.y, 0.00001)
    end)
end)

describe("arrays", function()
    describe("integer array", function()
        local sp

        before_each(function()
            sp = sproto.parse([[
.Numbers {
    values 0 : *integer
}
]])
        end)

        it("encodes and decodes integer array", function()
            local data = {values = {10, 20, 30, 40, 50}}
            local encoded = sp:encode("Numbers", data)
            local decoded = sp:decode("Numbers", encoded)
            
            assert.are.equal(5, #decoded.values)
            for i = 1, 5 do
                assert.are.equal(data.values[i], decoded.values[i])
            end
        end)
    end)

    describe("string array", function()
        local sp

        before_each(function()
            sp = sproto.parse([[
.Tags {
    items 0 : *string
}
]])
        end)

        it("encodes and decodes string array", function()
            local data = {items = {"apple", "banana", "cherry"}}
            local encoded = sp:encode("Tags", data)
            local decoded = sp:decode("Tags", encoded)
            
            assert.are.equal(3, #decoded.items)
            assert.are.equal("apple", decoded.items[1])
            assert.are.equal("banana", decoded.items[2])
            assert.are.equal("cherry", decoded.items[3])
        end)
    end)

    describe("boolean array", function()
        local sp

        before_each(function()
            sp = sproto.parse([[
.Flags {
    values 0 : *boolean
}
]])
        end)

        it("encodes and decodes boolean array", function()
            local data = {values = {true, false, true, false}}
            local encoded = sp:encode("Flags", data)
            local decoded = sp:decode("Flags", encoded)
            
            assert.are.equal(4, #decoded.values)
            assert.are.equal(true, decoded.values[1])
            assert.are.equal(false, decoded.values[2])
            assert.are.equal(true, decoded.values[3])
            assert.are.equal(false, decoded.values[4])
        end)
    end)
end)

describe("nested structs", function()
    local sp

    before_each(function()
        sp = sproto.parse([[
.Address {
    city 0 : string
    zip 1 : string
}
.Person {
    name 0 : string
    address 1 : Address
}
]])
    end)

    it("encodes and decodes nested struct", function()
        local data = {
            name = "Charlie",
            address = {
                city = "New York",
                zip = "10001"
            }
        }
        local encoded = sp:encode("Person", data)
        local decoded = sp:decode("Person", encoded)
        
        assert.are.equal("Charlie", decoded.name)
        assert.are.equal("New York", decoded.address.city)
        assert.are.equal("10001", decoded.address.zip)
    end)
end)

describe("pack/unpack", function()
    local sp

    before_each(function()
        sp = sproto.parse([[
.Data {
    value 0 : integer
}
]])
    end)

    it("packs and unpacks data", function()
        local encoded = sp:encode("Data", {value = 12345})
        local packed = sproto.pack(encoded)
        local unpacked = sproto.unpack(packed)
        
        assert.is_string(packed)
        assert.is_string(unpacked)
        
        -- Verify unpacked data can be decoded
        local decoded = sp:decode("Data", unpacked)
        assert.are.equal(12345, decoded.value)
    end)
end)

describe("schema introspection", function()
    local sp

    before_each(function()
        sp = sproto.parse([[
.Person {
    name 0 : string
    age 1 : integer
}
.LoginReq { username 0 : string }
.LoginResp { ok 0 : boolean }
login 1 {
    request LoginReq
    response LoginResp
}
]])
    end)

    it("get_type returns type info", function()
        local typeinfo = sp:get_type("Person")
        assert.is_not_nil(typeinfo)
        assert.are.equal("Person", typeinfo.name)
        assert.is_table(typeinfo.fields)
    end)

    it("get_type returns nil for unknown type", function()
        local typeinfo = sp:get_type("Unknown")
        assert.is_nil(typeinfo)
    end)

    it("get_protocol returns protocol info", function()
        local proto = sp:get_protocol("login")
        assert.is_not_nil(proto)
        assert.are.equal("login", proto.name)
        assert.are.equal(1, proto.tag)
    end)

    it("get_protocol returns nil for unknown protocol", function()
        local proto = sp:get_protocol("unknown")
        assert.is_nil(proto)
    end)
end)

describe("unicode support", function()
    local sp

    before_each(function()
        sp = sproto.parse([[
.Message {
    text 0 : string
}
]])
    end)

    it("handles unicode strings", function()
        local data = {text = "Hello 世界 🌍"}
        local encoded = sp:encode("Message", data)
        local decoded = sp:decode("Message", encoded)
        
        assert.are.equal("Hello 世界 🌍", decoded.text)
    end)

    it("handles Chinese characters", function()
        local data = {text = "你好，世界！"}
        local encoded = sp:encode("Message", data)
        local decoded = sp:decode("Message", encoded)
        
        assert.are.equal("你好，世界！", decoded.text)
    end)
end)

-- =============================================================================
-- Cross-Compatibility Tests (using testdata from C/Lua reference implementation)
-- =============================================================================

describe("cross-compatibility with C/Lua reference", function()
    describe("person_data schema", function()
        local sp

        before_each(function()
            local data = read_file(testdata .. "person_data_schema.bin")
            sp = sproto.load_binary(data)
        end)

        it("decodes example1 (Person: Alice)", function()
            local encoded = read_file(testdata .. "example1_encoded.bin")
            local decoded = sp:decode("Person", encoded)
            
            assert.are.equal("Alice", decoded.name)
            assert.are.equal(13, decoded.age)
            assert.are.equal(false, decoded.marital)
        end)

        it("decodes example2 (Person with children)", function()
            local encoded = read_file(testdata .. "example2_encoded.bin")
            local decoded = sp:decode("Person", encoded)
            
            assert.are.equal("Bob", decoded.name)
            assert.are.equal(40, decoded.age)
            assert.is_table(decoded.children)
            assert.are.equal(2, #decoded.children)
            assert.are.equal("Alice", decoded.children[1].name)
            assert.are.equal(13, decoded.children[1].age)
            assert.are.equal("Carol", decoded.children[2].name)
            assert.are.equal(5, decoded.children[2].age)
        end)

        it("decodes example3 (Data: integer array)", function()
            local encoded = read_file(testdata .. "example3_encoded.bin")
            local decoded = sp:decode("Data", encoded)
            
            assert.is_table(decoded.numbers)
            assert.are.equal(5, #decoded.numbers)
            for i = 1, 5 do
                assert.are.equal(i, decoded.numbers[i])
            end
        end)

        it("decodes example4 (Data: large integers)", function()
            local encoded = read_file(testdata .. "example4_encoded.bin")
            local decoded = sp:decode("Data", encoded)
            
            assert.is_table(decoded.numbers)
            assert.are.equal(3, #decoded.numbers)
            assert.are.equal((1 << 32) + 1, decoded.numbers[1])
            assert.are.equal((1 << 32) + 2, decoded.numbers[2])
            assert.are.equal((1 << 32) + 3, decoded.numbers[3])
        end)

        it("decodes example5 (Data: boolean array)", function()
            local encoded = read_file(testdata .. "example5_encoded.bin")
            local decoded = sp:decode("Data", encoded)
            
            assert.is_table(decoded.bools)
            assert.are.equal(3, #decoded.bools)
            assert.are.equal(false, decoded.bools[1])
            assert.are.equal(true, decoded.bools[2])
            assert.are.equal(false, decoded.bools[3])
        end)

        it("decodes example6 (Data: large negative integer)", function()
            local encoded = read_file(testdata .. "example6_encoded.bin")
            local decoded = sp:decode("Data", encoded)
            
            assert.are.equal(100000, decoded.number)
            assert.are.equal(-10000000000, decoded.bignumber)
        end)

        it("decodes example7 (Data: double and double array)", function()
            local encoded = read_file(testdata .. "example7_encoded.bin")
            local decoded = sp:decode("Data", encoded)
            
            assert.is_near(0.01171875, decoded.double, 0.0000001)
            assert.is_table(decoded.doubles)
            assert.are.equal(3, #decoded.doubles)
            assert.is_near(0.01171875, decoded.doubles[1], 0.0000001)
            assert.is_near(23, decoded.doubles[2], 0.0000001)
            assert.is_near(4, decoded.doubles[3], 0.0000001)
        end)

        it("decodes example8 (Data: fixed point number)", function()
            local encoded = read_file(testdata .. "example8_encoded.bin")
            local decoded = sp:decode("Data", encoded)
            
            -- fpn is integer(2), stored as 182 (1.82 * 100)
            assert.are.equal(182, decoded.fpn)
        end)

        -- Test pack/unpack with reference data
        it("unpacks example1_packed", function()
            local packed = read_file(testdata .. "example1_packed.bin")
            local unpacked = sproto.unpack(packed)
            local decoded = sp:decode("Person", unpacked)
            
            assert.are.equal("Alice", decoded.name)
            assert.are.equal(13, decoded.age)
        end)

        it("unpacks example2_packed", function()
            local packed = read_file(testdata .. "example2_packed.bin")
            local unpacked = sproto.unpack(packed)
            local decoded = sp:decode("Person", unpacked)
            
            assert.are.equal("Bob", decoded.name)
            assert.are.equal(2, #decoded.children)
        end)
    end)

    describe("addressbook schema", function()
        local sp

        before_each(function()
            local data = read_file(testdata .. "addressbook_schema.bin")
            sp = sproto.load_binary(data)
        end)

        it("decodes addressbook with map", function()
            local encoded = read_file(testdata .. "addressbook_encoded.bin")
            local decoded = sp:decode("AddressBook", encoded)
            
            -- person is a map indexed by id
            assert.is_table(decoded.person)
            
            -- others is a regular array
            assert.is_table(decoded.others)
            assert.are.equal(1, #decoded.others)
            assert.are.equal("Carol", decoded.others[1].name)
        end)

        it("unpacks addressbook_packed", function()
            local packed = read_file(testdata .. "addressbook_packed.bin")
            local unpacked = sproto.unpack(packed)
            local decoded = sp:decode("AddressBook", unpacked)
            
            assert.is_table(decoded.person)
            assert.is_table(decoded.others)
        end)
    end)

    describe("rpc schema", function()
        local sp

        before_each(function()
            local data = read_file(testdata .. "rpc_schema.bin")
            sp = sproto.load_binary(data)
        end)

        it("has package type", function()
            local typeinfo = sp:get_type("package")
            assert.is_not_nil(typeinfo)
        end)

        it("has foobar protocol", function()
            local proto = sp:get_protocol("foobar")
            assert.is_not_nil(proto)
            assert.are.equal(1, proto.tag)
        end)

        it("has foo protocol", function()
            local proto = sp:get_protocol("foo")
            assert.is_not_nil(proto)
            assert.are.equal(2, proto.tag)
        end)

        it("has bar protocol", function()
            local proto = sp:get_protocol("bar")
            assert.is_not_nil(proto)
            assert.are.equal(3, proto.tag)
        end)
    end)
end)

-- =============================================================================
-- Round-trip Tests (encode -> decode should preserve data)
-- =============================================================================

describe("round-trip consistency", function()
    local sp

    before_each(function()
        sp = sproto.parse([[
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
}
]])
    end)

    it("Person with children round-trip", function()
        local original = {
            name = "Parent",
            age = 45,
            marital = true,
            children = {
                {name = "Child1", age = 10},
                {name = "Child2", age = 8, marital = false},
            }
        }
        local encoded = sp:encode("Person", original)
        local decoded = sp:decode("Person", encoded)
        
        assert.are.equal(original.name, decoded.name)
        assert.are.equal(original.age, decoded.age)
        assert.are.equal(original.marital, decoded.marital)
        assert.are.equal(#original.children, #decoded.children)
        assert.are.equal("Child1", decoded.children[1].name)
        assert.are.equal("Child2", decoded.children[2].name)
    end)

    it("Data with all fields round-trip", function()
        local original = {
            numbers = {100, 200, 300},
            bools = {true, false, true},
            number = 999999,
            bignumber = (1 << 40),
            double = 3.14159265358979,
            doubles = {1.1, 2.2, 3.3}
        }
        local encoded = sp:encode("Data", original)
        local decoded = sp:decode("Data", encoded)
        
        assert.are.same(original.numbers, decoded.numbers)
        assert.are.same(original.bools, decoded.bools)
        assert.are.equal(original.number, decoded.number)
        assert.are.equal(original.bignumber, decoded.bignumber)
        assert.is_near(original.double, decoded.double, 0.0000001)
        for i = 1, 3 do
            assert.is_near(original.doubles[i], decoded.doubles[i], 0.0000001)
        end
    end)

    it("encode -> pack -> unpack -> decode round-trip", function()
        local original = {name = "Test", age = 100, marital = true}
        local encoded = sp:encode("Person", original)
        local packed = sproto.pack(encoded)
        local unpacked = sproto.unpack(packed)
        local decoded = sp:decode("Person", unpacked)
        
        assert.are.equal(original.name, decoded.name)
        assert.are.equal(original.age, decoded.age)
        assert.are.equal(original.marital, decoded.marital)
    end)
end)
