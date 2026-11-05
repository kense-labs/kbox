# Multi-platform OCI image for krun-gateway
# Supports: linux/amd64, linux/arm64, darwin/arm64
#
# Usage:
#   docker buildx build --platform linux/amd64,linux/arm64,darwin/arm64 \
#     -t ghcr.io/kense-labs/gateway:latest -f deploy/Dockerfile --push .
#
# Note: darwin/arm64 is a macOS binary for Docker Desktop on Apple Silicon.
#       For native macOS, use Homebrew: brew install krun-lab/tap/krun-gateway

# ── Linux: Alpine-based minimal image ────────────────────────────────────────────
FROM alpine:3 AS base
RUN apk add --no-cache ca-certificates tzdata && \
    addgroup -S gateway && adduser -S gateway -G gateway

# ── Linux amd64 ────────────────────────────────────────────────────────────────
FROM base AS linux-amd64
COPY dist/krun-gateway /usr/local/bin/krun-gateway
USER gateway
ENTRYPOINT ["krun-gateway"]

# ── Linux arm64 ────────────────────────────────────────────────────────────────
FROM base AS linux-arm64
COPY dist/krun-gateway /usr/local/bin/krun-gateway
USER gateway
ENTRYPOINT ["krun-gateway"]

# ── macOS arm64 (Apple Silicon) ───────────────────────────────────────────────
# Packaged as standalone binary for Docker Desktop on Apple Silicon
FROM scratch AS darwin-arm64
COPY dist/krun-gateway /krun-gateway
ENTRYPOINT ["/krun-gateway"]
