#![allow(unused_imports)]
#![allow(dead_code)]

use std::ffi::{OsStr, OsString};
use anyhow::{bail, Context, Result};
use tracing::{warn, info, debug};
use clap::{CommandFactory, Parser};
use subprocess::{Exec, NullFile, Redirection};
use std::time::{Instant, Duration};

const METADATA: &str = r#"{
  "SchemaVersion": "0.1.0",
  "Vendor": "Janne Virtala (github.com/veyh)",
  "Version": "1.0.0",
  "ShortDescription": "Restart compose service with no downtime"
}"#;

#[derive(Debug, Parser, Clone)]
struct AppArgs {
    /// Compose configuration file
    #[arg(short, long)]
    file: Option<String>,

    /// Specify an alternate environment file
    #[arg(long)]
    env_file: Option<String>,

    /// Command to run before stopping old container. {id} will be replaced
    /// with container id
    #[arg(long)]
    pre_stop_cmd: Option<String>,

    /// Assuming there is a health check, after running the pre-stop command,
    /// wait until the old container is unhealthy before stopping it.
    #[arg(long)]
    pre_stop_wait_until_unhealthy: bool,

    /// Health check timeout (in seconds)
    #[arg(long, default_value_t = 60)]
    healthcheck_timeout: u64,

    /// Wait X seconds before stopping old container when there is no health
    /// check
    #[arg(long, default_value_t = 10)]
    wait: u64,

    /// When there is a health check and it succeeds, wait additional X seconds
    /// before stopping old container
    #[arg(long, default_value_t = 0)]
    wait_after_healthy: u64,

    /// Service(s) to deploy.
    #[arg()]
    service: String,
}

#[derive(Debug, Clone)]
pub struct App {
    args: AppArgs,
    compose_command: Exec,
    docker_args: Vec<OsString>,
}

impl App {
    pub fn main() -> Result<()> {
        let mut env_args = vec![];
        let mut docker_args = vec![];

        for arg in std::env::args_os() {
            env_args.push(arg.clone());
        }

        if env_args.len() >= 2
        && env_args[1] == OsStr::new("docker-cli-plugin-metadata") {
            println!("{}", METADATA);
            return Ok(());
        }

        if env_args.contains(&OsString::from("reroll")) {
            let mut env_args_new = vec![
                env_args[0].clone()
            ];

            let mut reroll_found = false;

            for arg in env_args.iter().skip(1) {
                if arg == OsStr::new("reroll") {
                    reroll_found = true;
                    continue;
                }

                if reroll_found {
                    env_args_new.push(arg.clone());
                }

                else {
                    docker_args.push(arg.clone());
                }
            }

            env_args = env_args_new;
        }

        let mut app = Self {
            args: AppArgs::parse_from(env_args),
            compose_command: Self::detect_compose_command()?,
            docker_args,
        };

        app.main_internal()
    }

    fn detect_compose_command() -> Result<Exec> {
        let cmd = Exec::cmd("docker").arg("compose");
        let exit_status = cmd
            .clone()
            .stdout(NullFile)
            .stderr(NullFile)
            .join()?;

        if exit_status.success() {
            return Ok(cmd);
        }

        let cmd = Exec::cmd("docker-compose");
        let exit_status = cmd
            .clone()
            .stdout(NullFile)
            .stderr(NullFile)
            .join()?;

        if exit_status.success() {
            return Ok(cmd);
        }

        bail!("docker compose command not found");
    }

    fn compose_command(&self) -> Exec {
        let mut cmd = self.compose_command.clone();

        if let Some(value) = &self.args.file {
            cmd = cmd.args(&["-f", &value]);
        }

        if let Some(value) = &self.args.env_file {
            cmd = cmd.args(&["--env-file", &value]);
        }

        cmd
    }

    fn docker_command(&self) -> Exec {
        let mut cmd = Exec::cmd("docker");

        for arg in self.docker_args.iter() {
            cmd = cmd.arg(arg);
        }

        cmd
    }

    fn main_internal(&mut self) -> Result<()> {
        debug!(?self);

        if !self.is_service_running()? {
            info!("service is not running --> start");
            return self.start_service();
        }

        let old_ids = self.get_container_ids()?;
        debug!(?old_ids);

        let scale = old_ids.len() * 2;

        debug!("scale from {} to {} instances", old_ids.len(), scale);
        self.scale(scale)?;

        let all_ids = self.get_container_ids()?;
        let new_ids: Vec<_> = all_ids
            .iter()
            .filter(|x| !old_ids.contains(x))
            .cloned()
            .collect();

        debug!(?all_ids);
        debug!(?new_ids);

        if self.has_health_check(&old_ids[0])? {
            self.wait_for_healthy_or_rollback(&new_ids)?;
            self.wait_for_healthy_to_settle_down();
        }

        else {
            self.wait_fallback();
        }

        self.pre_stop(&old_ids)?;

        debug!("stop old containers");
        self.stop(&old_ids)?;

        debug!("remove old containers");
        self.remove(&old_ids)?;

        debug!("done");
        Ok(())
    }

    fn is_service_running(&self) -> Result<bool> {
        Ok(self.get_container_ids()?.is_empty())
    }

