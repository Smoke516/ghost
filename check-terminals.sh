#!/bin/bash

# Quick script to check which terminals are available on your system
echo "🔍 Checking for supported terminal emulators..."
echo "----------------------------------------"

terminals=(
    "ghostty:Ghostty terminal"
    "warp-terminal:Warp terminal (main)"
    "warp:Warp terminal (alt)"
    "gnome-terminal:GNOME Terminal"
    "konsole:KDE Konsole"
    "alacritty:Alacritty"
    "kitty:Kitty terminal"
    "wezterm:WezTerm"
    "xterm:XTerm"
)

found_count=0

for terminal_info in "${terminals[@]}"; do
    IFS=':' read -r cmd desc <<< "$terminal_info"
    if command -v "$cmd" >/dev/null 2>&1; then
        echo "✅ $desc ($cmd)"
        ((found_count++))
        
        # Show version if possible
        case $cmd in
            "ghostty")
                version=$(ghostty --version 2>/dev/null || echo "version unknown")
                echo "   Version: $version"
                ;;
            "warp-terminal"|"warp")
                echo "   AI-powered terminal with modern features"
                ;;
            "gnome-terminal")
                version=$(gnome-terminal --version 2>/dev/null || echo "version unknown")
                echo "   Version: $version"
                ;;
            "alacritty")
                version=$(alacritty --version 2>/dev/null || echo "version unknown")
                echo "   Version: $version"
                ;;
            *)
                echo "   Available"
                ;;
        esac
        echo
    else
        echo "❌ $desc ($cmd) - not found"
    fi
done

echo "----------------------------------------"
echo "📊 Found $found_count out of ${#terminals[@]} supported terminals"

if [ $found_count -gt 0 ]; then
    echo "🚀 Ghost will use the first available terminal for SSH sessions"
else
    echo "⚠️  No supported terminals found. Install one of the above for auto-launch"
    echo "💡 Ghost will still provide copy-paste SSH commands"
fi

echo
echo "🔧 To install popular terminals:"
echo "   Ghostty:  https://ghostty.org/"
echo "   Warp:     https://www.warp.dev/"
echo "   Alacritty: sudo apt install alacritty"
echo "   Kitty:    sudo apt install kitty"