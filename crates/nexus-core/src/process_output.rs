use nexus_domain::InstanceId;
use nexus_domain::InstanceLogStream;
use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;
use tokio::spawn;

use crate::InstanceLogStore;

const MAXIMUM_LOG_LINE_BYTES: usize = 64 * 1024;
const OUTPUT_READ_BUFFER_BYTES: usize = 8 * 1024;
const TRUNCATED_SUFFIX: &[u8] = b" [truncated]";

/// 异步读取子进程输出并写入实例日志存储。
///
/// 单行超过上限时保留前缀并附加截断标记，避免异常输出占用不受控的内存。
pub(crate) fn spawn_output_reader<R>(
    mut reader: R,
    instance_id: InstanceId,
    stream: InstanceLogStream,
    logs: InstanceLogStore,
) where
    R: AsyncRead + Send + Unpin + 'static,
{
    drop(spawn(async move {
        let mut buffer = [0_u8; OUTPUT_READ_BUFFER_BYTES];
        let mut line = Vec::new();
        let mut truncated = false;

        loop {
            let read = match reader.read(&mut buffer).await {
                Ok(read) => read,
                Err(error) => {
                    tracing::error!(%instance_id, ?stream, %error, "Unable to read instance process output");
                    return;
                }
            };
            if read == 0 {
                if !line.is_empty() || truncated {
                    append_line(&logs, &instance_id, stream, &mut line, truncated);
                }
                return;
            }

            for byte in &buffer[..read] {
                if *byte == b'\n' {
                    append_line(&logs, &instance_id, stream, &mut line, truncated);
                    line.clear();
                    truncated = false;
                } else if line.len() < MAXIMUM_LOG_LINE_BYTES
                    || (*byte == b'\r' && line.len() == MAXIMUM_LOG_LINE_BYTES)
                {
                    line.push(*byte);
                } else {
                    truncated = true;
                }
            }
        }
    }));
}

fn append_line(
    logs: &InstanceLogStore,
    instance_id: &InstanceId,
    stream: InstanceLogStream,
    line: &mut Vec<u8>,
    truncated: bool,
) {
    if line.last() == Some(&b'\r') {
        line.pop();
    }
    if truncated {
        line.truncate(MAXIMUM_LOG_LINE_BYTES.saturating_sub(TRUNCATED_SUFFIX.len()));
        line.extend_from_slice(TRUNCATED_SUFFIX);
    }
    let line = String::from_utf8_lossy(line).into_owned();
    if let Err(error) = logs.append(instance_id, stream, line) {
        tracing::error!(%instance_id, %error, "Unable to store instance process output");
    }
}
