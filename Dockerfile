# Multi-stage build for Rust components
FROM rust:1.75 as rust-builder
WORKDIR /usr/src/app
COPY . .
RUN cargo build --release --workspace

# Node.js stage
FROM node:18-alpine as node-builder
WORKDIR /app
COPY api-server/package*.json ./api-server/
COPY frontend-dashboard/package*.json ./frontend-dashboard/
RUN cd api-server && npm ci
RUN cd frontend-dashboard && npm ci

COPY api-server ./api-server
COPY frontend-dashboard ./frontend-dashboard
RUN cd api-server && npm run build
RUN cd frontend-dashboard && npm run build

# Final stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=rust-builder /usr/src/app/target/release/block-engine /usr/local/bin/
COPY --from=node-builder /app/api-server/dist /app/api-server/
COPY --from=node-builder /app/frontend-dashboard/.next /app/frontend-dashboard/

EXPOSE 8080 3001 3000
CMD ["block-engine"]
