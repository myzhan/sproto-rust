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
local testdata = "../tests/testdata/"

-- Pre-load binary schemas used across tests
local function load_schema()
    return sproto.load_binary(read_file(testdata .. "schema.bin"))
end

local function load_rpc_schema()
    return sproto.load_binary(read_file(testdata .. "rpc_schema.bin"))
end

-- =============================================================================
-- Basic Functionality Tests
-- =============================================================================

describe("sproto.load_binary", function()
    it("loads binary schema from testdata", function()
        local sp = load_schema()
        assert.is_not_nil(sp)
    end)

    it("loads rpc schema", function()
        local sp = load_rpc_schema()
        assert.is_not_nil(sp)
    end)
end)

describe("encode/decode", function()
    local sp

    before_each(function()
        -- schema.bin Person: name 0, age 1, active 2 (boolean), ...
        sp = load_schema()
    end)

    it("encodes simple struct", function()
        local data = {name = "Alice", age = 30}
        local encoded = sp:encode("Person", data)
        assert.is_string(encoded)
        assert.is_true(#encoded > 0)
    end)

    it("decodes to original values", function()
        local data = {name = "Bob", age = 25, active = true}
        local encoded = sp:encode("Person", data)
        local decoded = sp:decode("Person", encoded)

        assert.are.equal("Bob", decoded.name)
        assert.are.equal(25, decoded.age)
        assert.are.equal(true, decoded.active)
    end)

    it("handles boolean false correctly", function()
        local data = {name = "Carol", age = 20, active = false}
        local encoded = sp:encode("Person", data)
        local decoded = sp:decode("Person", encoded)

        assert.are.equal(false, decoded.active)
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
        -- schema.bin Person has: score 3 : double, values 13 : *double
        sp = load_schema()
    end)

    it("encodes and decodes doubles", function()
        local data = {score = 3.14159}
        local encoded = sp:encode("Person", data)
        local decoded = sp:decode("Person", encoded)

        assert.is_near(3.14159, decoded.score, 0.00001)
    end)
end)

describe("arrays", function()
    describe("integer array", function()
        local sp

        before_each(function()
            -- schema.bin Person has: numbers 11 : *integer
            sp = load_schema()
        end)

        it("encodes and decodes integer array", function()
            local data = {numbers = {10, 20, 30, 40, 50}}
            local encoded = sp:encode("Person", data)
            local decoded = sp:decode("Person", encoded)

            assert.are.equal(5, #decoded.numbers)
            for i = 1, 5 do
                assert.are.equal(data.numbers[i], decoded.numbers[i])
            end
        end)
    end)

    describe("string array", function()
        local sp

        before_each(function()
            -- schema.bin Person has: tags 10 : *string
            sp = load_schema()
        end)

        it("encodes and decodes string array", function()
            local data = {tags = {"apple", "banana", "cherry"}}
            local encoded = sp:encode("Person", data)
            local decoded = sp:decode("Person", encoded)

            assert.are.equal(3, #decoded.tags)
            assert.are.equal("apple", decoded.tags[1])
            assert.are.equal("banana", decoded.tags[2])
            assert.are.equal("cherry", decoded.tags[3])
        end)
    end)

    describe("boolean array", function()
        local sp

        before_each(function()
            -- schema.bin Person has: flags 12 : *boolean
            sp = load_schema()
        end)

        it("encodes and decodes boolean array", function()
            local data = {flags = {true, false, true, false}}
            local encoded = sp:encode("Person", data)
            local decoded = sp:decode("Person", encoded)

            assert.are.equal(4, #decoded.flags)
            assert.are.equal(true, decoded.flags[1])
            assert.are.equal(false, decoded.flags[2])
            assert.are.equal(true, decoded.flags[3])
            assert.are.equal(false, decoded.flags[4])
        end)
    end)
end)

describe("nested structs", function()
    local sp

    before_each(function()
        -- schema.bin Person has: phone 7 : PhoneNumber, phones 8 : *PhoneNumber
        -- children 9 : *Person (recursive)
        sp = load_schema()
    end)

    it("encodes and decodes nested struct", function()
        local data = {
            name = "Charlie",
            phone = {
                number = "123456789",
                type = 1
            }
        }
        local encoded = sp:encode("Person", data)
        local decoded = sp:decode("Person", encoded)

        assert.are.equal("Charlie", decoded.name)
        assert.are.equal("123456789", decoded.phone.number)
        assert.are.equal(1, decoded.phone.type)
    end)

    it("encodes and decodes struct array", function()
        local data = {
            name = "Parent",
            phones = {
                {number = "111", type = 1},
                {number = "222", type = 2},
            }
        }
        local encoded = sp:encode("Person", data)
        local decoded = sp:decode("Person", encoded)

        assert.are.equal("Parent", decoded.name)
        assert.are.equal(2, #decoded.phones)
        assert.are.equal("111", decoded.phones[1].number)
        assert.are.equal(1, decoded.phones[1].type)
        assert.are.equal("222", decoded.phones[2].number)
        assert.are.equal(2, decoded.phones[2].type)
    end)

    it("encodes and decodes recursive struct (children)", function()
        local data = {
            name = "Bob",
            age = 40,
            children = {
                {name = "Alice", age = 13},
                {name = "Carol", age = 5},
            }
        }
        local encoded = sp:encode("Person", data)
        local decoded = sp:decode("Person", encoded)

        assert.are.equal("Bob", decoded.name)
        assert.are.equal(40, decoded.age)
        assert.are.equal(2, #decoded.children)
        assert.are.equal("Alice", decoded.children[1].name)
        assert.are.equal(13, decoded.children[1].age)
        assert.are.equal("Carol", decoded.children[2].name)
        assert.are.equal(5, decoded.children[2].age)
    end)
end)

describe("pack/unpack", function()
    local sp

    before_each(function()
        sp = load_schema()
    end)

    it("packs and unpacks data", function()
        local encoded = sp:encode("Person", {age = 12345})
        local packed = sproto.pack(encoded)
        local unpacked = sproto.unpack(packed)

        assert.is_string(packed)
        assert.is_string(unpacked)

        -- Verify unpacked data can be decoded
        local decoded = sp:decode("Person", unpacked)
        assert.are.equal(12345, decoded.age)
    end)
end)

describe("schema introspection", function()
    local sp

    before_each(function()
        -- rpc_schema.bin has types and protocols
        sp = load_rpc_schema()
    end)

    it("get_type returns type info", function()
        local typeinfo = sp:get_type("package")
        assert.is_not_nil(typeinfo)
        assert.are.equal("package", typeinfo.name)
        assert.is_table(typeinfo.fields)
    end)

    it("get_type returns nil for unknown type", function()
        local typeinfo = sp:get_type("Unknown")
        assert.is_nil(typeinfo)
    end)

    it("get_protocol returns protocol info", function()
        local proto = sp:get_protocol("foobar")
        assert.is_not_nil(proto)
        assert.are.equal("foobar", proto.name)
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
        -- schema.bin Person has: name 0 : string
        sp = load_schema()
    end)

    it("handles unicode strings", function()
        local data = {name = "Hello \xe4\xb8\x96\xe7\x95\x8c \xf0\x9f\x8c\x8d"}
        local encoded = sp:encode("Person", data)
        local decoded = sp:decode("Person", encoded)

        assert.are.equal("Hello \xe4\xb8\x96\xe7\x95\x8c \xf0\x9f\x8c\x8d", decoded.name)
    end)

    it("handles Chinese characters", function()
        local data = {name = "\xe4\xbd\xa0\xe5\xa5\xbd\xef\xbc\x8c\xe4\xb8\x96\xe7\x95\x8c\xef\xbc\x81"}
        local encoded = sp:encode("Person", data)
        local decoded = sp:decode("Person", encoded)

        assert.are.equal("\xe4\xbd\xa0\xe5\xa5\xbd\xef\xbc\x8c\xe4\xb8\x96\xe7\x95\x8c\xef\xbc\x81", decoded.name)
    end)
end)

-- =============================================================================
-- Additional tests from Go reference (gosproto/sproto_test.go)
-- =============================================================================

describe("binary type", function()
    local sp

    before_each(function()
        -- schema.bin Person has: photo 4 : binary
        sp = load_schema()
    end)

    it("encodes and decodes binary data", function()
        local data = {photo = string.char(0x28, 0x29, 0x30, 0x31)}
        local encoded = sp:encode("Person", data)
        local decoded = sp:decode("Person", encoded)
        assert.are.equal(string.char(0x28, 0x29, 0x30, 0x31), decoded.photo)
    end)

    it("handles empty binary field", function()
        local data = {photo = ""}
        local encoded = sp:encode("Person", data)
        local decoded = sp:decode("Person", encoded)
        assert.are.equal("", decoded.photo)
    end)
end)

describe("large integers", function()
    local sp

    before_each(function()
        -- schema.bin Person has: age 1 : integer, id 6 : integer
        sp = load_schema()
    end)

    it("encodes 32-bit integer in data part (100000)", function()
        local data = {age = 100000}
        local encoded = sp:encode("Person", data)
        local decoded = sp:decode("Person", encoded)
        assert.are.equal(100000, decoded.age)
    end)

    it("encodes negative 64-bit integer (-10000000000)", function()
        local data = {id = -10000000000}
        local encoded = sp:encode("Person", data)
        local decoded = sp:decode("Person", encoded)
        assert.are.equal(-10000000000, decoded.id)
    end)

    it("encodes both age and id", function()
        local data = {age = 100000, id = -10000000000}
        local encoded = sp:encode("Person", data)
        local decoded = sp:decode("Person", encoded)
        assert.are.equal(100000, decoded.age)
        assert.are.equal(-10000000000, decoded.id)
    end)
end)

describe("big number array", function()
    local sp

    before_each(function()
        -- schema.bin Person has: numbers 11 : *integer
        sp = load_schema()
    end)

    it("encodes and decodes 64-bit integer array", function()
        local data = {numbers = {(1 << 32) + 1, (1 << 32) + 2, (1 << 32) + 3}}
        local encoded = sp:encode("Person", data)
        local decoded = sp:decode("Person", encoded)

        assert.are.equal(3, #decoded.numbers)
        assert.are.equal((1 << 32) + 1, decoded.numbers[1])
        assert.are.equal((1 << 32) + 2, decoded.numbers[2])
        assert.are.equal((1 << 32) + 3, decoded.numbers[3])
    end)
end)

describe("double array", function()
    local sp

    before_each(function()
        -- schema.bin Person has: score 3 : double, values 13 : *double
        sp = load_schema()
    end)

    it("encodes Go-specific double values (0.01171875, 23, 4)", function()
        local data = {score = 0.01171875, values = {0.01171875, 23, 4}}
        local encoded = sp:encode("Person", data)
        local decoded = sp:decode("Person", encoded)

        assert.is_near(0.01171875, decoded.score, 0.0000001)
        assert.are.equal(3, #decoded.values)
        assert.is_near(0.01171875, decoded.values[1], 0.0000001)
        assert.is_near(23, decoded.values[2], 0.0000001)
        assert.is_near(4, decoded.values[3], 0.0000001)
    end)
end)

describe("empty arrays", function()
    local sp

    before_each(function()
        -- schema.bin Person has: numbers 11, flags 12, tags 10, values 13
        sp = load_schema()
    end)

    it("encodes and decodes empty integer array", function()
        local data = {numbers = {}}
        local encoded = sp:encode("Person", data)
        local decoded = sp:decode("Person", encoded)

        assert.is_table(decoded.numbers)
        assert.are.equal(0, #decoded.numbers)
    end)

    it("encodes and decodes empty double array", function()
        local data = {values = {}}
        local encoded = sp:encode("Person", data)
        local decoded = sp:decode("Person", encoded)

        assert.is_table(decoded.values)
        assert.are.equal(0, #decoded.values)
    end)

    it("encodes and decodes empty boolean array", function()
        local data = {flags = {}}
        local encoded = sp:encode("Person", data)
        local decoded = sp:decode("Person", encoded)

        assert.is_table(decoded.flags)
        assert.are.equal(0, #decoded.flags)
    end)

    it("encodes and decodes empty string array", function()
        local data = {tags = {}}
        local encoded = sp:encode("Person", data)
        local decoded = sp:decode("Person", encoded)

        assert.is_table(decoded.tags)
        assert.are.equal(0, #decoded.tags)
    end)
end)

-- =============================================================================
-- Cross-Compatibility Tests (using testdata from C/Lua reference implementation)
-- =============================================================================

describe("cross-compatibility with C/Lua reference", function()
    describe("unified schema", function()
        local sp

        before_each(function()
            sp = load_schema()
        end)

        it("decodes simple_struct (Person: Alice)", function()
            local encoded = read_file(testdata .. "simple_struct_encoded.bin")
            local decoded = sp:decode("Person", encoded)

            assert.are.equal("Alice", decoded.name)
            assert.are.equal(13, decoded.age)
            assert.are.equal(false, decoded.active)
        end)

        it("decodes all_scalars (Person: all scalar types)", function()
            local encoded = read_file(testdata .. "all_scalars_encoded.bin")
            local decoded = sp:decode("Person", encoded)

            assert.are.equal("Alice", decoded.name)
            assert.are.equal(30, decoded.age)
            assert.are.equal(true, decoded.active)
            assert.is_near(0.01171875, decoded.score, 0.0000001)
            assert.are.equal(string.char(0x28, 0x29, 0x30, 0x31), decoded.photo)
            assert.are.equal(182, decoded.fpn)
        end)

        it("decodes nested_struct (Person: with phone)", function()
            local encoded = read_file(testdata .. "nested_struct_encoded.bin")
            local decoded = sp:decode("Person", encoded)

            assert.are.equal("Alice", decoded.name)
            assert.is_not_nil(decoded.phone)
            assert.are.equal("123456789", decoded.phone.number)
            assert.are.equal(1, decoded.phone.type)
        end)

        it("decodes struct_array (Person with children)", function()
            local encoded = read_file(testdata .. "struct_array_encoded.bin")
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

        it("decodes int_array (Person: integer array)", function()
            local encoded = read_file(testdata .. "int_array_encoded.bin")
            local decoded = sp:decode("Person", encoded)

            assert.is_table(decoded.numbers)
            assert.are.equal(5, #decoded.numbers)
            for i = 1, 5 do
                assert.are.equal(i, decoded.numbers[i])
            end
        end)

        it("decodes big_int_array (Person: large integers)", function()
            local encoded = read_file(testdata .. "big_int_array_encoded.bin")
            local decoded = sp:decode("Person", encoded)

            assert.is_table(decoded.numbers)
            assert.are.equal(3, #decoded.numbers)
            assert.are.equal((1 << 32) + 1, decoded.numbers[1])
            assert.are.equal((1 << 32) + 2, decoded.numbers[2])
            assert.are.equal((1 << 32) + 3, decoded.numbers[3])
        end)

        it("decodes bool_array (Person: boolean array)", function()
            local encoded = read_file(testdata .. "bool_array_encoded.bin")
            local decoded = sp:decode("Person", encoded)

            assert.is_table(decoded.flags)
            assert.are.equal(3, #decoded.flags)
            assert.are.equal(false, decoded.flags[1])
            assert.are.equal(true, decoded.flags[2])
            assert.are.equal(false, decoded.flags[3])
        end)

        it("decodes number (Person: large integers)", function()
            local encoded = read_file(testdata .. "number_encoded.bin")
            local decoded = sp:decode("Person", encoded)

            assert.are.equal(100000, decoded.age)
            assert.are.equal(-10000000000, decoded.id)
        end)

        it("decodes double (Person: double and double array)", function()
            local encoded = read_file(testdata .. "double_encoded.bin")
            local decoded = sp:decode("Person", encoded)

            assert.is_near(0.01171875, decoded.score, 0.0000001)
            assert.is_table(decoded.values)
            assert.are.equal(3, #decoded.values)
            assert.is_near(0.01171875, decoded.values[1], 0.0000001)
            assert.is_near(23, decoded.values[2], 0.0000001)
            assert.is_near(4, decoded.values[3], 0.0000001)
        end)

        it("decodes string_array (Person: string array with UTF-8)", function()
            local encoded = read_file(testdata .. "string_array_encoded.bin")
            local decoded = sp:decode("Person", encoded)

            assert.is_table(decoded.tags)
            assert.are.equal(3, #decoded.tags)
            assert.are.equal("hello", decoded.tags[1])
            assert.are.equal("world", decoded.tags[2])
            assert.are.equal("\xe4\xbd\xa0\xe5\xa5\xbd", decoded.tags[3])
        end)

        it("decodes fixed_point (Person: fixed point number)", function()
            local encoded = read_file(testdata .. "fixed_point_encoded.bin")
            local decoded = sp:decode("Person", encoded)

            -- fpn is integer(2), stored as 182 (1.82 * 100)
            assert.are.equal(182, decoded.fpn)
        end)

        it("decodes full (Person: all 14 fields)", function()
            local encoded = read_file(testdata .. "full_encoded.bin")
            local decoded = sp:decode("Person", encoded)

            assert.are.equal("Alice", decoded.name)
            assert.are.equal(30, decoded.age)
            assert.are.equal(true, decoded.active)
            assert.is_near(0.01171875, decoded.score, 0.0000001)
            assert.are.equal(string.char(0xDE, 0xAD, 0xBE, 0xEF), decoded.photo)
            assert.are.equal(182, decoded.fpn)
            assert.are.equal(10000, decoded.id)
            assert.are.equal("123456789", decoded.phone.number)
            assert.are.equal(1, decoded.phone.type)
            assert.are.equal(2, #decoded.phones)
            assert.are.equal(1, #decoded.children)
            assert.are.equal("Bob", decoded.children[1].name)
            assert.are.equal(3, #decoded.tags)
            assert.are.equal(5, #decoded.numbers)
            assert.are.equal(3, #decoded.flags)
            assert.are.equal(3, #decoded.values)
        end)

        -- Test pack/unpack with reference data
        it("unpacks simple_struct_packed", function()
            local packed = read_file(testdata .. "simple_struct_packed.bin")
            local unpacked = sproto.unpack(packed)
            local decoded = sp:decode("Person", unpacked)

            assert.are.equal("Alice", decoded.name)
            assert.are.equal(13, decoded.age)
        end)

        it("unpacks struct_array_packed", function()
            local packed = read_file(testdata .. "struct_array_packed.bin")
            local unpacked = sproto.unpack(packed)
            local decoded = sp:decode("Person", unpacked)

            assert.are.equal("Bob", decoded.name)
            assert.are.equal(2, #decoded.children)
        end)
    end)

    describe("rpc schema", function()
        local sp

        before_each(function()
            sp = load_rpc_schema()
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
        -- schema.bin Person has all needed fields for round-trip
        sp = load_schema()
    end)

    it("Person with children round-trip", function()
        local original = {
            name = "Parent",
            age = 45,
            active = true,
            children = {
                {name = "Child1", age = 10},
                {name = "Child2", age = 8, active = false},
            }
        }
        local encoded = sp:encode("Person", original)
        local decoded = sp:decode("Person", encoded)

        assert.are.equal(original.name, decoded.name)
        assert.are.equal(original.age, decoded.age)
        assert.are.equal(original.active, decoded.active)
        assert.are.equal(#original.children, #decoded.children)
        assert.are.equal("Child1", decoded.children[1].name)
        assert.are.equal("Child2", decoded.children[2].name)
    end)

    it("Person with all fields round-trip", function()
        local original = {
            numbers = {100, 200, 300},
            flags = {true, false, true},
            age = 999999,
            id = (1 << 40),
            score = 3.14159265358979,
            values = {1.1, 2.2, 3.3}
        }
        local encoded = sp:encode("Person", original)
        local decoded = sp:decode("Person", encoded)

        assert.are.same(original.numbers, decoded.numbers)
        assert.are.same(original.flags, decoded.flags)
        assert.are.equal(original.age, decoded.age)
        assert.are.equal(original.id, decoded.id)
        assert.is_near(original.score, decoded.score, 0.0000001)
        for i = 1, 3 do
            assert.is_near(original.values[i], decoded.values[i], 0.0000001)
        end
    end)

    it("encode -> pack -> unpack -> decode round-trip", function()
        local original = {name = "Test", age = 100, active = true}
        local encoded = sp:encode("Person", original)
        local packed = sproto.pack(encoded)
        local unpacked = sproto.unpack(packed)
        local decoded = sp:decode("Person", unpacked)

        assert.are.equal(original.name, decoded.name)
        assert.are.equal(original.age, decoded.age)
        assert.are.equal(original.active, decoded.active)
    end)
end)
