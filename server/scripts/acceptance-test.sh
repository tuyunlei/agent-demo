#!/usr/bin/env bash
set -euo pipefail

# 配置（可通过环境变量覆盖）
HOST="${ACCEPTANCE_HOST:-127.0.0.1:50051}"
PROTO_DIR="${PROTO_DIR:-$(dirname "$0")/../../proto}"
EMAIL="acceptance-test-$(date +%s)@test.local"
PASSWORD="test-password-$(date +%s)"

# 颜色输出
GREEN=$'\033[0;32m'
RED=$'\033[0;31m'
NC=$'\033[0m'

TOTAL=6
PASSED=0

pass() {
  PASSED=$((PASSED + 1))
  echo -e "${GREEN}✓ $1${NC}"
}

fail() {
  echo -e "${RED}✗ $1${NC}"
  echo ""
  echo "Summary: ${PASSED}/${TOTAL} tests passed"
  exit 1
}

json_get() {
  local json="$1"
  local key="$2"
  python3 -c 'import json,sys; d=json.loads(sys.argv[1]); v=d
for k in sys.argv[2].split("."):
    if isinstance(v, dict) and k in v: v=v[k]
    else: print(""); raise SystemExit(0)
print(v if v is not None else "")' "$json" "$key"
}

grpc_call() {
  local proto="$1"
  local method="$2"
  local data="$3"
  shift 3
  grpcurl -plaintext -import-path "$PROTO_DIR" -proto "$proto" "$@" -d "$data" "$HOST" "$method"
}

echo "Running acceptance tests against $HOST"

# 1) 注册新用户
register_payload="{\"email\":\"$EMAIL\",\"password\":\"$PASSWORD\",\"displayName\":\"Acceptance Test\"}"
register_resp="$(grpc_call auth.proto ai.agent.platform.v1.AuthService/Register "$register_payload")" || fail "Register new user call failed"
user_id="$(json_get "$register_resp" "userId")"
[[ -n "$user_id" ]] || fail "Register did not return userId"
pass "Register new user"

# 2) 登录
login_payload="{\"email\":\"$EMAIL\",\"password\":\"$PASSWORD\",\"deviceName\":\"acceptance-test\",\"platform\":\"linux\"}"
login_resp="$(grpc_call auth.proto ai.agent.platform.v1.AuthService/Login "$login_payload")" || fail "Login call failed"
access_token="$(json_get "$login_resp" "tokenPair.accessToken")"
[[ -n "$access_token" ]] || fail "Login did not return accessToken"
pass "Login and extract token"

# 3) 发送消息（需要 LLM）
user_text="Hello from acceptance test $(date +%s)"
send_payload="{\"requestId\":\"acceptance-$(date +%s)\",\"agentId\":\"default\",\"content\":[{\"text\":{\"text\":\"$user_text\"}}]}"
if [[ "${SKIP_LLM_TESTS:-}" == "1" ]]; then
  echo "⏭ Skipped: SendMessage (SKIP_LLM_TESTS=1)"
  TOTAL=$((TOTAL - 1))
else
  send_resp="$(grpc_call chat.proto ai.agent.platform.v1.ChatService/SendMessage "$send_payload" -H "authorization: Bearer $access_token")" || fail "SendMessage call failed"
  session_id="$(json_get "$send_resp" "sessionId")"
  assistant_content="$(python3 -c 'import json,sys; d=json.loads(sys.argv[1]); c=d.get("assistantContent") or []; print(c[0].get("text",{}).get("text","") if c else "")' "$send_resp")"
  [[ -n "$session_id" ]] || fail "SendMessage did not return sessionId"
  [[ -n "$assistant_content" ]] || fail "SendMessage did not return assistantContent"
  pass "SendMessage with auth and validate response"
fi

# 4) 拉取历史（依赖 SendMessage 的 session_id）
if [[ "${SKIP_LLM_TESTS:-}" == "1" ]]; then
  echo "⏭ Skipped: ListSessionMessages (SKIP_LLM_TESTS=1)"
  TOTAL=$((TOTAL - 1))
else
  list_payload="{\"sessionId\":\"$session_id\",\"createdAtOrder\":\"SORT_ORDER_ASC\"}"
  list_resp="$(grpc_call session.proto ai.agent.platform.v1.SessionService/ListSessionMessages "$list_payload" -H "authorization: Bearer $access_token")" || fail "ListSessionMessages call failed"
  contains_msg="$(python3 -c 'import json,sys
needle=sys.argv[2]
d=json.loads(sys.argv[1])
found=False
for m in d.get("messages",[]):
    for b in m.get("blocks",[]):
        t=b.get("text",{}).get("text","")
        if needle in t:
            found=True
            break
    if found:
        break
print("1" if found else "")' "$list_resp" "$user_text")"
  [[ "$contains_msg" == "1" ]] || fail "History does not contain sent message"
  pass "ListSessionMessages contains sent message"
fi

# 5) 重复注册同一邮箱（允许报错，验证冲突语义）
set +e
dup_output="$(grpc_call auth.proto ai.agent.platform.v1.AuthService/Register "$register_payload" 2>&1)"
dup_code=$?
set -e
if [[ $dup_code -ne 0 ]]; then
  echo "$dup_output" | grep -Eqi "AlreadyExists|already exists|Conflict|code = AlreadyExists" \
    || fail "Duplicate Register failed but not with expected conflict semantics"
fi
pass "Duplicate Register handled"

# 6) 无效 token 访问 SendMessage => Unauthenticated
set +e
invalid_output="$(grpc_call chat.proto ai.agent.platform.v1.ChatService/SendMessage "$send_payload" -H "authorization: Bearer fake-token" 2>&1)"
invalid_code=$?
set -e
[[ $invalid_code -ne 0 ]] || fail "Invalid token request unexpectedly succeeded"
echo "$invalid_output" | grep -Eqi "Unauthenticated|code = Unauthenticated" || fail "Invalid token did not return Unauthenticated"
pass "Invalid token rejected with Unauthenticated"

echo ""
echo "Summary: ${PASSED}/${TOTAL} tests passed"
