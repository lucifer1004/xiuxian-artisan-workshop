use clap::ValueEnum;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum TelegramChannelMode {
    Polling,
    Webhook,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ChannelProvider {
    Telegram,
    Discord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DiscordRuntimeMode {
    Gateway,
    Ingress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum WebhookDedupBackendMode {
    Memory,
    Valkey,
}
