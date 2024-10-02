docker run \
  -p 8080:8080 \
  --mount "type=bind,source=./config.yml,target=/app/config.yml" \
  --name persisted-query-to-rest \
  ghcr.io/apollosolutions/persisted-query-to-rest:v0.3.2 --config /app/config.yml
