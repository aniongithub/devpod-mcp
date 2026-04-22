FROM debian:bookworm-slim

ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        curl \
        wget \
        pkg-config \
        libssl-dev \
        ca-certificates \
        git \
        git-lfs \
        sudo \
    && git lfs install \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user — use --force to avoid failure if GID already exists
ARG USERNAME=vscode
ARG USER_UID=1000
ARG USER_GID=$USER_UID
RUN groupadd --force --gid $USER_GID $USERNAME \
    && useradd --uid $USER_UID --gid $USER_GID -m $USERNAME -s /bin/bash || true \
    && echo "$USERNAME ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers.d/$USERNAME \
    && chmod 0440 /etc/sudoers.d/$USERNAME

# Ensure feature-installed tools are on PATH for non-root user
ENV PATH="/usr/local/cargo/bin:/usr/local/share/nvm/current/bin:${PATH}"
ENV CARGO_HOME="/usr/local/cargo"
ENV RUSTUP_HOME="/usr/local/rustup"
