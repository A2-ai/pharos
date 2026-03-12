use extendr_api::{Result, Robj, prelude::*};

use hyperion_core::ResultExt;
use scheduler::slurm::{PartitionInfo, get_partitions_info as partition_info};

fn leading_digits(s: &str) -> (&str, &str) {
    let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    (&s[..end], &s[end..])
}

#[derive(Debug, Clone, PartialEq, IntoDataFrameRow)]
pub struct RPartitionInfo {
    pub partition: Rstr,
    pub cpus: Rint,
    pub memory: Rint,
}

impl RPartitionInfo {
    fn new(partition: &str, cpus: i32, memory: i32) -> Self {
        Self {
            partition: partition.into(),
            cpus: cpus.into(),
            memory: memory.into(),
        }
    }

    pub fn fits(&self, ncpu: i32) -> bool {
        ncpu <= self.cpus.0
    }
}

impl std::str::FromStr for RPartitionInfo {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        // partitions follow cpu<n>mem<s><suffix>
        let rest = s
            .strip_prefix("cpu")
            .ok_or_else(|| format!("Partition does not start with `cpu`: {s}"))?;

        let (cpu_str, rest) = leading_digits(rest);
        if cpu_str.is_empty() {
            return Err(format!("partition is missing cpu count: {s}"));
        }

        let rest = rest
            .strip_prefix("mem")
            .ok_or_else(|| format!("Partition does not have `mem` after ncpu: {s}"))?;

        let (mem_str, _) = leading_digits(rest);
        if mem_str.is_empty() {
            return Err(format!("partition is missing memory amount: {s}"));
        }

        let cpus = cpu_str
            .parse::<i32>()
            .map_err(|_| format!("Failed to parse cpu count as i32: {cpu_str}"))?;
        let memory = mem_str
            .parse::<i32>()
            .map_err(|_| format!("Failed to parse memory amount as i32: {mem_str}"))?;

        Ok(Self::new(s, cpus, memory))
    }
}

impl From<PartitionInfo> for RPartitionInfo {
    fn from(part_info: PartitionInfo) -> Self {
        Self {
            partition: Rstr::from(part_info.partition),
            cpus: Rint::from(part_info.cpus as i32),
            memory: Rint::from(part_info.memory as i32),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PartitionTable {
    table: Vec<PartitionInfo>,
}

#[derive(Debug, Clone, PartialEq)]
struct PackingStats<'a> {
    partition: &'a str,
    partition_cpus: i32,
    models_per_node: usize,
    nodes_needed: usize,
    used_cpus: i32,
    reserved_cpus: i32,
    final_node_models: usize,
    final_node_cpus: i32,
}

impl PartitionTable {
    pub fn from_slurm() -> Result<Self> {
        let partition = partition_info().map_to_extendr_err("Failed to get partition info")?;
        Ok(Self {
            table: partition.partition_table,
        })
    }

    pub fn find_partition(&self, partition: &str) -> Option<&PartitionInfo> {
        self.table.iter().find(|row| row.partition == partition)
    }

    fn packing_stats(
        &self,
        partition: &str,
        ncpu: i32,
        model_count: usize,
    ) -> Option<PackingStats<'_>> {
        let row = self.find_partition(partition)?;
        let partition_cpus = row.cpus as i32;

        if ncpu <= 0 || partition_cpus < ncpu {
            return None;
        }

        let models_per_node = (partition_cpus / ncpu) as usize;
        if models_per_node == 0 {
            return None;
        }

        let nodes_needed = if model_count == 0 {
            0
        } else {
            (model_count + models_per_node - 1) / models_per_node
        };

        let used_cpus = model_count as i32 * ncpu;
        let reserved_cpus = nodes_needed as i32 * partition_cpus;

        let leftover_models = model_count % models_per_node;
        let final_node_models = if model_count == 0 {
            0
        } else if leftover_models == 0 {
            models_per_node
        } else {
            leftover_models
        };

        let final_node_cpus = final_node_models as i32 * ncpu;

        Some(PackingStats {
            partition: row.partition.as_str(),
            partition_cpus,
            models_per_node,
            nodes_needed,
            used_cpus,
            reserved_cpus,
            final_node_models,
            final_node_cpus,
        })
    }

    fn ranked_partitions(&self, ncpu: i32, model_count: usize) -> Vec<PackingStats<'_>> {
        let mut candidates: Vec<PackingStats<'_>> = self
            .table
            .iter()
            .filter_map(|row| self.packing_stats(row.partition.as_str(), ncpu, model_count))
            .collect();

        candidates.sort_by(|a, b| {
            let left = (a.used_cpus as i64) * (b.reserved_cpus as i64);
            let right = (b.used_cpus as i64) * (a.reserved_cpus as i64);

            right
                .cmp(&left)
                .then(a.partition_cpus.cmp(&b.partition_cpus))
                .then(a.final_node_cpus.cmp(&b.final_node_cpus))
        });

        candidates
    }

