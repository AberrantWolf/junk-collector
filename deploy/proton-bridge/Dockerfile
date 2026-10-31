# Self-managed packaging of Proton's unchanged official AMD64 DEB.
FROM ubuntu:24.04
ARG BRIDGE_VERSION=3.18.0
ARG BRIDGE_SHA256=4a4b4f3efe5502d2fc1280a2abd3d3bae5091275b466d1148c1b7df27cd571b7
RUN test "$(dpkg --print-architecture)" = amd64
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl gnupg pass debsig-verify tini python3 && rm -rf /var/lib/apt/lists/*
COPY bridge_pubkey.asc /tmp/bridge_pubkey.asc
COPY bridge.pol /etc/debsig/policies/E2C75D68E6234B07/bridge.pol
RUN mkdir -p /usr/share/debsig/keyrings/E2C75D68E6234B07 && \
    gpg --show-keys --with-colons /tmp/bridge_pubkey.asc | grep -q '^fpr:::::::::D51E64D3E63EDC3EEF7864CEE2C75D68E6234B07:' && \
    gpg --dearmor --output /usr/share/debsig/keyrings/E2C75D68E6234B07/debsig.gpg /tmp/bridge_pubkey.asc && \
    curl --fail --location --proto '=https' --tlsv1.2 "https://proton.me/download/bridge/protonmail-bridge_${BRIDGE_VERSION}-1_amd64.deb" -o /tmp/bridge.deb && \
    echo "${BRIDGE_SHA256}  /tmp/bridge.deb" | sha256sum --check --strict && \
    debsig-verify /tmp/bridge.deb && \
    apt-get update && apt-get install -y --no-install-recommends /tmp/bridge.deb && \
    rm -rf /var/lib/apt/lists/* /tmp/bridge.deb /tmp/bridge_pubkey.asc && \
    useradd --uid 10001 --create-home bridge
COPY --chmod=755 entrypoint.sh /usr/local/bin/bridge-container
COPY status.py /usr/local/lib/bridge-status.py
ENV HOME=/home/bridge GNUPGHOME=/home/bridge/.gnupg PASSWORD_STORE_DIR=/home/bridge/.password-store
USER 10001:10001
ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/bridge-container"]
CMD ["serve"]
