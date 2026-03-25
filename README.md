# Router Bot

A simple Telegram bot written in Rust that sends **Wake-On-Lan (WOL)** magic packets to wake up devices in your network.

## Features

- **WOL over Telegram**: Trigger magic packets from anywhere using a simple `/wol` command.
- **Per-Chat Configuration**: Configure different MAC and IP addresses for different Telegram chats.
- **Easy Setup**: Minimal configuration using environment variables.

## Prerequisites

- A Telegram Bot Token (from [@BotFather](https://t.me/BotFather)).
- Rust (2024 edition or later).
- The target device must be configured to wake up on magic packets (WOL enabled in BIOS/UEFI and OS settings).

## Configuration

The bot is configured via environment variables.

| Variable             | Description                                       | Example                                       |
|----------------------|---------------------------------------------------|-----------------------------------------------|
| `TELOXIDE_TOKEN`     | Your Telegram Bot API token.                      | `123456789:ABCdefGHIjklMNOpqrsTUVwxyZ`        |
| `CHAT<id>`           | Per-chat WOL configuration: `<MAC>,<IP>[:PORT]`.  | `CHAT123456789=00:11:22:33:44:55,192.168.1.2` |
| `LOG_LEVEL`          | Logging level (`debug`, `info`, `warn`, `error`). | `info`                                        |
| `WITHOUT_ANSI_COLOR` | Disable ANSI colors in logs if set.               | `1`                                           |

### Chat-specific Configuration Example

If your Telegram Chat ID is `123456789`, you would set:
`CHAT123456789=00:11:22:33:44:55,192.168.1.2`

The bot defaults to port **9** if no port is specified in the IP address.

You can specify configuration for multiple chats.

## Usage

### Running the Bot

1. Clone the repository.
2. Set the environment variables.
3. Run the bot using Cargo:

```bash
cargo run --release
```

### Bot Commands

Once the bot is running, you can use the following commands in Telegram:

- `/help` - Displays the available commands.
- `/wol` - Sends the magic packet to the device configured for the current chat.

## Example Scenario

1. You have a PC at home with MAC address `00:11:22:33:44:55` and IP address `192.168.1.2`.
2. You find your Telegram Chat ID (e.g., by using a bot like `@userinfobot`).
3. You configure the environment:
   ```env
   TELOXIDE_TOKEN=your_token_here
   CHAT123456789=00:11:22:33:44:55,192.168.1.2
   ```
4. You start the bot.
5. In your Telegram chat, you type `/wol`.
6. The bot sends the magic packet and replies: `WOL packet sent to 00:11:22:33:44:55 (192.168.1.2)`.
