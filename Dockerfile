# ---- Stage 1: Build ----
# Use the official Rust image as a builder
FROM rust:1.90 as builder

# Create a new empty shell project
WORKDIR /usr/src/app
COPY . .

# Build for release
RUN cargo build --release

# ---- Stage 2: Run ----
# Use a minimal, secure base image
FROM debian:bullseye-slim

# Copy the compiled binary and static assets from the builder stage
COPY --from=builder /usr/src/app/target/release/cline-auth-service /usr/local/bin/
COPY --from=builder /usr/src/app/handle-auth.html /app/handle-auth.html

# Create a directory for auth state
WORKDIR /app
RUN mkdir -p /app/data

# Expose the port the app runs on
EXPOSE 8888

# Set environment variables
ENV CONTAINER_MODE=true
ENV AUTH_STATE_DIR=/app/data
ENV HOST_BINDING=0.0.0.0

# Set the entrypoint
CMD ["cline-auth-service"]
