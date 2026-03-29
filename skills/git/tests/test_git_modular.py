from pathlib import Path


def test_git_rust_bridge_extension_surface_exists() -> None:
    """The retained git extension surface keeps the rust_bridge package importable."""
    base = Path(__file__).parent.parent / "extensions" / "rust_bridge"
    assert (base / "__init__.py").exists()
    assert (base / "accelerator.py").exists()
    assert (base / "bindings.py").exists()


def test_git_rust_bridge_bindings_default_to_unavailable() -> None:
    """Without a retained Python Rust bridge, bindings should degrade gracefully."""
    import sys

    sys.path.insert(0, str(Path(__file__).parent.parent))
    from extensions.rust_bridge.bindings import RustBindings

    RustBindings._instance = None
    RustBindings._checked = False
    RustBindings._available = False
    RustBindings._error_msg = None

    assert RustBindings.is_available() is False
    assert RustBindings.get_sniffer_cls() is None
    assert RustBindings.get_error_message() is not None
