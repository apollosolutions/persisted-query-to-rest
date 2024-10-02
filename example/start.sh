docker run \
  -it -p 8080:8080 \
  --mount "type=bind,source=./config.yml,target=/app/config.yml" \
  ghcr.io/apollosolutions/persisted-query-to-rest:latest --config /app/config.yml
