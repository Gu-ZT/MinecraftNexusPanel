use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use nexus_domain::EventId;
use nexus_domain::InstanceId;
use nexus_domain::InstanceLogLine;
use nexus_domain::InstanceLogPage;
use nexus_domain::InstanceLogStream;
use nexus_protocol::WireMessage;
use serde_json::json;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::sync::broadcast;

use crate::InstanceLogStoreError;

const MAXIMUM_LINES_PER_INSTANCE: usize = 10_000;
type InstanceLogLines = BTreeMap<InstanceId, VecDeque<(u64, InstanceLogLine)>>;

#[derive(Clone)]
pub(crate) struct InstanceLogStore {
    cursor: Arc<AtomicU64>,
    event_sender: broadcast::Sender<WireMessage>,
    event_sequence: Arc<AtomicU64>,
    lines: Arc<Mutex<InstanceLogLines>>,
}

impl InstanceLogStore {
    pub(crate) fn new(
        event_sender: broadcast::Sender<WireMessage>,
        event_sequence: Arc<AtomicU64>,
    ) -> Self {
        Self {
            cursor: Arc::new(AtomicU64::new(1)),
            event_sender,
            event_sequence,
            lines: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    pub(crate) fn append(
        &self,
        instance_id: &InstanceId,
        stream: InstanceLogStream,
        line: String,
    ) -> Result<InstanceLogLine, InstanceLogStoreError> {
        let cursor = self.cursor.fetch_add(1, Ordering::Relaxed);
        let log_line = InstanceLogLine::new(cursor.to_string(), current_timestamp(), stream, line);
        let mut lines = self.lock_lines()?;
        let instance_lines = lines.entry(instance_id.clone()).or_default();
        instance_lines.push_back((cursor, log_line.clone()));
        while instance_lines.len() > MAXIMUM_LINES_PER_INSTANCE {
            instance_lines.pop_front();
        }
        drop(lines);

        let event = WireMessage::Event {
            event_id: EventId::new(),
            topic: "instance.console".to_owned(),
            sequence: self.event_sequence.fetch_add(1, Ordering::Relaxed),
            occurred_at: log_line.occurred_at().to_owned(),
            data: json!({
                "instanceId": instance_id,
                "stream": log_line.stream(),
                "line": log_line.line(),
                "cursor": log_line.cursor(),
            }),
        };
        let _ = self.event_sender.send(event);

        Ok(log_line)
    }

    pub(crate) fn read(
        &self,
        instance_id: &InstanceId,
        after: Option<u64>,
        before: Option<u64>,
        limit: usize,
    ) -> Result<InstanceLogPage, InstanceLogStoreError> {
        let lines = self.lock_lines()?;
        let filtered = lines
            .get(instance_id)
            .into_iter()
            .flatten()
            .filter(|(cursor, _)| after.is_none_or(|after| *cursor > after))
            .filter(|(cursor, _)| before.is_none_or(|before| *cursor < before))
            .collect::<Vec<_>>();
        let start = if before.is_some() && after.is_none() {
            filtered.len().saturating_sub(limit)
        } else {
            0
        };
        let end = filtered.len().min(start + limit);
        let has_more = if before.is_some() && after.is_none() {
            start > 0
        } else {
            end < filtered.len()
        };
        let items = filtered[start..end]
            .iter()
            .map(|(_, line)| (*line).clone())
            .collect::<Vec<_>>();
        let next_cursor = if has_more && before.is_some() && after.is_none() {
            items.first()
        } else if has_more {
            items.last()
        } else {
            None
        }
        .map(InstanceLogLine::cursor)
        .map(str::to_owned);

        Ok(InstanceLogPage::new(items, next_cursor))
    }

    fn lock_lines(&self) -> Result<MutexGuard<'_, InstanceLogLines>, InstanceLogStoreError> {
        self.lines
            .lock()
            .map_err(|_| InstanceLogStoreError::LockPoisoned)
    }
}

fn current_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}
