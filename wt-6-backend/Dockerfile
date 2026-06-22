FROM rust:1-bookworm AS build

WORKDIR /app

COPY wt-3-innertube ./wt-3-innertube
COPY wt-6-backend ./wt-6-backend

WORKDIR /app/wt-6-backend
RUN cargo build --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=build /app/wt-6-backend/target/release/wt-6-backend /usr/local/bin/wt-6-backend

ENV WT_BACKEND_BIND=0.0.0.0:8787
ENV RUST_LOG=info,tower_http=info

EXPOSE 8787

CMD ["wt-6-backend"]
