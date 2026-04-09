use std::fs;

use duckdb::Connection;

use super::runtime::resolve_enabled_search_duckdb_runtime;
use crate::duckdb::{DuckDbDatabasePath, SearchDuckDbRuntimeConfig};

/// Feature-gated host-owned `DuckDB` connection wrapper for bounded analytics.
pub struct SearchDuckDbConnection {
    connection: Connection,
    runtime: SearchDuckDbRuntimeConfig,
}

impl SearchDuckDbConnection {
    /// Open a configured bounded `DuckDB` connection from merged Wendao settings.
    ///
    /// # Errors
    ///
    /// Returns an error when the runtime is disabled or when the connection
    /// cannot be opened and initialized.
    pub fn configured() -> Result<Self, String> {
        let runtime = resolve_enabled_search_duckdb_runtime("configured search DuckDB connection")?;
        Self::from_runtime(runtime)
    }

    /// Open a bounded `DuckDB` connection from one resolved runtime config.
    ///
    /// # Errors
    ///
    /// Returns an error when the connection cannot be opened and initialized.
    pub fn from_runtime(runtime: SearchDuckDbRuntimeConfig) -> Result<Self, String> {
        let connection = open_search_duckdb_connection(&runtime)?;
        Ok(Self {
            connection,
            runtime,
        })
    }

    /// Access the underlying `DuckDB` connection.
    #[must_use]
    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    /// Access the runtime config used to open this connection.
    #[must_use]
    pub fn runtime(&self) -> &SearchDuckDbRuntimeConfig {
        &self.runtime
    }
}

/// Open one bounded `DuckDB` connection from a resolved runtime config.
///
/// # Errors
///
/// Returns an error when the runtime is disabled, when required directories
/// cannot be created, or when `DuckDB` rejects the initialization pragmas.
pub fn open_search_duckdb_connection(
    runtime: &SearchDuckDbRuntimeConfig,
) -> Result<Connection, String> {
    if !runtime.enabled {
        return Err("search DuckDB runtime is disabled".to_string());
    }

    match &runtime.database_path {
        DuckDbDatabasePath::InMemory => {}
        DuckDbDatabasePath::File(path) => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    format!(
                        "failed to create search DuckDB database directory `{}`: {error}",
                        parent.display()
                    )
                })?;
            }
        }
    }

    fs::create_dir_all(&runtime.temp_directory).map_err(|error| {
        format!(
            "failed to create search DuckDB temp directory `{}`: {error}",
            runtime.temp_directory.display()
        )
    })?;

    let connection = match &runtime.database_path {
        DuckDbDatabasePath::InMemory => Connection::open_in_memory().map_err(|error| {
            format!("failed to open in-memory search DuckDB connection: {error}")
        })?,
        DuckDbDatabasePath::File(path) => Connection::open(path).map_err(|error| {
            format!(
                "failed to open search DuckDB database `{}`: {error}",
                path.display()
            )
        })?,
    };

    let escaped_temp_directory = runtime.temp_directory.to_string_lossy().replace('\'', "''");
    connection
        .execute_batch(&format!(
            "PRAGMA temp_directory='{escaped_temp_directory}';\nPRAGMA threads={};\nPRAGMA enable_profiling='no_output';\nPRAGMA profiling_mode='standard';",
            runtime.threads
        ))
        .map_err(|error| format!("failed to initialize search DuckDB pragmas: {error}"))?;

    Ok(connection)
}
