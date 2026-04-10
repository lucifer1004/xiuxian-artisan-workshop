use super::*;

fn gap_matches_needle(gap: &serde_json::Map<String, Value>, needle: &str) -> bool {
    let title = gap.get("title").and_then(Value::as_str).unwrap_or_default();
    let page_id = gap
        .get("page_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    title.contains(needle) || page_id.contains(needle)
}

fn hit_gap_matches_needle(hit: &Value, needle: &str) -> bool {
    hit.get("gap")
        .and_then(Value::as_object)
        .is_some_and(|gap| gap_matches_needle(gap, needle))
}

fn group_preview_within_limit(group: &Value, limit: usize) -> bool {
    group
        .get("gaps")
        .and_then(Value::as_array)
        .is_some_and(|gaps| gaps.len() <= limit)
}

fn group_gaps_match_needle(group: &Value, needle: &str) -> bool {
    group
        .get("gaps")
        .and_then(Value::as_array)
        .is_some_and(|gaps| {
            gaps.iter().all(|gap| {
                gap.as_object()
                    .is_some_and(|gap| gap_matches_needle(gap, needle))
            })
        })
}

fn sum_u64_field(values: &[Value], field: &str) -> u64 {
    values
        .iter()
        .map(|value| value.get(field).and_then(Value::as_u64).unwrap_or_default())
        .sum()
}

fn selected_count_sum(values: &[Value]) -> Option<usize> {
    values.iter().try_fold(0usize, |acc, value| {
        let count = value.get("selected_count").and_then(Value::as_u64)?;
        let count = usize::try_from(count).ok()?;
        acc.checked_add(count)
    })
}

fn planner_rank_key(hit: &Value) -> (std::cmp::Reverse<i64>, String, String, String) {
    let gap = hit.get("gap").and_then(Value::as_object);
    (
        std::cmp::Reverse(
            hit.get("priority_score")
                .and_then(Value::as_i64)
                .unwrap_or_default(),
        ),
        gap.and_then(|gap| gap.get("kind"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        gap.and_then(|gap| gap.get("title"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        gap.and_then(|gap| gap.get("gap_id"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    )
}

fn modelica_nodocs_router(
    repo_id: &str,
) -> Result<(tempfile::TempDir, axum::Router), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_modelica_repo(temp.path(), "Projectionica")?;
    fs::write(
        repo_dir.join("Controllers").join("NoDocs.mo"),
        "within Projectionica.Controllers;\nmodel NoDocs\nend NoDocs;\n",
    )?;
    write_modelica_repo_config(temp.path(), &repo_dir, repo_id)?;
    let router = studio_router(gateway_state_for_project(temp.path()));
    Ok((temp, router))
}

#[tokio::test]
async fn docs_planner_search_endpoint_returns_gap_hits() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    fs::write(
        repo_dir.join("src").join("GatewaySyncPkg.jl"),
        "module GatewaySyncPkg\nexport solve\nsolve() = nothing\nend\n",
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("orphan.md"), "# orphan\n")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        "/api/docs/planner-search?repo=gateway-sync&query=orphan&page_kind=explanation&limit=5",
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_studio_json_snapshot("docs_planner_search_endpoint_json", payload);
    Ok(())
}

#[tokio::test]
async fn docs_planner_search_endpoint_executes_over_external_modelica_plugin_path() -> TestResult {
    let (_temp_dir, router) = modelica_nodocs_router("modelica-gateway-sync")?;

    let (status, payload) = request_json(
        router,
        "/api/docs/planner-search?repo=modelica-gateway-sync&query=NoDocs&limit=4",
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    let hits = payload
        .get("hits")
        .and_then(Value::as_array)
        .ok_or("planner-search payload should include a hits array")?;
    assert!(
        !hits.is_empty(),
        "planner-search endpoint should return at least one gap hit"
    );
    assert!(
        hits.len() <= 4,
        "planner-search endpoint should honor the configured hit limit"
    );
    assert!(
        hits.iter().all(|hit| hit_gap_matches_needle(hit, "NoDocs")),
        "planner-search endpoint hits should stay anchored to the injected no-doc target"
    );
    assert_studio_json_snapshot("docs_planner_search_endpoint_modelica_json", payload);
    Ok(())
}

#[tokio::test]
async fn docs_planner_queue_endpoint_returns_grouped_gap_backlog() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    fs::write(
        repo_dir.join("src").join("GatewaySyncPkg.jl"),
        "module GatewaySyncPkg\nexport solve, explain\nsolve() = nothing\nexplain() = nothing\nend\n",
    )?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::write(
        repo_dir.join("examples").join("orphan_demo.jl"),
        "println(\"detached example\")\n",
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("orphan.md"), "# orphan\n")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        "/api/docs/planner-queue?repo=gateway-sync&per_kind_limit=2",
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_studio_json_snapshot("docs_planner_queue_endpoint_json", payload);
    Ok(())
}

#[tokio::test]
async fn docs_planner_queue_endpoint_executes_over_external_modelica_plugin_path() -> TestResult {
    let (_temp_dir, router) = modelica_nodocs_router("modelica-gateway-queue")?;

    let (status, payload) = request_json(
        router,
        "/api/docs/planner-queue?repo=modelica-gateway-queue&gap_kind=symbol_reference_without_documentation&page_kind=reference&per_kind_limit=3",
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    let groups = payload
        .get("groups")
        .and_then(Value::as_array)
        .ok_or("planner-queue payload should include a groups array")?;
    let total_gap_count = payload
        .get("total_gap_count")
        .and_then(Value::as_u64)
        .ok_or("planner-queue payload should include total_gap_count")?;

    assert!(
        !groups.is_empty(),
        "planner-queue endpoint should return at least one grouped backlog lane"
    );
    assert_eq!(
        total_gap_count,
        sum_u64_field(groups, "count"),
        "planner-queue total should match grouped counts"
    );
    assert!(
        groups
            .iter()
            .all(|group| group_preview_within_limit(group, 3)),
        "planner-queue previews should honor per-kind truncation"
    );
    assert!(
        groups
            .iter()
            .all(|group| group_gaps_match_needle(group, "NoDocs")),
        "planner-queue endpoint gaps should stay anchored to the injected no-doc target"
    );
    assert_studio_json_snapshot("docs_planner_queue_endpoint_modelica_json", payload);
    Ok(())
}

#[tokio::test]
async fn docs_planner_rank_endpoint_returns_priority_sorted_gaps() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    fs::write(
        repo_dir.join("src").join("GatewaySyncPkg.jl"),
        "module GatewaySyncPkg\nexport solve, explain\nsolve() = nothing\nexplain() = nothing\nend\n",
    )?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::write(
        repo_dir.join("examples").join("orphan_demo.jl"),
        "println(\"detached example\")\n",
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("orphan.md"), "# orphan\n")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) =
        request_json(router, "/api/docs/planner-rank?repo=gateway-sync&limit=4").await?;
    assert_eq!(status, StatusCode::OK);
    assert_studio_json_snapshot("docs_planner_rank_endpoint_json", payload);
    Ok(())
}

#[tokio::test]
async fn docs_planner_rank_endpoint_executes_over_external_modelica_plugin_path() -> TestResult {
    let (_temp_dir, router) = modelica_nodocs_router("modelica-gateway-rank")?;

    let (status, payload) = request_json(
        router,
        "/api/docs/planner-rank?repo=modelica-gateway-rank&gap_kind=symbol_reference_without_documentation&page_kind=reference&limit=4",
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    let hits = payload
        .get("hits")
        .and_then(Value::as_array)
        .ok_or("planner-rank payload should include a hits array")?;
    assert!(
        !hits.is_empty(),
        "planner-rank endpoint should return at least one ranked gap hit"
    );
    assert!(
        hits.len() <= 4,
        "planner-rank endpoint should honor the configured hit limit"
    );
    assert!(
        hits.iter().all(|hit| {
            hit.get("reasons")
                .and_then(Value::as_array)
                .is_some_and(|reasons| !reasons.is_empty())
        }),
        "planner-rank endpoint should keep deterministic score explanations"
    );
    assert!(
        hits.iter().all(|hit| hit_gap_matches_needle(hit, "NoDocs")),
        "planner-rank endpoint hits should stay anchored to the injected no-doc target"
    );
    assert!(
        hits.windows(2)
            .all(|window| planner_rank_key(&window[0]) <= planner_rank_key(&window[1])),
        "planner-rank endpoint hits should stay in deterministic priority order"
    );
    assert_studio_json_snapshot("docs_planner_rank_endpoint_modelica_json", payload);
    Ok(())
}

#[tokio::test]
async fn docs_planner_workset_endpoint_returns_opened_gap_batch() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    fs::write(
        repo_dir.join("src").join("GatewaySyncPkg.jl"),
        "module GatewaySyncPkg\nexport solve, explain\nsolve() = nothing\nexplain() = nothing\nend\n",
    )?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::write(
        repo_dir.join("examples").join("orphan_demo.jl"),
        "println(\"detached example\")\n",
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("orphan.md"), "# orphan\n")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        "/api/docs/planner-workset?repo=gateway-sync&per_kind_limit=2&limit=2&family_limit=2",
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_studio_json_snapshot("docs_planner_workset_endpoint_json", payload);
    Ok(())
}

#[tokio::test]
async fn docs_planner_workset_endpoint_executes_over_external_modelica_plugin_path() -> TestResult {
    let (_temp_dir, router) = modelica_nodocs_router("modelica-gateway-workset")?;

    let (status, payload) = request_json(
        router,
        "/api/docs/planner-workset?repo=modelica-gateway-workset&gap_kind=symbol_reference_without_documentation&page_kind=reference&per_kind_limit=3&limit=4&family_kind=how_to&related_limit=3&family_limit=3",
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    let items = payload
        .get("items")
        .and_then(Value::as_array)
        .ok_or("planner-workset payload should include an items array")?;
    let ranked_hits = payload
        .get("ranked_hits")
        .and_then(Value::as_array)
        .ok_or("planner-workset payload should include a ranked_hits array")?;
    let queue = payload
        .get("queue")
        .and_then(Value::as_object)
        .ok_or("planner-workset payload should include a queue object")?;
    let queue_groups = queue
        .get("groups")
        .and_then(Value::as_array)
        .ok_or("planner-workset payload should include queue.groups")?;
    let total_gap_count = queue
        .get("total_gap_count")
        .and_then(Value::as_u64)
        .ok_or("planner-workset payload should include queue.total_gap_count")?;
    let groups = payload
        .get("groups")
        .and_then(Value::as_array)
        .ok_or("planner-workset payload should include groups")?;

    assert!(
        !items.is_empty(),
        "planner-workset endpoint should select at least one Modelica workset item"
    );
    assert_eq!(
        items.len(),
        ranked_hits.len(),
        "planner-workset endpoint should reopen every ranked hit into one item"
    );
    assert!(
        items.len() <= 4,
        "planner-workset endpoint should honor the ranked-hit limit"
    );
    assert_eq!(
        total_gap_count,
        sum_u64_field(queue_groups, "count"),
        "planner-workset queue total should match grouped counts"
    );
    assert_eq!(
        selected_count_sum(groups),
        Some(items.len()),
        "planner-workset grouped selected counts should match opened items"
    );
    assert!(
        items
            .iter()
            .all(|item| hit_gap_matches_needle(item, "NoDocs")),
        "planner-workset endpoint items should stay anchored to the injected no-doc target"
    );
    assert_studio_json_snapshot("docs_planner_workset_endpoint_modelica_json", payload);
    Ok(())
}
