use anyhow::Result;
use tokio::sync::mpsc;

use crate::channels::telegram::idempotency::WebhookDedupConfig;
use crate::channels::telegram::session_partition::TelegramSessionPartition;
use crate::channels::telegram::{TelegramCommandAdminRule, TelegramControlCommandPolicy};
use crate::channels::traits::ChannelMessage;

use super::super::app::TelegramWebhookApp;
use super::core::{
    TelegramWebhookCoreBuildRequest,
    build_telegram_webhook_app_with_partition_and_control_command_policy,
};

pub struct TelegramWebhookControlPolicyBuildRequest {
    pub bot_token: String,
    pub allowed_users: Vec<String>,
    pub allowed_groups: Vec<String>,
    pub control_command_policy: TelegramControlCommandPolicy,
    pub webhook_path: String,
    pub secret_token: Option<String>,
    pub dedup_config: WebhookDedupConfig,
    pub tx: mpsc::Sender<ChannelMessage>,
}

pub struct TelegramWebhookPartitionBuildRequest {
    pub bot_token: String,
    pub allowed_users: Vec<String>,
    pub allowed_groups: Vec<String>,
    pub admin_users: Vec<String>,
    pub webhook_path: String,
    pub secret_token: Option<String>,
    pub dedup_config: WebhookDedupConfig,
    pub session_partition: TelegramSessionPartition,
    pub tx: mpsc::Sender<ChannelMessage>,
}

pub fn build_telegram_webhook_app(
    bot_token: String,
    allowed_users: Vec<String>,
    allowed_groups: Vec<String>,
    webhook_path: impl Into<String>,
    secret_token: Option<String>,
    dedup_config: WebhookDedupConfig,
    tx: mpsc::Sender<ChannelMessage>,
) -> Result<TelegramWebhookApp> {
    build_telegram_webhook_app_with_control_command_policy(
        TelegramWebhookControlPolicyBuildRequest {
            bot_token,
            allowed_users,
            allowed_groups,
            control_command_policy: TelegramControlCommandPolicy::new(
                Vec::new(),
                None,
                Vec::<TelegramCommandAdminRule>::new(),
            ),
            webhook_path: webhook_path.into(),
            secret_token,
            dedup_config,
            tx,
        },
    )
}

pub fn build_telegram_webhook_app_with_control_command_policy(
    request: TelegramWebhookControlPolicyBuildRequest,
) -> Result<TelegramWebhookApp> {
    let TelegramWebhookControlPolicyBuildRequest {
        bot_token,
        allowed_users,
        allowed_groups,
        control_command_policy,
        webhook_path,
        secret_token,
        dedup_config,
        tx,
    } = request;
    build_telegram_webhook_app_with_partition_and_control_command_policy(
        TelegramWebhookCoreBuildRequest {
            bot_token,
            allowed_users,
            allowed_groups,
            control_command_policy,
            webhook_path,
            secret_token,
            dedup_config,
            session_partition: TelegramSessionPartition::default(),
            tx,
        },
    )
}

pub fn build_telegram_webhook_app_with_partition(
    request: TelegramWebhookPartitionBuildRequest,
) -> Result<TelegramWebhookApp> {
    let TelegramWebhookPartitionBuildRequest {
        bot_token,
        allowed_users,
        allowed_groups,
        admin_users,
        webhook_path,
        secret_token,
        dedup_config,
        session_partition,
        tx,
    } = request;
    build_telegram_webhook_app_with_partition_and_control_command_policy(
        TelegramWebhookCoreBuildRequest {
            bot_token,
            allowed_users,
            allowed_groups,
            control_command_policy: TelegramControlCommandPolicy::new(
                admin_users,
                None,
                Vec::<TelegramCommandAdminRule>::new(),
            ),
            webhook_path,
            secret_token,
            dedup_config,
            session_partition,
            tx,
        },
    )
}
