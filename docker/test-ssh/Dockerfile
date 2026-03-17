FROM alpine:3.19

RUN apk add --no-cache openssh-server python3 && \
    ssh-keygen -A && \
    adduser -D -s /bin/sh testuser && \
    echo "testuser:testpass" | chpasswd && \
    mkdir -p /home/testuser/.ssh && \
    chown -R testuser:testuser /home/testuser/.ssh && \
    chmod 700 /home/testuser/.ssh

# Allow password auth + pubkey auth
RUN sed -i 's/^#PasswordAuthentication.*/PasswordAuthentication yes/' /etc/ssh/sshd_config && \
    sed -i 's/^#PubkeyAuthentication.*/PubkeyAuthentication yes/' /etc/ssh/sshd_config && \
    sed -i 's/^#PermitRootLogin.*/PermitRootLogin no/' /etc/ssh/sshd_config && \
    sed -i 's/^AllowTcpForwarding no/AllowTcpForwarding yes/' /etc/ssh/sshd_config && \
    sed -i 's/^X11Forwarding no/X11Forwarding yes/' /etc/ssh/sshd_config && \
    echo "AllowTcpForwarding yes" >> /etc/ssh/sshd_config && \
    echo "GatewayPorts no" >> /etc/ssh/sshd_config

EXPOSE 22 80

COPY traffic-server.py /usr/local/bin/traffic-server.py
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

CMD ["/entrypoint.sh"]
