#!/bin/bash

echo "🧪 Testing SSH command launching with Ghostty..."
echo "This will simulate what Ghost does when connecting to a server"
echo

# Test the exact command format that Ghost uses
echo "1. Testing basic SSH command format:"
echo "   ghostty -e zsh -c \"ssh -p 22 user@example.com\""
echo

echo "2. Testing with public key:"
echo "   ghostty -e zsh -c \"ssh -p 22 -i ~/.ssh/id_rsa user@example.com\""
echo

echo "3. Testing current Ghostty executable:"
if command -v ghostty >/dev/null 2>&1; then
    echo "   ✅ Ghostty found: $(which ghostty)"
    echo "   Version: $(ghostty --version 2>/dev/null || echo 'version check failed')"
    echo
    
    echo "4. Testing manual command (won't actually connect):"
    echo "   This should open Ghostty with an SSH session..."
    echo "   Press Ctrl+C to cancel if it tries to connect"
    echo
    
    # Test with a dummy command that won't actually connect
    echo "   ghostty -e zsh -c \"echo 'SSH session would start here'; sleep 3\""
    echo "   Running in 3 seconds... (Press Ctrl+C to cancel)"
    sleep 3
    
    # Run the test
    ghostty -e zsh -c "echo 'SSH session would start here: ssh -p 22 user@example.com'; echo 'Ghost would run this command in a real connection'; sleep 5; echo 'Test complete!'"
else
    echo "   ❌ Ghostty not found in PATH"
    echo "   Make sure Ghostty is installed and accessible"
fi

echo
echo "🔍 If Ghostty opened with the test message, then Ghost should work!"
echo "   The actual SSH connection will replace the test message"