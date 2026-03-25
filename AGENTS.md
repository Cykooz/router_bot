# Router Bot Agents Guide

This project is a Telegram bot designed to send **Wake-On-Lan (WOL)** packets to wake up devices in a network.

## Technical Overview

- **Language**: Rust (2024 edition)
- **Framework**: `teloxide` for Telegram Bot API
- **Networking**: `tokio` (asynchronous runtime) and `wol` (for magic packets)
- **Logging**: `tracing` for structured logging

## Features & Commands

The bot provides a simple interface to trigger WOL packets:

- `/help`: Displays the list of available commands and their descriptions.
- `/wol`: Sends a magic WOL packet to the pre-configured MAC address and IP address.

## Configuration

The bot relies on several environment variables for its operation:

- `TELOXIDE_TOKEN`: The API token for your Telegram bot (obtained from @BotFather).
- `CHAT<tg_chat_id>`: Per-chat configuration for WOL.
  The value should be in the format `<MAC>,<IP>` (e.g., `CHAT123456789=00:11:22:33:44:55,192.168.1.2`).
  Where:
    - `MAC`: The MAC address of the target device (e.g., `00:11:22:33:44:55`).
    - `IP`: The destination IP address or hostname for the magic packet.
      If no port is specified, it defaults to port 9 (e.g., `192.168.1.2` or `192.168.1.10:9`).
- `LOG_LEVEL`: The logging level for the bot (e.g., `debug`, `info`, `warn`, `error`). Defaults to `info`.
- `WITHOUT_ANSI_COLOR`: If set, disables ANSI color output in the logs.

## Development

The bot's entry point is in `src/main.rs`. It uses a REPL (Read-Eval-Print Loop) to handle incoming messages and
commands asynchronously. On startup, it automatically registers the available commands with the Telegram Bot API.

- **`answer` function**: The main command handler.
- **`execute_wal_command` function**: Responsible for parsing addresses and sending the magic packet using the `wol`
  crate.
