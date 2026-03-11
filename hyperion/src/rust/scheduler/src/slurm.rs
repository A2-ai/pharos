use extendr_api::{Result, Robj, prelude::*};

use hyperion_core::ResultExt;
use scheduler::slurm::{PartitionInfo, get_partitions_info as partition_info}; //() -> Result<PartitionCache> {

#[derive(Debug, Clone, PartialEq, IntoDataFrameRow)]
struct RPartitionInfo {
    pub partition: Rstr,
    pub cpus: Rint,
    pub memory: Rint,
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

#[extendr]
pub fn get_partitions_info() -> Result<Robj> {
    let partition = partition_info().map_to_extendr_err("Failed to get partition info")?;
    let table = partition.partition_table;

    let rows: Vec<RPartitionInfo> = table.into_iter().map(RPartitionInfo::from).collect();
    let df = rows.into_dataframe()?;

    Ok(df.into())
}

extendr_module! {
    mod slurm;

    fn get_partitions_info;
}