    fn start_service(&self) -> Result<()> {
        self
            .compose_command()
            .arg("up")
            .arg("--detach")
            .arg("--no-recreate")
            .arg(&self.args.service)
            .join()
            .context("failed to start service")?;

        Ok(())
    }

    fn get_container_ids(&self) -> Result<Vec<String>> {
        let res = self
            .compose_command()
            .arg("ps")
            .arg("--quiet")
            .arg(&self.args.service)
            .stdout(Redirection::Pipe)
            .stderr(NullFile)
            .capture()
            .context("failed to get container ids")?;

        let mut ids = vec![];

        for line in res.stdout_str().trim().lines() {
            ids.push(line.to_string());
        }

        Ok(ids)
    }

    fn scale(&self, num_instances: usize) -> Result<()> {
        let res = self
            .compose_command()
            .arg("up")
            .arg("--detach")
            .arg("--scale")
            .arg(format!("{}={}", &self.args.service, num_instances))
            .arg("--no-recreate")
            .arg(&self.args.service)
            .join()
            .context("failed to scale instances")?;

        if !res.success() {
            bail!("failed to scale instances (non-zero exit code)");
        }

        Ok(())
    }

    fn stop(&self, container_ids: &[String]) -> Result<()> {
        let mut cmd = self.docker_command().arg("stop");

        for id in container_ids {
            cmd = cmd.arg(id);
        }

        debug!(?container_ids, "stop");

        let res = cmd
            .stdout(NullFile)
            .join()
            .context("failed to stop instances")?;

        if !res.success() {
            bail!("failed to stop instances (non-zero exit code)");
        }

        Ok(())
    }

    fn remove(&self, container_ids: &[String]) -> Result<()> {
        let mut cmd = self.docker_command().arg("rm");

        for id in container_ids {
            cmd = cmd.arg(id);
        }

        debug!(?container_ids, "remove");

        let res = cmd
            .stdout(NullFile)
            .join()
            .context("failed to remove instances")?;

        if !res.success() {
            bail!("failed to remove instances (non-zero exit code)");
        }

        Ok(())
    }

    fn has_health_check(&self, container_id: &str) -> Result<bool> {
        let res = self
            .docker_command()
            .arg("inspect")
            .arg("--format")
            .arg("{{json .State.Health}}")
            .arg(container_id)
            .stdout(Redirection::Pipe)
            .capture()?;

        Ok(res.stdout_str().contains("Status"))
    }

    fn is_healthy(&self, container_id: &str) -> Result<bool> {
        let res = self
            .docker_command()
            .arg("inspect")
            .arg("--format")
            .arg("{{json .State.Health.Status}}")
            .arg(container_id)
            .stdout(Redirection::Pipe)
            .capture()?;

        let s = res.stdout_str();

        Ok(!s.contains("unhealthy") && s.contains("healthy"))
    }

    fn wait_for_healthy_or_rollback(&self, container_ids: &[String]) -> Result<()> {
        debug!(
            "wait for new containers to be healthy (timeout {} seconds)",
            self.args.healthcheck_timeout
        );

        if self.wait_until_healthy_count(container_ids.len(), container_ids).is_ok() {
            return Ok(());
        }

        debug!("new containers aren't healthy after timeout --> rollback");

        self.stop(container_ids)?;
        self.remove(container_ids)?;

        bail!("new containers weren't healthy after timeout")
    }

    fn wait_for_healthy_to_settle_down(&self) {
        if self.args.wait_after_healthy == 0 {
            return;
        }

        debug!(
            "wait for healthy containers to settle down ({} seconds",
            self.args.wait_after_healthy
        );

        std::thread::sleep(
            Duration::from_secs(self.args.wait_after_healthy)
        );
    }

    fn wait_until_healthy_count(&self, target_count: usize, container_ids: &[String]) -> Result<()> {
        let end_time = Instant::now()
                     + Duration::from_secs(self.args.healthcheck_timeout);

        while Instant::now() < end_time {
            let mut healthy_count = 0;

            for id in container_ids.iter() {
                match self.is_healthy(id) {
                    Ok(value) if value => { healthy_count += 1; },
                    _ => {},
                }
            }

            debug!(?healthy_count, ?target_count);

            if healthy_count == target_count {
                return Ok(());
            }

            std::thread::sleep(Duration::from_secs(1));
        }

        bail!("timed out")
    }

    fn wait_fallback(&self) {
        debug!(
            "wait for new containers to be ready ({} seconds",
            self.args.wait
        );

        std::thread::sleep(Duration::from_secs(self.args.wait));
    }

    fn pre_stop(&self, container_ids: &[String]) -> Result<()> {
        let Some(pre_stop_cmd) = &self.args.pre_stop_cmd else {
            return Ok(());
        };

        for id in container_ids.iter() {
            let cmd = pre_stop_cmd.replace("{id}", id);

            debug!(?cmd, "run pre-stop command");

            if let Err(e) = Exec::shell(cmd).join() {
                warn!(container_id = ?id, err = ?e, "pre-stop command failed");
            }
        }

        if !self.args.pre_stop_wait_until_unhealthy {
            return Ok(());
        }

        debug!(
            "wait for old containers to be unhealthy (timeout {} seconds)",
            self.args.healthcheck_timeout
        );

        if self.wait_until_healthy_count(0, container_ids).is_err() {
            warn!("timed out while waiting for old containers to become unhealthy");
        }

        Ok(())
    }
}