    pub fn is_underutilized(&self, partition: &str, ncpu: i32, model_count: usize) -> bool {
        let Some(stats) = self.packing_stats(partition, ncpu, model_count) else {
            return false;
        };

        stats.final_node_cpus * 2 < stats.partition_cpus
    }

    pub fn partition_advice(
        &self,
        ncpu: i32,
        partition: &str,
        model_count: usize,
        underutilized: bool,
    ) -> String {
        let mut suggested: Vec<&str> = self
            .ranked_partitions(ncpu, model_count)
            .iter()
            .map(|row| row.partition)
            .collect();

        if underutilized {
            suggested.retain(|name| *name != partition)
        }

        match suggested.as_slice() {
            [first, second, ..] if underutilized => {
                let stats = self.packing_stats(partition, ncpu, model_count);
                let partition_cpus = stats.as_ref().map(|s| s.partition_cpus).unwrap_or(ncpu);
                let final_models = stats
                    .as_ref()
                    .map(|s| s.final_node_models)
                    .unwrap_or(model_count);
                let final_node_cpus = stats.as_ref().map(|s| s.final_node_cpus).unwrap_or(ncpu);
                format!(
                    "You submitted {model_count} model(s) to `{partition}`.\nThe final group of {final_models} model(s) would use {final_node_cpus} of {partition_cpus} CPUs, which is less than 50% of the CPUs available on this partition.\nConsider increasing `ncpu`, or using a different partition for this submission.\nYou might try `{first}` or `{second}`."
                )
            }
            [first] if underutilized => {
                let stats = self.packing_stats(partition, ncpu, model_count);
                let partition_cpus = stats.as_ref().map(|s| s.partition_cpus).unwrap_or(ncpu);
                let final_models = stats
                    .as_ref()
                    .map(|s| s.final_node_models)
                    .unwrap_or(model_count);
                let final_node_cpus = stats.as_ref().map(|s| s.final_node_cpus).unwrap_or(ncpu);
                format!(
                    "You submitted {model_count} model(s) to `{partition}`.\nThe final group of {final_models} model(s) would use {final_node_cpus} of {partition_cpus} CPUs, which is less than 50% of the CPUs available on this partition.\nConsider increasing `ncpu`, or using a different partition for this submission.\nYou might try `{first}`."
                )
            }
            [] if underutilized => {
                let stats = self.packing_stats(partition, ncpu, model_count);
                let partition_cpus = stats.as_ref().map(|s| s.partition_cpus).unwrap_or(ncpu);
                let final_models = stats
                    .as_ref()
                    .map(|s| s.final_node_models)
                    .unwrap_or(model_count);
                let final_node_cpus = stats.as_ref().map(|s| s.final_node_cpus).unwrap_or(ncpu);
                format!(
                    "You submitted {model_count} model(s) to `{partition}`.\nThe final group of {final_models} model(s) would use {final_node_cpus} of {partition_cpus} CPUs, which is less than 50% of the CPUs available on this partition.\nConsider increasing `ncpu`."
                )
            }
            [first, second, ..] => format!("You might try `{first}` or `{second}`."),
            [first] => format!("You might try `{first}`."),
            [] => format!(
                "Input a smaller value for ncpu. No existing partition has {ncpu} or more CPUs per node."
            ),
        }
    }
}

