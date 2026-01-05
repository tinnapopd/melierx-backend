#!/usr/bin/env bash

set -x
set -eo pipefail

# If redis is already running, print instuctions to kill it and exit
RUNNING_CONTAINERS=$(docker ps --filter "name=redis" --format "{{.ID}}")
if [ -n "$RUNNING_CONTAINERS" ]; then
    echo >&2 "there is a redis container already running, kill it with:"
    echo >&2 "  docker kill $RUNNING_CONTAINERS"
    exit 1
fi

# Launch redis using docker
docker run \
    -p "6379:6379" \
    --name "redis_$(date +%s)" \
    -d \
    redis:8.4.0

>&2 echo "Redis is ready to go!"