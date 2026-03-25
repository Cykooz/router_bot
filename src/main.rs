use std::collections::HashMap;
use std::env;
use std::net::Ipv4Addr;
use std::sync::Arc;

use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use tracing::metadata::LevelFilter;
use tracing::{error, info};
use wol::MacAddress;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let log_level = get_from_env("LOG_LEVEL", Some(LevelFilter::INFO));
    let with_ansi_color: bool = env::var("WITHOUT_ANSI_COLOR").is_err();

    use tracing_subscriber::fmt::Subscriber;
    use tracing_subscriber::util::SubscriberInitExt;
    let builder = Subscriber::builder()
        .with_max_level(log_level)
        .with_target(false)
        .with_ansi(with_ansi_color)
        .without_time();
    let subscriber = builder.finish();
    subscriber.try_init().unwrap();

    let config = Arc::new(Config::from_env());
    let num_of_chats = config.chats.len();
    info!("Starting router bot with {num_of_chats} configured chat(s)...");

    let bot = Bot::from_env();

    if let Err(e) = bot.set_my_commands(Command::bot_commands()).await {
        error!("Failed to set bot commands: {e}");
    }

    let handler = Update::filter_message()
        .filter_command::<Command>()
        .endpoint(answer);

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![config])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "send Wake-On-Lan packet.")]
    Wol,
}

async fn answer(bot: Bot, msg: Message, cmd: Command, config: Arc<Config>) -> ResponseResult<()> {
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?;
        }
        Command::Wol => execute_wal_command(bot, msg, config).await?,
    }
    Ok(())
}

async fn execute_wal_command(bot: Bot, msg: Message, config: Arc<Config>) -> ResponseResult<()> {
    let Some(&mac) = config.chats.get(&msg.chat.id) else {
        bot.send_message(
            msg.chat.id,
            format!("No configuration found for chat {}", msg.chat.id),
        )
        .await?;
        return Ok(());
    };

    let send_result = wol::send_magic_packet(mac, None, (Ipv4Addr::BROADCAST, 9).into());
    match send_result {
        Ok(_) => {
            bot.send_message(msg.chat.id, format!("WOL packet sent to {mac}"))
                .await?;
        }
        Err(e) => {
            bot.send_message(msg.chat.id, format!("Failed to send WOL packet: {e}"))
                .await?;
        }
    }

    Ok(())
}

fn get_from_env<T: std::str::FromStr>(name: &str, default: Option<T>) -> T {
    match env::var(name).ok() {
        Some(v) => v
            .parse()
            .unwrap_or_else(|_| panic!("Invalid value for environment variable {name}: {v}.")),
        None => default.unwrap_or_else(|| panic!("Environment variable {name} is not set.")),
    }
}

struct Config {
    chats: HashMap<ChatId, MacAddress>,
}

impl Config {
    fn from_env() -> Self {
        let mut chats = HashMap::new();
        for (key, value) in env::vars() {
            if let Some(chat_id_str) = key.strip_prefix("CHAT") {
                let chat_id = match chat_id_str.parse::<i64>() {
                    Ok(id) => ChatId(id),
                    Err(_) => {
                        error!("Invalid chat ID in environment variable {key}: {chat_id_str}");
                        continue;
                    }
                };

                let mac_addr = match value.parse::<MacAddress>() {
                    Ok(mac) => mac,
                    Err(e) => {
                        error!("Invalid MAC address '{value}': {e}");
                        continue;
                    }
                };
                chats.insert(chat_id, mac_addr);
            }
        }
        Self { chats }
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    #[test]
    fn test_config_from_env() {
        unsafe {
            env::set_var("CHAT12345", "00:11:22:33:44:55");
            env::set_var("CHATinvalid", "invalid");
            env::set_var("CHAT67890", "invalid_format");
        }

        let config = Config::from_env();
        assert_eq!(config.chats.len(), 1);
        let &mac_addr = config.chats.get(&ChatId(12345)).unwrap();
        assert_eq!(mac_addr, "00:11:22:33:44:55".parse().unwrap());

        unsafe {
            env::remove_var("CHAT12345");
            env::remove_var("CHATinvalid");
            env::remove_var("CHAT67890");
        }
    }
}
