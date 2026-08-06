use std::mem::size_of;
use std::ptr::null_mut;
use std::slice;

use windows_sys::Win32::System::SystemInformation::GetLogicalProcessorInformationEx;
use windows_sys::Win32::System::SystemInformation::RelationProcessorCore;
use windows_sys::Win32::System::SystemInformation::SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX;

const RECORD_HEADER_SIZE: usize = 8;
const PROCESSOR_RELATIONSHIP_PREFIX_SIZE: usize = 24;
const GROUP_AFFINITY_OFFSET: usize = RECORD_HEADER_SIZE + PROCESSOR_RELATIONSHIP_PREFIX_SIZE;
const MAX_TOPOLOGY_BUFFER_SIZE: usize = 16 * 1024 * 1024;
const MAX_QUERY_ATTEMPTS: usize = 3;
const PROCESSORS_PER_GROUP: u32 = 64;

/// Windows 报告的单个物理核心及其活动逻辑处理器。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessorCore {
    efficiency_class: u8,
    logical_processor_ids: Vec<u32>,
}

impl ProcessorCore {
    /// 返回 Windows `PROCESSOR_RELATIONSHIP.EfficiencyClass` 原始值。
    #[must_use]
    pub const fn efficiency_class(&self) -> u8 {
        self.efficiency_class
    }

    /// 返回由处理器组和组内位号组成的稳定逻辑处理器 ID。
    #[must_use]
    pub fn logical_processor_ids(&self) -> &[u32] {
        &self.logical_processor_ids
    }
}

/// 读取 Windows Processor Relationship 核心记录。
///
/// Windows 处理器组各自使用独立位图，因此逻辑处理器 ID 按
/// `group * 64 + bit` 编码，避免多组主机上的组内编号冲突。
#[must_use]
pub fn processor_cores() -> Option<Vec<ProcessorCore>> {
    parse_processor_cores(&query_processor_core_bytes()?)
}

/// 调用 Windows 可变长缓冲区 API，并在离开 FFI 边界前复制有效字节。
///
/// 第一次调用只获取长度；若查询期间拓扑变化导致长度增长，最多重试三次。
/// 缓冲区使用 `usize` 对齐，满足 API 对其结构体指针的对齐要求。
#[allow(unsafe_code)]
fn query_processor_core_bytes() -> Option<Vec<u8>> {
    let mut required_bytes = 0_u32;
    unsafe {
        GetLogicalProcessorInformationEx(RelationProcessorCore, null_mut(), &mut required_bytes);
    }
    if required_bytes == 0 || required_bytes as usize > MAX_TOPOLOGY_BUFFER_SIZE {
        return None;
    }

    for _ in 0..MAX_QUERY_ATTEMPTS {
        let required = required_bytes as usize;
        let word_count = required.checked_add(size_of::<usize>() - 1)? / size_of::<usize>();
        let mut buffer = vec![0_usize; word_count];
        let capacity_bytes = buffer.len().checked_mul(size_of::<usize>())?;
        let mut returned_bytes = required_bytes;
        let succeeded = unsafe {
            GetLogicalProcessorInformationEx(
                RelationProcessorCore,
                buffer
                    .as_mut_ptr()
                    .cast::<SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX>(),
                &mut returned_bytes,
            )
        };

        if succeeded != 0 {
            let returned = returned_bytes as usize;
            if returned > capacity_bytes {
                return None;
            }
            let bytes = unsafe { slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), returned) };
            return Some(bytes.to_vec());
        }

        if returned_bytes <= required_bytes || returned_bytes as usize > MAX_TOPOLOGY_BUFFER_SIZE {
            return None;
        }
        required_bytes = returned_bytes;
    }

    None
}

fn parse_processor_cores(bytes: &[u8]) -> Option<Vec<ProcessorCore>> {
    let mut cores = Vec::new();
    let mut offset = 0_usize;

    while offset < bytes.len() {
        let remaining = bytes.get(offset..)?;
        if remaining.len() < RECORD_HEADER_SIZE {
            return None;
        }
        let relationship = read_i32(remaining, 0)?;
        let record_size = usize::try_from(read_u32(remaining, 4)?).ok()?;
        if record_size < RECORD_HEADER_SIZE || record_size > remaining.len() {
            return None;
        }

        if relationship == RelationProcessorCore {
            cores.push(parse_processor_core(&remaining[..record_size])?);
        }
        offset = offset.checked_add(record_size)?;
    }

    (!cores.is_empty()).then_some(cores)
}

