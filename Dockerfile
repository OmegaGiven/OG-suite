FROM node:22-bookworm-slim AS web
WORKDIR /app
COPY package.json package-lock.json tsconfig.base.json ./
COPY apps ./apps
COPY packages ./packages
RUN npm ci
RUN npm run build --workspace @og-suite/suite

FROM rust:1-bookworm AS backend
WORKDIR /app
COPY backend ./backend
RUN cargo build --manifest-path backend/Cargo.toml --release

FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*
COPY --from=backend /app/backend/target/release/og-suite-backend /usr/local/bin/og-suite-backend
COPY --from=web /app/apps/suite/dist /app/public
ENV OG_SUITE_BIND=0.0.0.0:8080
ENV OG_SUITE_STATIC_DIR=/app/public
ENV OG_SUITE_DATA_DIR=/app/data
EXPOSE 8080
CMD ["og-suite-backend"]
