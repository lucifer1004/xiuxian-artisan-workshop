"""Unit tests for schema extraction helper surfaces."""


class TestInputSchemaExtractionFramework:
    """Tests for `_generate_tool_schema` using direct function signatures."""

    def test_input_schema_basic_types(self):
        from xiuxian_foundation.api.decorators import _generate_tool_schema

        def basic_func(message: str, count: int):
            pass

        schema = _generate_tool_schema(basic_func)
        assert schema["type"] == "object"
        assert "message" in schema["properties"]
        assert "count" in schema["properties"]
        assert "message" in schema["required"]
        assert "count" in schema["required"]

    def test_input_schema_with_defaults(self):
        from xiuxian_foundation.api.decorators import _generate_tool_schema

        def func_with_default(message: str, count: int = 10):
            pass

        schema = _generate_tool_schema(func_with_default)
        assert "message" in schema["required"]
        assert "count" not in schema["required"]

    def test_input_schema_with_optional_params(self):
        from xiuxian_foundation.api.decorators import _generate_tool_schema

        def func_with_optional(path: str, encoding: str = "utf-8"):
            pass

        schema = _generate_tool_schema(func_with_optional)
        assert "path" in schema["required"]
        assert "encoding" not in schema["required"]

    def test_input_schema_complex_types(self):
        from xiuxian_foundation.api.decorators import _generate_tool_schema

        def complex_func(name: str, tags: list[str], count: int, enabled: bool = True):
            pass

        schema = _generate_tool_schema(complex_func)
        assert schema["properties"]["name"]["type"] == "string"
        assert schema["properties"]["tags"]["type"] == "array"
        assert schema["properties"]["count"]["type"] == "integer"
        assert schema["properties"]["enabled"]["type"] == "boolean"
