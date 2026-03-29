from xiuxian_foundation.config.paths import ConfigPaths, get_config_paths


def test_get_config_paths_returns_project_directories():
    paths = get_config_paths()

    assert isinstance(paths, ConfigPaths)
    assert paths.project_root.name == "xiuxian-artisan-workshop"
    assert paths.config_home.name == ".config"
    assert paths.runtime_dir.name == ".run"
    assert paths.cache_home.name == ".cache"
    assert paths.data_home.name == ".data"
    assert paths.path_dir.name == ".bin"
    assert paths.get_log_dir() == paths.runtime_dir / "logs"


def test_get_config_paths_is_cached():
    assert get_config_paths() is get_config_paths()
