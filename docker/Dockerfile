# Stage 1: Builder stage to fetch the Tini binary
FROM --platform=$BUILDPLATFORM alpine:latest AS builder
ARG TARGETPLATFORM
ARG TARGETARCH
ARG TARGETOS
ARG TARGETVARIANT

# Set Tini version (check for the latest version on the Tini GitHub releases page)
ARG TINI_VERSION=v0.19.0

# Download the static Tini binary
RUN apk add --no-cache curl openssl
RUN curl -L -o /tini https://github.com/krallin/tini/releases/download/${TINI_VERSION}/tini-static-$TARGETARCH

COPY $TARGETARCH/router_bot /

# Make the binary executable
RUN chmod +x /tini /router_bot

# ===================
# Stage 2: Final scratch image
FROM --platform=$BUILDPLATFORM alpine:latest
ARG TARGETPLATFORM
ARG TARGETARCH
ARG TARGETOS
ARG TARGETVARIANT

# Copy the Tini binary from the builder stage to the final image
COPY --from=builder /tini /router_bot /

ENV LOG_LEVEL="info"

ENTRYPOINT ["/tini", "--"]
CMD ["/router_bot"]
