# docker-reroll

This is a tool for zero-downtime deployment of docker containers, very much like [docker-rollout](https://github.com/Wowu/docker-rollout/), except that this is not in bash, and has some additional features.

## Install (x86_64)

```sh
mkdir -p ~/.docker/cli-plugins

curl -fsSL \
  httos://cdn.soupbawx.com/docker-reroll/docker-reroll-1.0.0-x86_64-unknown-linux-musl \
  -o ~/.docker/cli-plugins/docker-reroll

chmod +x ~/.docker/cli-plugins/docker-rollout
```

## Usage

```
Usage: docker reroll [OPTIONS] <SERVICE>

Arguments:
  <SERVICE>  Service to deploy

Options:
  -f, --file <FILE>
          Compose configuration file
      --env-file <ENV_FILE>
          Specify an alternate environment file
      --pre-stop-cmd <PRE_STOP_CMD>
          Command to run before stopping old container. {id} will be replaced with container id
      --pre-stop-wait-until-unhealthy
          Assuming there is a health check, after running the pre-stop command, wait until the old container is unhealthy before stopping it
      --healthcheck-timeout <HEALTHCHECK_TIMEOUT>
          Health check timeout (in seconds) [default: 60]
      --wait <WAIT>
          Wait X seconds before stopping old container when there is no health check [default: 10]
      --wait-after-healthy <WAIT_AFTER_HEALTHY>
          When there is a health check and it succeeds, wait additional X seconds before stopping old container [default: 0]
  -h, --help
          Print help
  -V, --version
          Print version
```
