# Ghost SSH Manager - System Metrics Setup Guide

## Current Issue
System metrics are not showing up because Ghost requires **passwordless SSH access** to collect system information from remote servers in the background.

## Solution: Set Up SSH Key Authentication

### Step 1: Generate SSH Keys (if you don't have them)
```bash
# Generate a new SSH key pair (if you don't have one)
ssh-keygen -t ed25519 -C "your_email@example.com"

# Or use RSA if ed25519 is not supported
ssh-keygen -t rsa -b 4096 -C "your_email@example.com"

# Press Enter to use default location (~/.ssh/id_ed25519 or ~/.ssh/id_rsa)
# Set a passphrase if desired (recommended for security)
```

### Step 2: Copy Your Public Key to Remote Servers

For **blacksquid** (black-snow.ddns.net:1126):
```bash
ssh-copy-id -p 1126 seawn@black-snow.ddns.net
```

For **BlackSquid2** (100.101.235.47:1127):
```bash
ssh-copy-id -p 1127 seawn@100.101.235.47
```

### Step 3: Test Passwordless Access
```bash
# Test blacksquid
ssh -o PasswordAuthentication=no -p 1126 seawn@black-snow.ddns.net "echo 'Test successful'"

# Test BlackSquid2
ssh -o PasswordAuthentication=no -p 1127 seawn@100.101.235.47 "echo 'Test successful'"
```

If these commands work without prompting for a password, you're ready!

### Step 4: Update Ghost Configuration

Edit your Ghost config to use key-based authentication:

**Option A: Use SSH Agent (Recommended)**
```toml
[servers.b3a5d643-6e33-42f0-a04a-be835e96d5fb.auth_method]
type = "agent"

[servers.75890f4d-2a73-4ad5-90de-c9708f23abdb.auth_method]
type = "agent"
```

**Option B: Specify Key Path**
```toml
[servers.b3a5d643-6e33-42f0-a04a-be835e96d5fb.auth_method]
type = "public_key"
key_path = "~/.ssh/id_ed25519"

[servers.75890f4d-2a73-4ad5-90de-c9708f23abdb.auth_method]
type = "public_key"
key_path = "~/.ssh/id_ed25519"
```

### Step 5: Start SSH Agent (if using agent auth)
```bash
# Start SSH agent
eval "$(ssh-agent -s)"

# Add your SSH key
ssh-add ~/.ssh/id_ed25519  # or ~/.ssh/id_rsa
```

## What Metrics Will Be Collected

Once set up, Ghost will collect these system metrics every 10 seconds:

- **CPU Usage** - Current processor utilization
- **Memory Usage** - RAM utilization and totals
- **Disk Usage** - Root partition usage
- **Load Average** - System load (1, 5, 15 minute averages)
- **Uptime** - How long the system has been running
- **Process Count** - Number of running processes

## Troubleshooting

### If metrics still don't show up:

1. **Check SSH agent is running**:
   ```bash
   ssh-add -l
   ```

2. **Verify passwordless access**:
   ```bash
   ssh -o BatchMode=yes -p PORT username@hostname "echo test"
   ```

3. **Check Ghost debug output**:
   ```bash
   cargo run
   # Look for messages about skipping servers or collection failures
   ```

4. **Test the metrics script manually**:
   ```bash
   ssh -p PORT username@hostname "free -k | grep 'Mem:'"
   ```

## Security Note

Setting up SSH keys is actually more secure than password authentication because:
- Keys are much longer and more complex than typical passwords
- You can add passphrases to keys for additional security
- Keys can be easily revoked if compromised
- No password is transmitted over the network

## Current Status

- **blacksquid**: Password auth (metrics disabled)
- **BlackSquid2**: Agent auth configured but keys not set up (metrics disabled)

Once you complete the setup above, you should see system metrics in the Ghost interface!