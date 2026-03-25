use std::env;
use std::net::SocketAddr;

use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

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

    let bot = Bot::from_env();

    Command::repl(bot, answer).await;
}

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?
        }
        Command::Wol => {
            let (mac, ip) = match get_wol_config(msg.chat.id) {
                Ok(config) => config,
                Err(e) => {
                    bot.send_message(msg.chat.id, format!("Configuration error: {}", e))
                        .await?;
                    return Ok(());
                }
            };

            match send_wol(&mac, &ip) {
                Ok(_) => {
                    bot.send_message(msg.chat.id, format!("WOL packet sent to {} ({})", mac, ip))
                        .await?
                }
                Err(e) => {
                    bot.send_message(msg.chat.id, format!("Failed to send WOL packet: {}", e))
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
        .map_err(|e| format!("Invalid MAC address: {}", e))?;
    let dst_addr = ip
        .parse::<SocketAddr>()
        .or_else(|_| {
            // If it's just an IP without port, default to port 9
            format!("{}:9", ip).parse::<SocketAddr>()
        })
        .map_err(|e| format!("Invalid IP address: {}", e))?;

    wol::send_magic_packet(mac_addr, None, dst_addr).map_err(|e| format!("WOL error: {}", e))?;
    Ok(())
}

fn get_wol_config(chat_id: ChatId) -> Result<(String, String), String> {
    let chat_var_name = format!("CHAT{}", chat_id);
    if let Ok(val) = env::var(&chat_var_name) {
        let parts: Vec<&str> = val.split(',').collect();
        if parts.len() == 2 {
            Ok((parts[0].trim().to_string(), parts[1].trim().to_string()))
        } else {
            Err(format!(
                "Invalid format for {}. Expected <MAC>,<IP>",
                chat_var_name
            ))
        }
    } else {
        Err(format!("No configuration found for chat {}", chat_id))
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    #[test]
    fn test_get_wol_config_for_unknown_chat() {
        unsafe {
            env::remove_var("CHAT123");
        }

        let res = get_wol_config(ChatId(123));
        assert!(matches!(res, Err(e) if e.contains("No configuration found for chat")));
    }

    #[test]
    fn test_get_wol_config_chat_specific() {
        unsafe {
            env::set_var("CHAT456", "AA:BB:CC:DD:EE:FF,10.0.0.255");
        }

        let (mac, ip) = get_wol_config(ChatId(456)).unwrap();
        assert_eq!(mac, "AA:BB:CC:DD:EE:FF");
        assert_eq!(ip, "10.0.0.255");
    }

    #[test]
    fn test_get_wol_config_chat_specific_with_spaces() {
        unsafe {
            env::set_var("CHAT789", " AA:BB:CC:DD:EE:FF , 10.0.0.1:9 ");
        }

        let (mac, ip) = get_wol_config(ChatId(789)).unwrap();
        assert_eq!(mac, "AA:BB:CC:DD:EE:FF");
        assert_eq!(ip, "10.0.0.1:9");
    }

    #[test]
    fn test_get_wol_config_invalid_format() {
        unsafe {
            env::set_var("CHAT101", "invalid_format");
        }

        let result = get_wol_config(ChatId(101));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Invalid format for CHAT101. Expected <MAC>,<IP>"
        );
    }
}
