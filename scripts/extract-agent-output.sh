#!/bin/bash
# Extract the final assistant text content from a Claude Code agent JSONL file
# Usage: ./extract-agent-output.sh <agent-id>
# Example: ./extract-agent-output.sh aba6fe5

AGENT_ID="$1"
if [ -z "$AGENT_ID" ]; then
    echo "Usage: $0 <agent-id>"
    exit 1
fi

AGENT_FILE="$HOME/.claude/projects/-Users-jeffrey-code-tether/agent-${AGENT_ID}.jsonl"

if [ ! -f "$AGENT_FILE" ]; then
    echo "Error: Agent file not found: $AGENT_FILE"
    exit 1
fi

# Extract the last assistant message with text content
cat "$AGENT_FILE" | jq -sr 'map(select(.message.role == "assistant" and .message.content)) | last | .message.content[] | select(.type == "text") | .text' 2>/dev/null
