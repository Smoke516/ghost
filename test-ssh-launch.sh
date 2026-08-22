#!/bin/bash
#
# Shows the exact command Ghost builds when you connect, and checks that your
# terminal emulator can run it.
#
# Ghost never assembles a shell string. It passes `ssh` and each of its options
# as separate argv elements, so a hostname or username containing shell
# metacharacters cannot be interpreted as a command. This script mirrors that.

set -u

HOST="${1:-example.com}"
USER_NAME="${2:-user}"
PORT="${3:-22}"

echo "🧪 Ghost SSH launch check"
echo

echo "Ghost builds this argv (note: no shell involved):"
printf '   ssh'
for arg in -p "$PORT" -o ServerAliveInterval=60 -o ServerAliveCountMax=3 \
           -o ConnectTimeout=10 -o BatchMode=no "${USER_NAME}@${HOST}"; do
    printf ' %q' "$arg"
done
echo
echo

echo "Terminal emulators Ghost can launch a session in:"
found=0
for entry in \
    "ghostty:ghostty -e ssh …" \
    "alacritty:alacritty -e ssh …" \
    "kitty:kitty ssh …" \
    "wezterm:wezterm start -- ssh …" \
    "gnome-terminal:gnome-terminal -- ssh …" \
    "konsole:konsole -e ssh …" \
    "xfce4-terminal:xfce4-terminal -x ssh …" \
    "xterm:xterm -e ssh …"
do
    cmd="${entry%%:*}"
    form="${entry#*:}"
    if command -v "$cmd" >/dev/null 2>&1; then
        echo "   ✅ $cmd    →  $form"
        found=$((found + 1))
    fi
done

echo
if [ "$found" -eq 0 ]; then
    echo "   ⚠️  None found. Ghost will fall back to direct mode, connecting in"
    echo "      the current terminal. Force it explicitly with: ghost --direct"
else
    echo "   $found supported terminal(s) available."
fi

echo
echo "Note: gnome-terminal and konsole hand off to a background daemon and exit"
echo "immediately, so Ghost does not track sessions opened through them."
