#!/bin/bash
# The Launcher — and, when there is no Flipchart process to hand its place over
# to, the Unavailable server (docs/adr/0014-the-launcher-never-fails.md).
#
# The constraint is first-class: it never fails. It always answers the
# handshake, in milliseconds, binary or no binary, and exits with 0. The reason
# is not the host's 30 s deadline, which is a probability: it is that a failed
# start bans the server for 15 minutes, reinstalling the plugin does not cure
# it, and all the user sees are two words —✘ failed— with our stderr buried in
# the debug log.
#
# Hence what is not here: no `set -e`, no `jq`, no `python3` —which on a machine
# without Xcode does not exist—, no `perl`, no `ruby`, not one line of network.
# Bare Bash 3.2, the one macOS ships.

trap 'exit 0' INT TERM HUP

readonly BOX="${CLAUDE_PLUGIN_ROOT:-$(dirname "$0")}"

# The box carries a binary per Machine and the choice lives here, because there
# is nowhere else for it: measured in ADR-0014, a marketplace entry has no `os`,
# no `platform`, no `arch` and no `requires`, so the catalog cannot route
# anybody anywhere. `uname` is POSIX, and the macOS binary is a universal
# Mach-O, which is why no Darwin is asked what architecture it is.
readonly SYSTEM=$(uname -s 2>/dev/null)
readonly ARCHITECTURE=$(uname -m 2>/dev/null)
case "$SYSTEM $ARCHITECTURE" in
  'Darwin '*) BINARY="$BOX/flipchart-macos" ;;
  'Linux x86_64') BINARY="$BOX/flipchart-linux-x86_64" ;;
  *) BINARY= ;;
esac
readonly BINARY

readonly MISSING='the flipchart binary is not in the plugin directory'
readonly UNRUNNABLE='the flipchart binary could not be given execute permission'
readonly FOREIGN='this machine refused to execute the flipchart binary the box carries for it'
# What was found is named: the box carries no binary for this Machine, which is
# not a fault in the user's setup and would be looked for there otherwise.
readonly ELSEWHERE="the box carries no flipchart binary for this machine, which is $SYSTEM $ARCHITECTURE"
# The two variables are named because whoever can put one back reads this: an
# `ssh -X` not asked for, a container started without one. The binary survives
# a missing display now —the main thread parks and the MCP server keeps
# answering— but a Flipchart that cannot draw is worth less than a sentence
# that says so, so the Launcher does not start down that road at all.
readonly NO_DISPLAY='this machine has no display, because neither DISPLAY nor WAYLAND_DISPLAY is set'

# A backstop, not the mechanism: the host preserves the 0755 from the Info-ZIP
# zip, but nobody promises it in its schema. It applies to the chosen binary,
# which is the only one this Machine is ever going to run.
[ -n "$BINARY" ] && chmod +x "$BINARY" 2>/dev/null

if [ -z "$BINARY" ]; then
  DIAGNOSIS=$ELSEWHERE
# Only Linux is asked: over SSH, in a container or in a devcontainer —normal
# ways to run Claude Code— there is no display, and without one `winit` cannot
# create an event loop at all (ADR-0019). On macOS the question does not exist:
# there is a window server wherever a User is logged in, and `DISPLAY` means
# nothing there.
#
# And it is asked before anything about the binary, because it is the one thing
# here that reinstalling the plugin will not cure: said second, it would send a
# User to fix the half that was going to leave them here anyway.
elif [ "$SYSTEM" = Linux ] && [ -z "$DISPLAY" ] && [ -z "$WAYLAND_DISPLAY" ]; then
  DIAGNOSIS=$NO_DISPLAY
elif [ ! -e "$BINARY" ]; then
  DIAGNOSIS=$MISSING
elif [ ! -x "$BINARY" ]; then
  DIAGNOSIS=$UNRUNNABLE
