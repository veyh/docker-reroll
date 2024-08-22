# docker-reroll

This is a tool for zero-downtime deployment of docker containers, very much like [docker-rollout](https://github.com/Wowu/docker-rollout/), except that this is not in bash, and has some additional features.

## Install (x86_64)

```sh
mkdir -p ~/.docker/cli-plugins

curl -fsSL \
  https://cdn.soupbawx.com/docker-reroll/docker-reroll-latest-x86_64-unknown-linux-musl \
  -o ~/.docker/cli-plugins/docker-reroll

chmod +x ~/.docker/cli-plugins/docker-reroll
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

## Example: Traefik

Go into `examples/traefik` directory. In one terminal, do

```sh
docker compose up
```

From another terminal, try eg.

```sh
curl -i http://127.115.183.188:3000/
```

```
HTTP/1.1 200 OK
Content-Length: 12
Content-Type: text/plain; charset=utf-8
Date: Thu, 22 Aug 2024 17:24:57 GMT
X-App-Name: http-echo
X-App-Version: 1.0.0

hello world
```

Now, to ensure a proper zero-downtime deployment, we want to start a new container and wait for it to become healthy, then make sure that the old container becomes unhealthy (which drops it from Traefik's load balancer) before removing it.

When the health check in our compose file is simply

````yml
healthcheck:
  test: "! test -e /.dead"
````

We can then do

```sh
docker reroll \
  --pre-stop-cmd 'docker exec {id} touch /.dead' \
  --pre-stop-wait-until-unhealthy \
  example
```

Output might look something like this

```
2024-06-15T07:38:01.187563Z DEBUG docker_reroll::app: self=App { args: AppArgs { file: None, env_file: None, pre_stop_cmd: Some("docker exec {id} touch /.dead"), pre_stop_wait_until_unhealthy: true, healthcheck_timeout: 60, wait: 10, wait_after_healthy: 0, service: "example" }, compose_command: Exec { docker compose }, docker_args: [] }
2024-06-15T07:38:01.304160Z DEBUG docker_reroll::app: old_ids=["fcbccba6cb7cf224dd7ab1d67ca935f95be60782475019909fadeb393856e1a6"]
2024-06-15T07:38:01.304197Z DEBUG docker_reroll::app: scale from 1 to 2 instances
[+] Running 3/3
 ✔ Container traefik-traefik-1  Running        0.0s
 ✔ Container traefik-example-1  Running        0.0s
 ✔ Container traefik-example-2  Started        0.2s
2024-06-15T07:38:01.960876Z DEBUG docker_reroll::app: all_ids=["fcbccba6cb7cf224dd7ab1d67ca935f95be60782475019909fadeb393856e1a6", "c4369d8ac6a25635c9173a1825d49b0927e9ef175ed096a6b3e482eabec8d996"]
2024-06-15T07:38:01.960909Z DEBUG docker_reroll::app: new_ids=["c4369d8ac6a25635c9173a1825d49b0927e9ef175ed096a6b3e482eabec8d996"]
2024-06-15T07:38:01.974485Z DEBUG docker_reroll::app: wait for new containers to be healthy (timeout 60 seconds)
2024-06-15T07:38:01.987508Z DEBUG docker_reroll::app: healthy_count=0 target_count=1
2024-06-15T07:38:02.999383Z DEBUG docker_reroll::app: healthy_count=1 target_count=1
2024-06-15T07:38:02.999433Z DEBUG docker_reroll::app: run pre-stop command cmd="docker exec fcbccba6cb7cf224dd7ab1d67ca935f95be60782475019909fadeb393856e1a6 touch /.dead"
2024-06-15T07:38:03.077105Z DEBUG docker_reroll::app: wait for old containers to be unhealthy (timeout 60 seconds)
2024-06-15T07:38:03.090285Z DEBUG docker_reroll::app: healthy_count=1 target_count=0
2024-06-15T07:38:04.102987Z DEBUG docker_reroll::app: healthy_count=1 target_count=0
2024-06-15T07:38:05.116826Z DEBUG docker_reroll::app: healthy_count=0 target_count=0
2024-06-15T07:38:05.116867Z DEBUG docker_reroll::app: stop old containers
2024-06-15T07:38:05.116880Z DEBUG docker_reroll::app: stop container_ids=["fcbccba6cb7cf224dd7ab1d67ca935f95be60782475019909fadeb393856e1a6"]
2024-06-15T07:38:06.192339Z DEBUG docker_reroll::app: remove old containers
2024-06-15T07:38:06.192386Z DEBUG docker_reroll::app: remove container_ids=["fcbccba6cb7cf224dd7ab1d67ca935f95be60782475019909fadeb393856e1a6"]
2024-06-15T07:38:06.254073Z DEBUG docker_reroll::app: done
```

If you were to run the curl command above in a tight loop while deploying, you shouldn't see any 5xx errors (which you **would** see with [docker-rollout](https://github.com/Wowu/docker-rollout/)).

**NOTE: Use a unique `traefik_id` label in the compose file to make sure that Traefik doesn't interact with unrelated containers!**


```yml
services:
  example:
    labels:
      - "traefik_id=example"
  traefik:
    command:
      - "--providers.docker.constraints=Label(`traefik_id`, `example`)"
```
