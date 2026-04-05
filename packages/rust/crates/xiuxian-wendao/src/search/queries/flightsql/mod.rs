mod discovery;
mod metadata;
mod service;
mod statement;

pub use self::service::{StudioFlightSqlService, build_studio_flightsql_service};

#[cfg(test)]
mod tests;
