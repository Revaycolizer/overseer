#!/usr/bin/env bash
#
# Overseer REST API Client — single-file Bash implementation using curl + jq.
#
# Usage:
#   export OVESEER_BASE_URL=http://localhost:8080
#   export OVESEER_TOKEN=your-token-here
#   ./scripts/client.sh conversations list
#   ./scripts/client.sh agents list
#   ./scripts/client.sh health
#
# Or with inline auth:
#   ./scripts/client.sh --token "$TOKEN" conversations list
#
set -euo pipefail

BASE_URL="${OVERSEER_BASE_URL:-${OVESEER_BASE_URL:-http://localhost:8080}}"
TOKEN="${OVERSEER_API_TOKEN:-${OVESEER_TOKEN:-}}"

show_help() {
    cat <<EOF
Overseer API client

Usage:
  $0 [--base-url URL] [--token TOKEN] <command> [args...]

Commands:
  conversations list                          List conversations
  conversations create --title <title>        Create a conversation
  messages list --conversation-id <id>        List messages
  agents list                                 List agents
  tools list                                  List tools
  health                                      Check server health
  orchestrate --conversation-id <id> --query <q>  Send query
  models                                      List available models

Environment:
  OVERSEER_BASE_URL Server base URL (default: http://localhost:8080)
  OVERSEER_API_TOKEN Bearer token for authentication
EOF
}

# ── Parse CLI arguments ──────────────────────────────────────────────
POSITIONAL=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --base-url) BASE_URL="$2"; shift 2 ;;
        --token)    TOKEN="$2";    shift 2 ;;
        -h|--help)  show_help; exit 0 ;;
        *)          POSITIONAL+=("$1"); shift ;;
    esac
done
set -- "${POSITIONAL[@]}"

# ── HTTP helpers ─────────────────────────────────────────────────────
curl_() {
    local method="$1" path="$2" body="${3:-}"
    local -a args=(-sS --fail -X "$method" "${BASE_URL}${path}")
    if [[ -n "$TOKEN" ]]; then
        args+=(-H "Authorization: Bearer ${TOKEN}")
    fi
    if [[ -n "$body" ]]; then
        args+=(-H "Content-Type: application/json" -d "$body")
    fi
    curl "${args[@]}"
}

get() {
    curl_ GET "$1"
}

post() {
    curl_ POST "$1" "$2"
}

# ── Commands ─────────────────────────────────────────────────────────
CMD="${1:-help}"
shift || true

case "$CMD" in
    conversations)
        ACTION="${1:-list}"
        shift || true
        case "$ACTION" in
            list)
                get "/api/conversations?limit=50"
                ;;
            create)
                TITLE="${1:-}"
                if [[ -z "$TITLE" ]]; then
                    echo "Usage: $0 conversations create <title>" >&2
                    exit 1
                fi
                post "/api/conversations" "{\"title\": \"$TITLE\"}"
                ;;
            delete)
                ID="${1:-}"
                if [[ -z "$ID" ]]; then
                    echo "Usage: $0 conversations delete <id>" >&2
                    exit 1
                fi
                post "/api/conversations/delete" "{\"id\": $ID}"
                ;;
            *)
                echo "Unknown action: $ACTION" >&2
                exit 1
                ;;
        esac
        ;;

    messages)
        ACTION="${1:-list}"
        shift || true
        case "$ACTION" in
            list)
                CID="${1:-}"
                if [[ -z "$CID" ]]; then
                    echo "Usage: $0 messages list <conversation-id>" >&2
                    exit 1
                fi
                get "/api/messages?conversation_id=$CID"
                ;;
            send)
                CID="${1:-}"
                CONTENT="${2:-}"
                if [[ -z "$CID" || -z "$CONTENT" ]]; then
                    echo "Usage: $0 messages send <conversation-id> <content>" >&2
                    exit 1
                fi
                post "/api/messages" "{\"conversation_id\": $CID, \"role\": \"user\", \"content\": \"$CONTENT\", \"hide_evidence\": false}"
                ;;
            *)
                echo "Unknown action: $ACTION" >&2
                exit 1
                ;;
        esac
        ;;

    agents)
        get "/api/agents"
        ;;

    tools)
        get "/api/tools"
        ;;

    health)
        get "/health"
        ;;

    orchestrate)
        CID="${1:-}"
        QUERY="${2:-}"
        if [[ -z "$CID" || -z "$QUERY" ]]; then
            echo "Usage: $0 orchestrate <conversation-id> <query>" >&2
            exit 1
        fi
        post "/api/orchestrate" "{\"conversation_id\": $CID, \"query\": \"$QUERY\"}"
        ;;

    models)
        get "/api/models/available"
        ;;

    help|--help|-h)
        show_help
        ;;

    *)
        echo "Unknown command: $CMD" >&2
        show_help >&2
        exit 1
        ;;
esac

# Pretty-print JSON if jq is available
if command -v jq &>/dev/null; then
    # Re-read output piped through jq for pretty printing
    true
fi
