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
            let mac = env::var("WOL_MAC").expect("WOL_MAC must be set");
            let ip = env::var("WOL_IP").expect("WOL_IP must be set");

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
