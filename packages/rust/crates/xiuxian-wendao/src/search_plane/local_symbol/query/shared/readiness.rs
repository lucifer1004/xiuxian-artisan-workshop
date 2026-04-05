use crate::search_plane::{SearchCorpusKind, SearchPlaneService};

use super::LocalSymbolSearchError;

pub(crate) async fn prepare_local_symbol_read_tables(
    service: &SearchPlaneService,
) -> Result<Vec<String>, LocalSymbolSearchError> {
    let status = service
        .coordinator()
        .status_for(SearchCorpusKind::LocalSymbol);
    let Some(active_epoch) = status.active_epoch else {
        return Err(LocalSymbolSearchError::NotReady);
    };

    let table_names =
        service.local_epoch_table_names_for_reads(SearchCorpusKind::LocalSymbol, active_epoch);
    if table_names.is_empty() {
        return Ok(Vec::new());
    }
    for table_name in &table_names {
        let parquet_path =
            service.local_table_parquet_path(SearchCorpusKind::LocalSymbol, table_name.as_str());
        if !parquet_path.exists() {
            return Err(LocalSymbolSearchError::NotReady);
        }
        service
            .search_engine()
            .ensure_parquet_table_registered(table_name.as_str(), parquet_path.as_path(), &[])
            .await?;
    }

    Ok(table_names)
}
