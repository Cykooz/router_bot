use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;

use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

#[derive(Clone, Debug)]
struct WolConfig {
    mac: String,
    ip: String,
}

struct Config {
    chats: HashMap<ChatId, WolConfig>,
}

impl Config {
    fn from_env() -> Self {
        let mut chats = HashMap::new();
        for (key, value) in env::vars() {
            if key.starts_with("CHAT") {
                let chat_id_str = &key[4..];
                let chat_id = match chat_id_str.parse::<i64>() {
                    Ok(id) => ChatId(id),
                    Err(_) => {
                        log::error!(
                            "Invalid chat ID in environment variable {}: {}",
                            key,
                            chat_id_str
                        );
                        continue;
                    }
                };

                match parse_wol_config(&value) {
                    Ok(config) => {
                        chats.insert(chat_id, config);
                    }
                    Err(e) => {
                        log::error!("Invalid configuration for {}: {}", key, e);
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
        Ok(WolConfig {
            mac: parts[0].trim().to_string(),
            ip: parts[1].trim().to_string(),
        })
    } else {
        Err("Expected <MAC>,<IP>".to_string())
    }
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

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    pretty_env_logger::init();

    log::info!("Starting router bot...");

    let config = Arc::new(Config::from_env());
    let bot = Bot::from_env();

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

async fn answer(bot: Bot, msg: Message, cmd: Command, config: Arc<Config>) -> ResponseResult<()> {
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?
        }
        Command::Wol => {
            let (mac, ip) = match config.chats.get(&msg.chat.id) {
                Some(config) => (config.mac.clone(), config.ip.clone()),
                None => {
                    bot.send_message(
                        msg.chat.id,
                        format!("No configuration found for chat {}", msg.chat.id),
                    )
                    .await?;
                    return Ok(());
                }
            };

            match send_wol(&mac, &ip) {
                Ok(_) => {
                    bot.send_message(msg.chat.id, format!("WOL packet sent to {mac} ({ip})"))
                        .await?
                }
                Err(e) => {
                    bot.send_message(msg.chat.id, format!("Failed to send WOL packet: {e}"))
                        .await?
                }
            }
        }
    };

    Ok(())
}

fn send_wol(mac: &str, ip: &str) -> Result<(), String> {
    let mac_addr = mac
        .parse::<wol::MacAddress>()
        .map_err(|e| format!("Invalid MAC address: {e}"))?;
    let dst_addr = ip
        .parse::<SocketAddr>()
        .or_else(|_| {
            // If it's just an IP without port, default to port 9
            format!("{ip}:9").parse::<SocketAddr>()
        })
        .map_err(|e| format!("Invalid IP address: {}", e))?;

    wol::send_magic_packet(mac_addr, None, dst_addr).map_err(|e| format!("WOL error: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    #[test]
    fn test_parse_wol_config_valid() {
        let config = parse_wol_config("AA:BB:CC:DD:EE:FF,10.0.0.255").unwrap();
        assert_eq!(config.mac, "AA:BB:CC:DD:EE:FF");
        assert_eq!(config.ip, "10.0.0.255");
    }

    #[test]
    fn test_parse_wol_config_with_spaces() {
        let config = parse_wol_config(" AA:BB:CC:DD:EE:FF , 10.0.0.1:9 ").unwrap();
        assert_eq!(config.mac, "AA:BB:CC:DD:EE:FF");
        assert_eq!(config.ip, "10.0.0.1:9");
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
        assert_eq!(wol.mac, "00:11:22:33:44:55");
        assert_eq!(wol.ip, "192.168.1.10");

        unsafe {
            env::remove_var("CHAT12345");
            env::remove_var("CHATinvalid");
            env::remove_var("CHAT67890");
        }
    }
}
