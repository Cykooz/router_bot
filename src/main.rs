use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
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

    info!("Starting router bot...");

    let config = Arc::new(Config::from_env());
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
    let Some(config) = config.chats.get(&msg.chat.id) else {
        bot.send_message(
            msg.chat.id,
            format!("No configuration found for chat {}", msg.chat.id),
        )
        .await?;
        return Ok(());
    };

    let (mac, ip) = (config.mac_addr, config.ip_addr);
    let send_result = wol::send_magic_packet(mac, None, ip);
    match send_result {
        Ok(_) => {
            let ip_port = ip.to_string();
            let ip = ip_port.split(':').next().unwrap_or(&ip_port);
            bot.send_message(msg.chat.id, format!("WOL packet sent to {mac} ({ip})"))
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

#[derive(Clone, Debug)]
struct WolConfig {
    mac_addr: MacAddress,
    ip_addr: SocketAddr,
}

impl WolConfig {
    pub fn new(mac: &str, ip: &str) -> Result<Self, String> {
        let mac_addr = mac
            .parse::<MacAddress>()
            .map_err(|e| format!("Invalid MAC address '{mac}': {e}"))?;
        let ip_addr = ip
            .parse::<SocketAddr>()
            .or_else(|_| {
                // If it's just an IP without port, default to port 9
                format!("{ip}:9").parse::<SocketAddr>()
            })
            .map_err(|e| format!("Invalid IP address '{ip}': {e}"))?;
        Ok(Self { mac_addr, ip_addr })
    }
}

struct Config {
    chats: HashMap<ChatId, WolConfig>,
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

                match parse_wol_config(&value) {
                    Ok(config) => {
                        chats.insert(chat_id, config);
                    }
                    Err(e) => {
                        error!("Invalid configuration for {key}: {e}");
                    }
                }
            }
        }
        Self { chats }
    }
}

fn parse_wol_config(val: &str) -> Result<WolConfig, String> {
    let parts: Vec<&str> = val.split(',').collect();
    if parts.len() == 2 {
        WolConfig::new(parts[0].trim(), parts[1].trim())
    } else {
        Err("Expected <MAC>,<IP>".to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    #[test]
    fn test_parse_wol_config_valid() {
        let config = parse_wol_config("AA:BB:CC:DD:EE:FF,10.0.0.255").unwrap();
        assert_eq!(config.mac_addr, "AA:BB:CC:DD:EE:FF".parse().unwrap());
        assert_eq!(config.ip_addr, "10.0.0.255:9".parse().unwrap());
    }

    #[test]
    fn test_parse_wol_config_with_spaces() {
        let config = parse_wol_config(" AA:BB:CC:DD:EE:FF , 10.0.0.1:9 ").unwrap();
        assert_eq!(config.mac_addr, "AA:BB:CC:DD:EE:FF".parse().unwrap());
        assert_eq!(config.ip_addr, "10.0.0.1:9".parse().unwrap());
    }

    #[test]
    fn test_parse_wol_config_invalid_format() {
        let result = parse_wol_config("invalid_format");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Expected <MAC>,<IP>");
    }

    #[test]
    fn test_config_from_env() {
        unsafe {
            env::set_var("CHAT12345", "00:11:22:33:44:55,192.168.1.10");
            env::set_var("CHATinvalid", "invalid");
            env::set_var("CHAT67890", "invalid_format");
        }

        let config = Config::from_env();
        assert_eq!(config.chats.len(), 1);
        let wol = config.chats.get(&ChatId(12345)).unwrap();
        assert_eq!(wol.mac_addr, "00:11:22:33:44:55".parse().unwrap());
        assert_eq!(wol.ip_addr, "192.168.1.10:9".parse().unwrap());

        unsafe {
            env::remove_var("CHAT12345");
            env::remove_var("CHATinvalid");
            env::remove_var("CHAT67890");
        }
    }
}
