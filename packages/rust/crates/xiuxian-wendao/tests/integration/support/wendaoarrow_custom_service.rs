use std::process::{Command, Stdio};

use super::wendaoarrow_common::{
    WendaoArrowServiceGuard, repo_root, reserve_test_port, wait_for_health, wendaoarrow_package_dir,
};

pub(crate) struct WendaoArrowScoreRow<'a> {
    pub(crate) doc_id: &'a str,
    pub(crate) analyzer_score: f64,
    pub(crate) final_score: f64,
}

pub(crate) async fn spawn_wendaoarrow_custom_scoring_service(
    rows: &[WendaoArrowScoreRow<'_>],
) -> (String, WendaoArrowServiceGuard) {
    let port = reserve_test_port();
    let base_url = format!("http://127.0.0.1:{port}");
    let package_dir = wendaoarrow_package_dir();
    let processor = processor_script(rows);

    let child = Command::new("julia")
        .arg(format!("--project={}", package_dir.display()))
        .arg("-e")
        .arg(processor)
        .arg(port.to_string())
        .current_dir(repo_root())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|error| panic!("spawn WendaoArrow custom scoring service: {error}"));
    let guard = WendaoArrowServiceGuard::new(child);

    wait_for_health(base_url.as_str()).await;
    (base_url, guard)
}

fn processor_script(rows: &[WendaoArrowScoreRow<'_>]) -> String {
    let mut mappings = String::new();
    for row in rows {
        mappings.push_str(
            format!(
                "\"{}\" => ({}, {}),\n",
                row.doc_id, row.analyzer_score, row.final_score
            )
            .as_str(),
        );
    }

    format!(
        r#"
using WendaoArrow
using Tables

const SCORE_MAP = Dict(
{mappings}
)

function processor(table)
    columns = Tables.columntable(table)
    analyzer_scores = Float64[]
    final_scores = Float64[]
    sizehint!(analyzer_scores, length(columns.doc_id))
    sizehint!(final_scores, length(columns.doc_id))

    for raw_doc_id in columns.doc_id
        doc_id = String(raw_doc_id)
        analyzer_score, final_score = get(SCORE_MAP, doc_id, (0.0, 0.0))
        push!(analyzer_scores, analyzer_score)
        push!(final_scores, final_score)
    end

    return (
        doc_id = collect(columns.doc_id),
        analyzer_score = analyzer_scores,
        final_score = final_scores,
    )
end

config = WendaoArrow.InterfaceConfig(host="127.0.0.1", port=parse(Int, ARGS[1]))
WendaoArrow.serve(processor; config=config)
"#
    )
}