# The probe, and why there is one: `execfail` only reaches a failure at
# `execve`, which is how a Mach-O of another architecture fails. On Linux a
# binary built against a newer glibc is a valid ELF —`execve` succeeds and the
# dynamic loader fails afterwards—, and by then bash has been replaced and
# there is nobody left to answer the handshake: the host sees a dead server and
# the user gets the fifteen-minute ban this script exists to make impossible.
# So the chosen binary is started once, and only a zero exit earns the
# hand-over. It costs one process start, on the order of milliseconds, and
# milliseconds is what was promised.
#
# Its stdin is `/dev/null` and not ours: the probe does not read, and one that
# did would eat the `initialize` we are here to answer. Not mitigated, removed.
elif ! "$BINARY" probe </dev/null >/dev/null 2>&1; then
  DIAGNOSIS=$FOREIGN
else
  # `execfail` stays as the backstop behind the probe: what answered it a
  # millisecond ago can be gone —a reinstall mid-flight, a quarantine that
  # lands in between— by the time `exec` reaches it, and without this line that
  # `exec` would kill the only voice left.
  shopt -s execfail
  exec "$BINARY" "$@"
  DIAGNOSIS=$FOREIGN
fi

# ── The Unavailable server ────────────────────────────────────────────────────
#
# The JSON is built by hand, so no text here carries a `"` or a `\`.

# The `command`'s stderr ends up in the host's debug log, so no user reads this
# line: it is for whoever looks at the log after them.
printf 'flipchart: %s\n' "$DIAGNOSIS" >&2

readonly TOOL=unavailable
readonly MESSAGE="The flipchart is not available in this session and cannot draw anything: ${DIAGNOSIS}. Nothing will appear on screen, so do not offer the user a diagram - explain in prose instead. Reinstalling the plugin is what brings it back."
readonly LAST_KNOWN_PROTOCOL=2025-06-18

string_field() {
  local rest=${2#*\"$1\"}
  [ "$rest" = "$2" ] && return 1
  rest=${rest#*\"}
  printf '%s' "${rest%%\"*}"
}

# The first `"id":` on the line is always the JSON-RPC one: with a single tool
# that takes no arguments, no message this loop ever parses carries a nested
# `"id"`. The fragility is not mitigated, it is removed.
the_id_in() {
  local rest=${1#*\"id\"}
  [ "$rest" = "$1" ] && return 1
  rest=${rest#*:}
  while [ "${rest# }" != "$rest" ]; do rest=${rest# }; done
  case $rest in
    '"'*)
      rest=${rest#\"}
      printf '"%s"' "${rest%%\"*}"
      ;;
    *)
      rest=${rest%%,*}
      rest=${rest%%\}*}
      printf '%s' "${rest%% *}"
      ;;
  esac
}

answer() {
  printf '{"jsonrpc":"2.0","id":%s,"result":%s}\n' "$1" "$2"
}

# We answer with whatever version the client speaks: this loop depends on
# nothing a protocol revision could move, and staying anchored to an old version
# would be the way to fail the handshake three releases from now.
greeting() {
  printf '{"protocolVersion":"%s","capabilities":{"tools":{}},"serverInfo":{"name":"flipchart","version":"unavailable"}}' \
    "$(string_field protocolVersion "$1" || printf '%s' "$LAST_KNOWN_PROTOCOL")"
}

the_only_tool() {
  printf '{"tools":[{"name":"%s","description":"%s","inputSchema":{"type":"object","properties":{}}}]}' \
    "$TOOL" "$MESSAGE"
}

the_message_as_a_result() {
  printf '{"content":[{"type":"text","text":"%s"}],"isError":true}' "$MESSAGE"
}

while IFS= read -r line || [ -n "$line" ]; do
  case "$(string_field method "$line")" in
    initialize) answer "$(the_id_in "$line")" "$(greeting "$line")" ;;
    tools/list) answer "$(the_id_in "$line")" "$(the_only_tool)" ;;
    tools/call) answer "$(the_id_in "$line")" "$(the_message_as_a_result)" ;;
    # The protocol's `ping` is not asked for by the box, it is asked for by the
    # session: an unanswered request is a connection the host can give up for
    # dead, and with it would go the only warning the user was going to get.
    ping) answer "$(the_id_in "$line")" '{}' ;;
  esac
done

exit 0
