# SSH LLM Chat Server

SSH-accessible AI chat server powered by any OpenAI-compatible LLM API.

## Quick Start

```bash
cargo run
```

Then connect:
```bash
ssh -p 2222 localhost
# or for login without adding host key to known_hosts and user id
ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -p 2222 localhost
```

## Configuration

Environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `SSHLLM_PORT` | `2222` | SSH server port |
| `SSHLLM_API_URL` | `http://[IP_ADDRESS]:[PORT]/v1` | OpenAI-compatible API URL |
| `SSHLLM_API_KEY` | - | API key (optional for local LLMs) |
| `SSHLLM_MODEL` | `default` | Model to use |
| `SSHLLM_SYSTEM_PROMPT` | `You are a helpful AI assistant. Be concise and friendly.` | Custom system prompt |
| `SSHLLM_LOGS_DIR` | `logs` | Directory for chat logs |
| `SSHLLM_HOST_KEY` | - | Path to persist host key |

## Logging Structure

```
logs/
└── 192.168.1.100/
    ├── summary.txt           # User info (name, sessions, etc.)
    ├── chat_2026-02-01.log   # Daily chat log
    └── chat_2026-02-02.log
```

## Commands

| Command | Description |
|---------|-------------|
| `/name <name>` | Set your name |
| `/clear` | Clear chat history |
| `/help` | Show available commands |
| `/quit` | Exit the chat |

## Features

- **Immediate feedback** - Real-time thinking indicator shows you when the AI is processing
- **Persistent identity** - Recognizes users by IP and remembers their name across sessions
- **Chat history** - Automatic daily chat logs with timestamps
- **Context awareness** - Maintains conversation memory within the active session
- **Simple access** - Standard SSH terminal access with no specialized client required
