//! Telegram runtime `/job` and `/jobs` command behavior tests.

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::mpsc;

use xiuxian_daochang::test_support::{push_telegram_background_completion, session_messages};
use xiuxian_daochang::{
    Channel, ChannelAttachment, ChannelMessage, JobCompletion, JobCompletionKind,
};

use super::{MockChannel, build_agent, build_job_manager, handle_inbound_message, inbound};

#[tokio::test]
async fn runtime_handle_inbound_job_status_not_found_reports_dashboard() -> Result<()> {
    let agent = build_agent().await?;
    let channel = Arc::new(MockChannel::default());
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let job_manager = build_job_manager(agent.clone());
    let (foreground_tx, mut foreground_rx) = mpsc::channel::<ChannelMessage>(8);

    assert!(
        handle_inbound_message(
            inbound("/job missing-123"),
            &channel_dyn,
            &foreground_tx,
            &job_manager,
            &agent,
        )
        .await
    );
    assert!(
        foreground_rx.try_recv().is_err(),
        "job status command should not forward to foreground queue"
    );

    let sent = channel.sent_messages().await;
    assert_eq!(sent.len(), 1);
    assert!(sent[0].0.contains("job-status dashboard"));
    assert!(sent[0].0.contains("status=not_found"));
    assert!(sent[0].0.contains("job_id=missing-123"));
    assert!(sent[0].0.contains("jobs_dashboard=/jobs"));
    Ok(())
}

#[tokio::test]
async fn runtime_handle_inbound_job_status_not_found_reports_json() -> Result<()> {
    let agent = build_agent().await?;
    let channel = Arc::new(MockChannel::default());
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let job_manager = build_job_manager(agent.clone());
    let (foreground_tx, mut foreground_rx) = mpsc::channel::<ChannelMessage>(8);

    assert!(
        handle_inbound_message(
            inbound("/job missing-123 json"),
            &channel_dyn,
            &foreground_tx,
            &job_manager,
            &agent,
        )
        .await
    );
    assert!(
        foreground_rx.try_recv().is_err(),
        "job status json command should not forward to foreground queue"
    );

    let sent = channel.sent_messages().await;
    assert_eq!(sent.len(), 1);
    let payload: serde_json::Value = serde_json::from_str(&sent[0].0)?;
    assert_eq!(payload["kind"], "job_status");
    assert_eq!(payload["found"], false);
    assert_eq!(payload["job_id"], "missing-123");
    assert_eq!(payload["status"], "not_found");
    Ok(())
}

#[tokio::test]
async fn runtime_handle_inbound_jobs_summary_reports_dashboard() -> Result<()> {
    let agent = build_agent().await?;
    let channel = Arc::new(MockChannel::default());
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let job_manager = build_job_manager(agent.clone());
    let (foreground_tx, mut foreground_rx) = mpsc::channel::<ChannelMessage>(8);

    assert!(
        handle_inbound_message(
            inbound("/jobs"),
            &channel_dyn,
            &foreground_tx,
            &job_manager,
            &agent,
        )
        .await
    );
    assert!(
        foreground_rx.try_recv().is_err(),
        "jobs summary command should not forward to foreground queue"
    );

    let sent = channel.sent_messages().await;
    assert_eq!(sent.len(), 1);
    assert!(sent[0].0.contains("jobs-health dashboard"));
    assert!(sent[0].0.contains("Overview:"));
    assert!(sent[0].0.contains("Health:"));
    assert!(sent[0].0.contains("state=healthy"));
    Ok(())
}

#[tokio::test]
async fn runtime_handle_inbound_jobs_summary_reports_json() -> Result<()> {
    let agent = build_agent().await?;
    let channel = Arc::new(MockChannel::default());
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let job_manager = build_job_manager(agent.clone());
    let (foreground_tx, mut foreground_rx) = mpsc::channel::<ChannelMessage>(8);

    assert!(
        handle_inbound_message(
            inbound("/jobs json"),
            &channel_dyn,
            &foreground_tx,
            &job_manager,
            &agent,
        )
        .await
    );
    assert!(
        foreground_rx.try_recv().is_err(),
        "jobs summary json command should not forward to foreground queue"
    );

    let sent = channel.sent_messages().await;
    assert_eq!(sent.len(), 1);
    let payload: serde_json::Value = serde_json::from_str(&sent[0].0)?;
    assert_eq!(payload["kind"], "jobs_health");
    assert_eq!(payload["health"], "healthy");
    assert_eq!(payload["total"], 0);
    assert_eq!(payload["queued"], 0);
    assert_eq!(payload["running"], 0);
    Ok(())
}

