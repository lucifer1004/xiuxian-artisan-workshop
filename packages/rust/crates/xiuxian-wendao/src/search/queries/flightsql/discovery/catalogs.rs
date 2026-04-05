use arrow::datatypes::SchemaRef;
use arrow::record_batch::RecordBatch;
use arrow_flight::sql::CommandGetCatalogs;
use tonic::Status;

pub(in super::super) const WENDAO_FLIGHTSQL_CATALOG_NAME: &str = "wendao";

pub(in super::super) fn build_catalogs_flight_info_schema(query: CommandGetCatalogs) -> SchemaRef {
    query.into_builder().schema()
}

pub(in super::super) fn build_catalogs_batch(
    query: CommandGetCatalogs,
) -> Result<RecordBatch, Status> {
    let mut builder = query.into_builder();
    builder.append(WENDAO_FLIGHTSQL_CATALOG_NAME);
    builder.build().map_err(|error| {
        Status::internal(format!(
            "FlightSQL failed to build catalogs discovery batch: {error}"
        ))
    })
}