fn parse_processor_core(record: &[u8]) -> Option<ProcessorCore> {
    if record.len() < GROUP_AFFINITY_OFFSET {
        return None;
    }
    let efficiency_class = *record.get(RECORD_HEADER_SIZE + 1)?;
    let group_count = usize::from(read_u16(record, GROUP_AFFINITY_OFFSET - 2)?);
    if group_count == 0 {
        return None;
    }

    let affinity_size = size_of::<usize>() + size_of::<u16>() * 4;
    let affinities_size = group_count.checked_mul(affinity_size)?;
    let required_size = GROUP_AFFINITY_OFFSET.checked_add(affinities_size)?;
    if required_size > record.len() {
        return None;
    }

    let mut logical_processor_ids = Vec::new();
    for index in 0..group_count {
        let affinity_offset =
            GROUP_AFFINITY_OFFSET.checked_add(index.checked_mul(affinity_size)?)?;
        let mut mask = read_usize(record, affinity_offset)?;
        let group = read_u16(record, affinity_offset + size_of::<usize>())?;
        while mask != 0 {
            let bit = mask.trailing_zeros();
            let id = u32::from(group)
                .checked_mul(PROCESSORS_PER_GROUP)?
                .checked_add(bit)?;
            logical_processor_ids.push(id);
            mask &= mask - 1;
        }
    }
    logical_processor_ids.sort_unstable();
    logical_processor_ids.dedup();
    if logical_processor_ids.is_empty() {
        return None;
    }

    Some(ProcessorCore {
        efficiency_class,
        logical_processor_ids,
    })
}

fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    let value = bytes.get(offset..offset.checked_add(size_of::<u16>())?)?;
    Some(u16::from_ne_bytes(value.try_into().ok()?))
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let value = bytes.get(offset..offset.checked_add(size_of::<u32>())?)?;
    Some(u32::from_ne_bytes(value.try_into().ok()?))
}

fn read_i32(bytes: &[u8], offset: usize) -> Option<i32> {
    let value = bytes.get(offset..offset.checked_add(size_of::<i32>())?)?;
    Some(i32::from_ne_bytes(value.try_into().ok()?))
}

fn read_usize(bytes: &[u8], offset: usize) -> Option<usize> {
    let value = bytes.get(offset..offset.checked_add(size_of::<usize>())?)?;
    Some(usize::from_ne_bytes(value.try_into().ok()?))
}

#[cfg(test)]
mod tests {
    use super::GROUP_AFFINITY_OFFSET;
    use super::ProcessorCore;
    use super::RelationProcessorCore;
    use super::parse_processor_cores;
    use super::processor_cores;
    use std::mem::size_of;

    #[test]
    fn parses_processor_groups_without_id_collisions() {
        let mut bytes = processor_record(8, &[(0, 0b0101), (1, 0b0010)]);
        bytes.extend(processor_record(2, &[(0, 0b1010)]));

        let cores = parse_processor_cores(&bytes).expect("synthetic topology is valid");

        assert_eq!(
            cores,
            vec![
                ProcessorCore {
                    efficiency_class: 8,
                    logical_processor_ids: vec![0, 2, 65],
                },
                ProcessorCore {
                    efficiency_class: 2,
                    logical_processor_ids: vec![1, 3],
                },
            ]
        );
    }

    #[test]
    fn rejects_truncated_processor_relationships() {
        let mut record = processor_record(4, &[(0, 1)]);
        record.truncate(record.len() - 1);

        assert!(parse_processor_cores(&record).is_none());
    }

    #[test]
    fn reads_nonempty_host_processor_relationships() {
        let cores = processor_cores().expect("Windows processor relationships are available");

        assert!(!cores.is_empty());
        assert!(
            cores
                .iter()
                .all(|core| !core.logical_processor_ids().is_empty())
        );
    }

    fn processor_record(efficiency_class: u8, affinities: &[(u16, usize)]) -> Vec<u8> {
        let affinity_size = size_of::<usize>() + size_of::<u16>() * 4;
        let record_size = GROUP_AFFINITY_OFFSET + affinities.len() * affinity_size;
        let mut record = vec![0_u8; record_size];
        record[0..4].copy_from_slice(&RelationProcessorCore.to_ne_bytes());
        record[4..8].copy_from_slice(&(record_size as u32).to_ne_bytes());
        record[9] = efficiency_class;
        record[30..32].copy_from_slice(&(affinities.len() as u16).to_ne_bytes());

        for (index, (group, mask)) in affinities.iter().enumerate() {
            let offset = GROUP_AFFINITY_OFFSET + index * affinity_size;
            record[offset..offset + size_of::<usize>()].copy_from_slice(&mask.to_ne_bytes());
            let group_offset = offset + size_of::<usize>();
            record[group_offset..group_offset + size_of::<u16>()]
                .copy_from_slice(&group.to_ne_bytes());
        }
        record
    }
}
