#!/bin/sh

# If a public key was mounted, install it for testuser
if [ -f /home/testuser/.ssh/authorized_keys ]; then
    chown testuser:testuser /home/testuser/.ssh/authorized_keys
    chmod 600 /home/testuser/.ssh/authorized_keys
fi

# Start HTTP traffic server on port 80 (for port-forward testing)
python3 /usr/local/bin/traffic-server.py &

# Start sshd in foreground
exec /usr/sbin/sshd -D -e
