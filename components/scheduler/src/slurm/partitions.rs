use anyhow::{Context, Result, bail};
use std::process::Command;
use std::sync::OnceLock;

const PARTITION_CACHE: OnceLock<PartitionCache> = OnceLock::new();

#[derive(Debug, Clone, PartialEq)]
pub struct PartitionInfo {
    pub partition: String,
    cpus: u32,
    memory: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PartitionCache {
    sinfo_output: String,
    partition_table: Vec<PartitionInfo>,
}

impl PartitionCache {
    pub fn default_partition(&self) -> &PartitionInfo {
        self.partition_table.first().unwrap()
    }

    pub fn exists(&self, name: &str) -> bool {
        self.partition_table
            .iter()
            .find(|x| x.partition == name)
            .is_some()
    }
}

fn run_sinfo() -> Result<String> {
    let sinfo_bin = which::which("sinfo").context("failed to find `sinfo`")?;
    let output = Command::new(sinfo_bin)
        .args(&["--format", "%P,%c,%m"])
        .output()
        .context("failed to execute sinfo command to retrieve partition information")?;

    if !output.status.success() {
        bail!(
            "`sinfo` failed with `{}`:\nstdout: {}\nstderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn get_partitions_info() -> Result<PartitionCache> {
    if let Some(cached) = PARTITION_CACHE.get() {
        log::debug!("Getting partition info from cache");
        return Ok(cached.clone());
    }

    log::debug!("Getting partition info from sinfo");
    let raw = run_sinfo()?;
    // Process the table as is
    let mut partitions = Vec::new();
    let mut default_idx = 0;
    // Skip header line
    for line in raw.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts = line.split(",").collect::<Vec<_>>();
        if parts.len() != 3 {
            continue;
        }
        let mut partition = parts[0].trim().to_string();
        if partition.ends_with('*') {
            partition = partition.trim_end_matches('*').to_string();
            default_idx = partitions.len();
        }
        let cpus = parts[1].trim().parse::<u32>().with_context(|| {
            format!(
                "failed to parse CPU count '{}' from sinfo output line: '{}'",
                parts[1].trim(),
                line
            )
        })?;
        let memory = parts[2].trim().parse::<u32>().with_context(|| {
            format!(
                "failed to parse memory value '{}' from sinfo output line: '{}'",
                parts[2].trim(),
                line
            )
        })?;
        partitions.push(PartitionInfo {
            partition,
            cpus,
            memory,
        });
    }
    let default = partitions.remove(default_idx);
    partitions.insert(0, default);
    let cache = PartitionCache {
        sinfo_output: raw,
        partition_table: partitions,
    };

    log::debug!("Finished getting partition info from sinfo");
    Ok(PARTITION_CACHE.get_or_init(|| cache).clone())
}