#[tokio::test]
async fn runtime_handle_inbound_image_message_auto_routes_to_background() -> Result<()> {
    let agent = build_agent().await?;
    let channel = Arc::new(MockChannel::default());
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let job_manager = build_job_manager(agent.clone());
    let (foreground_tx, mut foreground_rx) = mpsc::channel::<ChannelMessage>(8);

    let mut msg = inbound("extract text from image");
    msg.attachments = vec![ChannelAttachment::ImageUrl {
        url: "https://example.com/demo.png".to_string(),
    }];

    assert!(handle_inbound_message(msg, &channel_dyn, &foreground_tx, &job_manager, &agent).await);
    assert!(
        foreground_rx.try_recv().is_err(),
        "auto-routed image message should not forward to foreground queue"
    );

    let sent = channel.sent_messages().await;
    assert_eq!(sent.len(), 1);
    assert!(sent[0].0.contains("Queued background job"));
    assert!(sent[0].0.contains("Auto-routed image message"));

    let job_id = sent[0]
        .0
        .split('`')
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("missing job id in ack"))?;
    let status = job_manager
        .get_status(job_id)
        .await
        .ok_or_else(|| anyhow::anyhow!("missing job status for {job_id}"))?;
    assert!(status.prompt_preview.contains("extract text from image"));
    assert!(
        status
            .prompt_preview
            .contains("[IMAGE:https://example.com/demo.png]")
    );
    Ok(())
}

#[tokio::test]
async fn runtime_background_command_with_image_preserves_image_marker() -> Result<()> {
    let agent = build_agent().await?;
    let channel = Arc::new(MockChannel::default());
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let job_manager = build_job_manager(agent.clone());
    let (foreground_tx, mut foreground_rx) = mpsc::channel::<ChannelMessage>(8);

    let mut msg = inbound("/bg summarize this image");
    msg.attachments = vec![ChannelAttachment::ImageUrl {
        url: "https://example.com/chart.png".to_string(),
    }];

    assert!(handle_inbound_message(msg, &channel_dyn, &foreground_tx, &job_manager, &agent).await);
    assert!(
        foreground_rx.try_recv().is_err(),
        "/bg command should not forward to foreground queue"
    );

    let sent = channel.sent_messages().await;
    assert_eq!(sent.len(), 1);
    assert!(sent[0].0.contains("Queued background job"));
    assert!(!sent[0].0.contains("Auto-routed image message"));

    let job_id = sent[0]
        .0
        .split('`')
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("missing job id in ack"))?;
    let status = job_manager
        .get_status(job_id)
        .await
        .ok_or_else(|| anyhow::anyhow!("missing job status for {job_id}"))?;
    assert!(status.prompt_preview.contains("summarize this image"));
    assert!(
        status
            .prompt_preview
            .contains("[IMAGE:https://example.com/chart.png]")
    );
    Ok(())
}

#[tokio::test]
async fn runtime_background_completion_persists_into_parent_session_context() -> Result<()> {
    let agent = build_agent().await?;
    let channel = Arc::new(MockChannel::default());
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let session_id = "telegram:-200:888";
    let completion = JobCompletion {
        job_id: "job-test-1".to_string(),
        recipient: "-200".to_string(),
        parent_session_id: session_id.to_string(),
        kind: JobCompletionKind::Succeeded {
            output: "image OCR summary".to_string(),
        },
    };

    push_telegram_background_completion(&channel_dyn, &agent, completion).await;

    let sent = channel.sent_messages().await;
    assert_eq!(sent.len(), 1);
    assert!(sent[0].0.contains("Background job `job-test-1` completed."));
    assert!(sent[0].0.contains("image OCR summary"));

    let messages = session_messages(&agent, session_id).await?;
    assert_eq!(messages.len(), 2);
    assert_eq!(
        messages[0].content.as_deref(),
        Some("[background] job `job-test-1` completion")
    );
    let assistant = messages[1].content.as_deref().unwrap_or_default();
    assert!(assistant.contains("Background job `job-test-1` completed."));
    assert!(assistant.contains("image OCR summary"));
    Ok(())
}
