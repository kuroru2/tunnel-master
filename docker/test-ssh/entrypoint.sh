#!/bin/sh

# If a public key was mounted, install it for testuser
if [ -f /home/testuser/.ssh/authorized_keys ]; then
    chown testuser:testuser /home/testuser/.ssh/authorized_keys
    chmod 600 /home/testuser/.ssh/authorized_keys
fi

# Start HTTP traffic servers on multiple ports (simulating various services)
python3 /usr/local/bin/traffic-server.py 5432 6379 8080 3000 9200 9090 &

# Start sshd in foreground
exec /usr/sbin/sshd -D -e
