use std::sync::LazyLock;

use arrow_flight::sql::SqlInfo;
use arrow_flight::sql::metadata::{SqlInfoData, SqlInfoDataBuilder};

pub(super) static STUDIO_FLIGHT_SQL_INFO_DATA: LazyLock<SqlInfoData> = LazyLock::new(|| {
    let mut builder = SqlInfoDataBuilder::new();
    builder.append(SqlInfo::FlightSqlServerName, "Wendao FlightSQL Server");
    builder.append(SqlInfo::FlightSqlServerVersion, env!("CARGO_PKG_VERSION"));
    builder.append(SqlInfo::FlightSqlServerArrowVersion, "1.3");
    builder.append(SqlInfo::SqlIdentifierQuoteChar, "\"");
    builder
        .build()
        .unwrap_or_else(|error| panic!("build Wendao FlightSQL sql_info payload: {error}"))
});