/// Get the cluster partition information
///
/// @return Data frame of partition info
/// @export
///
/// @examples \dontrun{
/// get_partitions_info()
/// }
#[extendr]
pub fn get_partition_info() -> Result<Robj> {
    let table = PartitionTable::from_slurm()?;
    let rows: Vec<RPartitionInfo> = table.table.into_iter().map(RPartitionInfo::from).collect();
    let df = rows.into_dataframe()?;

    Ok(df.into())
}

extendr_module! {
    mod slurm;

    fn get_partition_info;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_table() -> PartitionTable {
        PartitionTable {
            table: vec![
                PartitionInfo {
                    partition: "cpu2mem4gb".to_string(),
                    cpus: 2,
                    memory: 3891,
                },
                PartitionInfo {
                    partition: "cpu4mem32gb".to_string(),
                    cpus: 4,
                    memory: 31129,
                },
                PartitionInfo {
                    partition: "cpu8mem64gb".to_string(),
                    cpus: 8,
                    memory: 62259,
                },
                PartitionInfo {
                    partition: "cpu2mem8gb".to_string(),
                    cpus: 2,
                    memory: 7782,
                },
                PartitionInfo {
                    partition: "cpu4mem16gb".to_string(),
                    cpus: 4,
                    memory: 15564,
                },
                PartitionInfo {
                    partition: "cpu16mem128gb".to_string(),
                    cpus: 16,
                    memory: 124518,
                },
                PartitionInfo {
                    partition: "cpu8mem32gb".to_string(),
                    cpus: 8,
                    memory: 31129,
                },
                PartitionInfo {
                    partition: "cpu16mem64gb".to_string(),
                    cpus: 16,
                    memory: 62259,
                },
                PartitionInfo {
                    partition: "cpu32mem128gb".to_string(),
                    cpus: 32,
                    memory: 124518,
                },
            ],
        }
    }

    #[test]
    fn underutilized_when_last_node_uses_less_than_half() {
        let table = mock_table();

        assert!(table.is_underutilized("cpu8mem64gb", 1, 3));
        assert!(table.is_underutilized("cpu8mem64gb", 3, 5));
    }

    #[test]
    fn not_underutilized_when_nodes_pack_cleanly() {
        let table = mock_table();

        assert!(!table.is_underutilized("cpu32mem128gb", 8, 4));
        assert!(!table.is_underutilized("cpu8mem64gb", 4, 2));
    }

    #[test]
    fn underutilized_warning_mentions_final_group_and_cpu_usage() {
        let table = mock_table();

        let msg = table.partition_advice(1, "cpu8mem64gb", 3, true);

        assert!(msg.contains("You submitted 3 model(s)"));
        assert!(msg.contains("final group of 3 model(s)"));
        assert!(msg.contains("use 3 of 8 CPUs"));
        assert!(msg.contains("less than 50% of the CPUs available"));
        assert!(msg.contains("cpu2mem4gb"));
    }

    #[test]
    fn underutilized_warning_for_three_cpu_models_uses_single_model_final_group() {
        let table = mock_table();

        let msg = table.partition_advice(3, "cpu8mem64gb", 5, true);

        assert!(msg.contains("You submitted 5 model(s)"));
        assert!(msg.contains("final group of 1 model(s)"));
        assert!(msg.contains("use 3 of 8 CPUs"));
        assert!(msg.contains("cpu16mem64gb"));
    }

    #[test]
    fn underutilized_warning_prefers_better_packing_partition() {
        let table = mock_table();

        let msg = table.partition_advice(3, "cpu8mem64gb", 5, true);

        assert!(msg.contains("cpu16mem64gb") || msg.contains("cpu16mem128gb"));
    }
}
