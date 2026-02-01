# SSH LLM Chat Server

SSH-accessible AI chat server powered by any OpenAI-compatible LLM API.

## Installation

```bash
cargo build --release
```

## Usage

```bash
# With custom settings
./target/release/sshllm \
    --port 2222 \
    --endpoint "http://localhost:8080/v1" \
    --model "default" \
    --api-key "optional" \
    --system-prompt "You are a helpful AI assistant. Be concise and friendly."

# Full options
./target/release/sshllm --help
```

Then connect:
```bash
ssh -p 2222 localhost
# or connect without storing host keys or specifying identity (auto-recognizes keys)
ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -p 2222 localhost
```

## Configuration

Environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `SSHLLM_PORT` | `2222` | SSH server port |
| `SSHLLM_API_URL` | - | OpenAI-compatible API URL |
| `SSHLLM_API_KEY` | - | API key (optional for local LLMs) |
| `SSHLLM_MODEL` | `default` | Model to use |
| `SSHLLM_SYSTEM_PROMPT` | `You are a helpful AI assistant. Be concise and friendly.` | Custom system prompt |
| `SSHLLM_LOGS_DIR` | `logs` | Directory for chat logs |
| `SSHLLM_HOST_KEY` | `keys/host_ed25519` | Path to persistent host key |

## Logging Structure

Users are identified by their SSH key fingerprint. If no key is provided, the server falls back to the client IP address.

```
logs/
├── key_abc123def456/         # Identity via SSH key fingerprint
│   ├── summary.txt           # User info (name, sessions, etc.)
│   └── chat_2026-02-01.log   # Daily chat log
└── 127.0.0.1/               # Identity via IP fallback
    ├── summary.txt
    └── chat_2026-02-01.log
```

## Commands

| Command | Description |
|---------|-------------|
| `/name <name>` | Set your name |
| `/clear` | Clear chat history |
| `/help` | Show available commands |
| `/quit` | Exit the chat |

## Features

- **Immediate feedback** - Real-time thinking indicator shows you when the AI is processing.
- **Robust Identity** - Recognizes users primarily by SSH public key fingerprints.
- **IP Fallback** - Seamlessly functions via IP-based folders for users without SSH keys.
- **Chat history** - Automatic daily chat logs with structured metadata.
- **Context awareness** - Automatically loads recent daily context upon reconnection.
- **Persistent Host Key** - Automatically generates and saves server identity on first run.
- **Standard SSH** - No specialized client required; works with any terminal SSH client.
