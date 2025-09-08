#!/usr/bin/env bash
set -euo pipefail

SOCK="${2:-/tmp/openvpn-mock.sock}"
STATE_DIR="${STATE_DIR:-/tmp/openvpn-mock-state}"
CLIENTS_FILE="$STATE_DIR/clients.tsv"

usage() {
  cat <<EOF
Usage:
  $0 start [socket]        # start mock mgmt server on UNIX socket
  $0 seed                  # seed demo clients
  $0 ls                    # show current mock clients
  $0 clear                 # remove all mock clients & socket
EOF
}

ensure_state() {
  mkdir -p "$STATE_DIR"
  touch "$CLIENTS_FILE"
}

seed() {
  ensure_state
  : > "$CLIENTS_FILE"
  # id  cn           real_ip:port      vpn_ip     bytes_in bytes_out connected_since_epoch
  echo -e "1\tclient1\t203.0.113.5:54321\t10.8.0.2\t12345\t67890\t$(date -u +%s)" >> "$CLIENTS_FILE"
  echo -e "2\tclient2\t203.0.113.6:54322\t10.8.0.3\t22345\t77890\t$(date -u +%s)" >> "$CLIENTS_FILE"
  echo "Seeded:"
  cat "$CLIENTS_FILE"
}

ls_clients() {
  ensure_state
  if [[ ! -s "$CLIENTS_FILE" ]]; then
    echo "(no clients)"
    return
  fi
  column -t -s $'\t' "$CLIENTS_FILE" | sed '1iID  CN       REAL                VPN        IN     OUT    SINCE'
}

clear_all() {
  rm -f "$CLIENTS_FILE" "$SOCK"
  rmdir "$STATE_DIR" 2>/dev/null || true
  echo "Cleared state and socket."
}

sum_bytes() {
  awk -F'\t' '{in+=$5; out+=$6} END {printf "%d %d", in+0, out+0}' "$CLIENTS_FILE" 2>/dev/null || echo "0 0"
}

print_status_3() {
  echo "TITLE,OpenVPN Mock Management"
  echo "TIME,$(date -u +%s)"
  echo "HEADER,CLIENT_LIST,Common Name,Real Address,Virtual Address,Bytes Received,Bytes Sent,Connected Since,Client ID"
  while IFS=$'\t' read -r id cn real vpn bin bout since; do
    [[ -z "$id" ]] && continue
    echo "CLIENT_LIST,$cn,$real,$vpn,$bin,$bout,$since,$id"
  done < <(awk -F'\t' 'NF>=7 {print $1"\t"$2"\t"$3"\t"$4"\t"$5"\t"$6"\t"$7}' "$CLIENTS_FILE" 2>/dev/null)
  echo "HEADER,ROUTING_TABLE,Virtual Address,Common Name,Real Address,Last Ref,Client ID"
  while IFS=$'\t' read -r id cn real vpn _ _ since; do
    [[ -z "$id" ]] && continue
    echo "ROUTING_TABLE,$vpn,$cn,$real,$since,$id"
  done < "$CLIENTS_FILE" 2>/dev/null
  echo "GLOBAL_STATS,Max bcast/mcast queue length,0"
  echo "END"
}

do_kill_id() {
  local id="$1"
  if [[ -z "$id" ]]; then
    echo "ERROR: missing id"
    return
  fi
  if grep -qE "^${id}\t" "$CLIENTS_FILE" 2>/dev/null; then
    tmp="$(mktemp)"
    grep -vE "^${id}\t" "$CLIENTS_FILE" > "$tmp" || true
    mv "$tmp" "$CLIENTS_FILE"
    echo "SUCCESS: client-id $id killed"
  else
    echo "ERROR: client-id $id not found"
  fi
}

do_kill_cn() {
  local cn="$1"
  if [[ -z "$cn" ]]; then
    echo "ERROR: missing cn"
    return
  fi
  if grep -qE "^[^\t]+\t${cn}\t" "$CLIENTS_FILE" 2>/dev/null; then
    tmp="$(mktemp)"
    grep -vE "^[^\t]+\t${cn}\t" "$CLIENTS_FILE" > "$tmp" || true
    mv "$tmp" "$CLIENTS_FILE"
    echo "SUCCESS: client-cn $cn killed"
  else
    echo "ERROR: client-cn $cn not found"
  fi
}

handler() {
  # one process per connection
  ensure_state
  # Greeting similar to OpenVPN mgmt
  echo ">INFO:OpenVPN Management Interface Version 3 -- type 'help' for more info"
  echo ">HOLD:Waiting for hold release"
  echo ">STATE:$(date -u +%s),CONNECTED,SUCCESS,10.8.0.1,,"  # harmless line

  while IFS= read -r line; do
    cmd="$(echo "$line" | tr -d '\r')"
    case "$cmd" in
      "load-stats")
        read -r IN OUT < <(sum_bytes)
        nclients=$(wc -l < "$CLIENTS_FILE" 2>/dev/null || echo 0)
        echo "SUCCESS: nclients=$nclients,bytesin=$IN,bytesout=$OUT"
        ;;
      "status 3")
        print_status_3
        ;;
      quit)
        echo "SUCCESS: quit"
        exit 0
        ;;
      help)
        echo "OpenVPN Mock commands: load-stats | status 3 | kill <id> | client-kill <cn> | quit"
        ;;
      kill\ *)
        do_kill_id "${cmd#kill }"
        ;;
      client-kill\ *)
        do_kill_cn "${cmd#client-kill }"
        ;;
      *)
        echo "ERROR: unknown command: $cmd"
        ;;
    esac
  done
}

start() {
  ensure_state
  rm -f "$SOCK"
  echo "Mock mgmt listening on $SOCK"
  umask 007
  exec socat -d -d UNIX-LISTEN:"$SOCK",fork,unlink-early SYSTEM:"$0 __handle '$SOCK'"
}

case "${1:-}" in
  start) start ;;
  seed)  seed ;;
  ls)    ls_clients ;;
  clear) clear_all ;;
  __handle) handler ;; # internal
  *) usage; exit 1 ;;
esac
