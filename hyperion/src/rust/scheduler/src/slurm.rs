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
pub struct RPartitionTable {
    table: Vec<RPartitionInfo>,
}

impl RPartitionTable {
    pub fn from_slurm() -> Result<Self> {
        let partition = partition_info().map_to_extendr_err("Failed to get partition info")?;
        let rows = partition
            .partition_table
            .into_iter()
            .map(RPartitionInfo::from)
            .collect();

        Ok(Self { table: rows })
    }

    pub fn find_partition(&self, partition: &str) -> Option<&RPartitionInfo> {
        self.table
            .iter()
            .find(|row| row.partition.as_ref() == partition)
    }

    fn final_batch_cpus(&self, partition: &str, ncpu: i32, model_count: usize) -> Option<i32> {
        let row = self.find_partition(partition)?;
        let partition_cpus = row.cpus.0;
        let total_requested = ncpu * model_count as i32;

        Some(match total_requested % partition_cpus {
            0 if total_requested >= partition_cpus => partition_cpus,
            0 => total_requested,
            remainder => remainder,
        })
    }

    pub fn is_underutilized(&self, partition: &str, ncpu: i32, model_count: usize) -> bool {
        let Some(row) = self.find_partition(partition) else {
            return false;
        };

        let Some(final_batch_cpus) = self.final_batch_cpus(partition, ncpu, model_count) else {
            return false;
        };

        final_batch_cpus < row.cpus.0 / 2
    }

    pub fn partition_advice(
        &self,
        ncpu: i32,
        partition: &str,
        model_count: usize,
        underutilized: bool,
    ) -> String {
        let target_ncpu = if underutilized {
            match self.final_batch_cpus(partition, ncpu, model_count) {
                Some(cpus) => cpus,
                None => return "Consider increasing `ncpu`.".to_string(),
            }
        } else {
            ncpu
        };

        let mut candidates: Vec<RPartitionInfo> = self
            .table
            .iter()
            .filter(|row| row.fits(target_ncpu))
            .cloned()
            .collect();

        candidates.sort_by(|a, b| a.cpus.0.cmp(&b.cpus.0).then(a.memory.0.cmp(&b.memory.0)));

        let mut suggested: Vec<&str> = candidates
            .iter()
            .map(|row| row.partition.as_ref())
            .collect();

        if underutilized {
            suggested.retain(|name| *name != partition)
        }

        match suggested.as_slice() {
            [first, second, ..] if underutilized => {
                let partition_cpus = self
                    .find_partition(partition)
                    .map(|row| row.cpus.0)
                    .unwrap_or(target_ncpu);
                format!(
                    "You're using {target_ncpu} of {partition_cpus} CPUs in the final batch on partition `{partition}`, which is less than 50%.\nConsider increasing `ncpu`, or submitting the remainder model(s) to a smaller partition.\nYou might try `{first}` or `{second}` for the remainder model(s)."
                )
            }
            [first] if underutilized => {
                let partition_cpus = self
                    .find_partition(partition)
                    .map(|row| row.cpus.0)
                    .unwrap_or(target_ncpu);
                format!(
                    "You're using {target_ncpu} of {partition_cpus} CPUs in the final batch on partition `{partition}`, which is less than 50%.\nConsider increasing `ncpu`, or submitting the remainder model(s) to a smaller partition.\nYou might try `{first}` for the remainder model(s)."
                )
            }
            [] if underutilized => {
                let partition_cpus = self
                    .find_partition(partition)
                    .map(|row| row.cpus.0)
                    .unwrap_or(target_ncpu);
                format!(
                    "You're using {target_ncpu} of {partition_cpus} CPUs in the final batch on partition `{partition}`, which is less than 50%.\nConsider increasing `ncpu`."
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
    let table = RPartitionTable::from_slurm()?;
    let df = table.table.into_dataframe()?;

    Ok(df.into())
}

extendr_module! {
    mod slurm;

    fn get_partition_info;
}
