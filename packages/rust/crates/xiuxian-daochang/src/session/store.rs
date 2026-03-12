use super::*;

impl Agent {
    pub(in crate::agent) async fn handle_shortcuts(
        &self,
        session_id: &str,
        user_message_owned: &mut String,
        force_react: &mut bool,
        turn_id: u64,
    ) -> Result<Option<String>> {
        if !*force_react
            && let Some(shortcut) = parse_workflow_bridge_shortcut(user_message_owned.as_str())
        {
            let decision = omega::decide_for_shortcut(
                shortcut.mode,
                user_message_owned.as_str(),
                &shortcut.tool_name,
            );
            self.record_omega_decision(
                session_id,
                &decision,
                Some(shortcut.mode),
                Some(shortcut.tool_name.as_str()),
            );

            if decision.route == OmegaRoute::Graph {
                let shortcut_snapshot = self
                    .build_shortcut_injection_snapshot(
                        session_id,
                        turn_id,
                        user_message_owned.as_str(),
                    )
                    .await?;
                if let Some(snapshot) = &shortcut_snapshot {
                    self.record_injection_snapshot(session_id, snapshot);
                }
                let graph_plan =
                    graph::build_shortcut_plan(shortcut.mode, &decision, &shortcut.tool_name);
                self.record_graph_plan(session_id, &graph_plan);
                let arguments = injection::augment_shortcut_arguments(
                    shortcut.arguments.clone(),
                    shortcut_snapshot.as_ref(),
                    &decision,
                    shortcut.mode,
                    Some(&graph_plan),
                );

                let execution = self
                    .execute_graph_shortcut_plan(
                        session_id,
                        &decision,
                        &graph_plan,
                        graph::GraphPlanExecutionInput {
                            workflow_mode: shortcut.mode,
                            turn_id,
                            shortcut_user_message: user_message_owned.clone(),
                            bridge_arguments_with_metadata: arguments,
                            bridge_arguments_without_metadata: shortcut.arguments.clone(),
                            injection: shortcut_snapshot.as_ref().map(|snapshot| {
                                RouteTraceInjection {
                                    blocks_used: snapshot.blocks.len() as u64,
                                    chars_injected: snapshot.total_chars as u64,
                                    dropped_by_budget: snapshot.dropped_block_ids.len() as u64,
                                }
                            }),
                        },
                    )
                    .await;

                let completed = match execution {
                    Ok(graph::GraphPlanExecutionOutcome::Completed {
                        output,
                        tool_summary,
                    }) => Some((output, tool_summary)),
                    Ok(graph::GraphPlanExecutionOutcome::RouteToReact {
                        rewritten_user_message,
                        tool_summary: _tool_summary,
                    }) => {
                        *force_react = true;
                        *user_message_owned = rewritten_user_message;
                        None
                    }
                    Err(graph::GraphPlanExecutionError {
                        error,
                        tool_summary,
                    }) => {
                        let error_text = error.to_string();
                        let _ = self
                            .update_recall_feedback(
                                session_id,
                                user_message_owned.as_str(),
                                &error_text,
                                Some(&tool_summary),
                            )
                            .await;
                        self.reflect_turn_and_update_policy_hint(
                            session_id,
                            turn_id,
                            decision.route,
                            user_message_owned.as_str(),
                            &error_text,
                            "error",
                            tool_summary.attempted,
                        )
                        .await;
                        return Err(error);
                    }
                };
                if let Some((out, tool_summary)) = completed {
                    let _ = self
                        .update_recall_feedback(
                            session_id,
                            user_message_owned.as_str(),
                            &out,
                            Some(&tool_summary),
                        )
                        .await;
                    let effective_tool_count = tool_summary.attempted.max(1);
                    self.append_turn_to_session(
                        session_id,
                        user_message_owned.as_str(),
                        &out,
                        effective_tool_count,
                    )
                    .await?;
                    self.reflect_turn_and_update_policy_hint(
                        session_id,
                        turn_id,
                        decision.route,
                        user_message_owned.as_str(),
                        &out,
                        "completed",
                        tool_summary.attempted,
                    )
                    .await;
                    return Ok(Some(out));
                }
            } else {
                *force_react = true;
            }
        }

        let user_message = user_message_owned.as_str();
        if !*force_react && let Some(shortcut) = parse_crawl_shortcut(user_message) {
            let mut tool_summary = ToolExecutionSummary::default();
            let out = match self
                .call_mcp_tool_with_diagnostics(CRAWL_TOOL_NAME, Some(shortcut.to_arguments()))
                .await
            {
                Ok(output) => {
                    tool_summary.record_result(output.is_error);
                    output.text
                }
                Err(error) => {
                    tool_summary.record_transport_failure();
                    let error_text = error.to_string();
                    let _ = self
                        .update_recall_feedback(
                            session_id,
                            user_message,
                            &error_text,
                            Some(&tool_summary),
                        )
                        .await;
                    self.reflect_turn_and_update_policy_hint(
                        session_id,
                        turn_id,
                        OmegaRoute::React,
                        user_message,
                        &error_text,
                        "error",
                        tool_summary.attempted,
                    )
                    .await;
                    return Err(error);
                }
            };
            let _ = self
                .update_recall_feedback(session_id, user_message, &out, Some(&tool_summary))
                .await;
            self.append_turn_to_session(session_id, user_message, &out, 1)
                .await?;
            self.reflect_turn_and_update_policy_hint(
                session_id,
                turn_id,
                OmegaRoute::React,
                user_message,
                &out,
                "completed",
                tool_summary.attempted,
            )
            .await;
            return Ok(Some(out));
        }

        Ok(None)
    }
}
