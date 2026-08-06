//! History — pure store contract + in-memory store + helpers.
//!
//! The disk-backed stores (`FileHistoryStore` JSON, `SQLCipherHistoryStore`)
//! live in `src-tauri/src/history.rs` and implement the `HistoryStore` trait
//! defined here. The migration-gate decision (`migration_verified`) and the
//! cap helper (`trim_records`) are pure and unit-tested here.

use crate::delivery::DeliveryRecord;
use serde::{Deserialize, Serialize};

pub const MAX_HISTORY_ITEMS: usize = 200;
pub const DEFAULT_HISTORY_PAGE_SIZE: u32 = 50;
pub const MAX_HISTORY_PAGE_SIZE: u32 = 200;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistoryRetention {
    #[default]
    #[serde(rename = "forever")]
    Forever,
    #[serde(rename = "7_days")]
    SevenDays,
    #[serde(rename = "30_days")]
    ThirtyDays,
    #[serde(rename = "90_days")]
    NinetyDays,
    #[serde(rename = "1_year")]
    OneYear,
}

impl HistoryRetention {
    pub fn cutoff_ms(self, now_ms: u64) -> Option<u64> {
        const DAY_MS: u64 = 24 * 60 * 60 * 1_000;
        let days = match self {
            Self::Forever => return None,
            Self::SevenDays => 7,
            Self::ThirtyDays => 30,
            Self::NinetyDays => 90,
            Self::OneYear => 365,
        };
        Some(now_ms.saturating_sub(days * DAY_MS))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryFilter {
    #[serde(default)]
    pub search: String,
    #[serde(default)]
    pub from_ms: Option<u64>,
    #[serde(default)]
    pub until_ms: Option<u64>,
    #[serde(default)]
    pub app: String,
    /// Outcome scope for the two history tabs. `false` (default) = the
    /// Dictations tab: exclude both `DeliveryOutcome::Cancelled` (raw/L0,
    /// never delivered) and `DeliveryOutcome::Draft` (in-progress crash-recovery
    /// snapshots from `TranscriptPartial`). `true` = the Cancelled tab:
    /// EXCLUSIVELY cancelled records (drafts stay hidden). This is a scope, not
    /// an additive include — the tabs are mutually exclusive, so the count and
    /// pagination reflect exactly the rows shown (no client-side re-filtering).
    /// Defaults to `false` (serde default) so callers that omit it get Dictations.
    #[serde(default)]
    pub cancelled_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryQuery {
    #[serde(flatten)]
    pub filter: HistoryFilter,
    #[serde(default)]
    pub offset: u32,
    #[serde(default = "default_history_page_size")]
    pub limit: u32,
}

impl Default for HistoryQuery {
    fn default() -> Self {
        Self {
            filter: HistoryFilter::default(),
            offset: 0,
            limit: DEFAULT_HISTORY_PAGE_SIZE,
        }
    }
}

impl HistoryQuery {
    pub fn bounded_limit(&self) -> usize {
        self.limit.clamp(1, MAX_HISTORY_PAGE_SIZE) as usize
    }
}

fn default_history_page_size() -> u32 {
    DEFAULT_HISTORY_PAGE_SIZE
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryPage {
    pub items: Vec<DeliveryRecord>,
    pub total: usize,
    pub has_more: bool,
}

pub trait HistoryStore: Send {
    fn push(&mut self, record: DeliveryRecord) -> Result<(), HistoryError>;
    fn recent(&self) -> &[DeliveryRecord];
    /// Look up a record by id without mutating the store. Returns a clone so
    /// callers can diff-then-write without racing the borrow checker. The
    /// WS4 vocab hook uses this to grab the old transcript under the same
    /// lock the upcoming `update_transcript` takes, so the extracted
    /// suggestions never observe a torn read.
    fn find(&self, delivery_id: &str) -> Option<DeliveryRecord>;
    fn update_transcript(
        &mut self,
        delivery_id: &str,
        transcript: String,
    ) -> Result<bool, HistoryError>;
    /// Create-or-replace the single draft row for `session_id` with the
    /// latest committed-transcript text (crash-recovery draft history rows).
    /// Exactly one row per session — repeated calls during a recording
    /// update the same row, never accumulate. See
    /// `crate::delivery::draft_delivery_id` / `DeliveryRecord::new_draft`.
    fn upsert_draft(&mut self, session_id: &str, text: String) -> Result<(), HistoryError>;
    fn delete(&mut self, delivery_id: &str) -> Result<bool, HistoryError>;
    fn query(&self, query: &HistoryQuery) -> Result<HistoryPage, HistoryError>;
    fn delete_matching(&mut self, filter: &HistoryFilter) -> Result<usize, HistoryError>;
    fn prune_before(&mut self, cutoff_ms: u64) -> Result<usize, HistoryError>;
    fn clear(&mut self) -> Result<(), HistoryError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryError(String);

impl HistoryError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl std::fmt::Display for HistoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for HistoryError {}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryHistoryStore {
    records: Vec<DeliveryRecord>,
}

impl MemoryHistoryStore {
    pub fn push(&mut self, record: DeliveryRecord) -> Result<(), HistoryError> {
        <Self as HistoryStore>::push(self, record)
    }

    pub fn recent(&self) -> &[DeliveryRecord] {
        <Self as HistoryStore>::recent(self)
    }

    pub fn query(&self, query: &HistoryQuery) -> Result<HistoryPage, HistoryError> {
        <Self as HistoryStore>::query(self, query)
    }

    pub fn delete_matching(&mut self, filter: &HistoryFilter) -> Result<usize, HistoryError> {
        <Self as HistoryStore>::delete_matching(self, filter)
    }

    pub fn upsert_draft(&mut self, session_id: &str, text: String) -> Result<(), HistoryError> {
        <Self as HistoryStore>::upsert_draft(self, session_id, text)
    }
}

impl HistoryStore for MemoryHistoryStore {
    fn push(&mut self, record: DeliveryRecord) -> Result<(), HistoryError> {
        self.records.insert(0, record);
        trim_records(&mut self.records);
        Ok(())
    }

    fn recent(&self) -> &[DeliveryRecord] {
        &self.records
    }

    fn find(&self, delivery_id: &str) -> Option<DeliveryRecord> {
        self.records
            .iter()
            .find(|r| r.delivery_id == delivery_id)
            .cloned()
    }

    fn update_transcript(
        &mut self,
        delivery_id: &str,
        transcript: String,
    ) -> Result<bool, HistoryError> {
        if let Some(record) = self
            .records
            .iter_mut()
            .find(|r| r.delivery_id == delivery_id)
        {
            record.transcript = transcript;
            return Ok(true);
        }
        Ok(false)
    }

    fn upsert_draft(&mut self, session_id: &str, text: String) -> Result<(), HistoryError> {
        let delivery_id = crate::delivery::draft_delivery_id(session_id);
        if let Some(existing) = self
            .records
            .iter_mut()
            .find(|r| r.delivery_id == delivery_id)
        {
            existing.transcript = text;
            existing.delivered_at_ms = crate::delivery::now_ms();
            return Ok(());
        }
        self.records
            .insert(0, DeliveryRecord::new_draft(session_id, text));
        trim_records(&mut self.records);
        Ok(())
    }

    fn delete(&mut self, delivery_id: &str) -> Result<bool, HistoryError> {
        let before = self.records.len();
        self.records
            .retain(|record| record.delivery_id != delivery_id);
        Ok(self.records.len() != before)
    }

    fn query(&self, query: &HistoryQuery) -> Result<HistoryPage, HistoryError> {
        Ok(query_records(&self.records, query))
    }

    fn delete_matching(&mut self, filter: &HistoryFilter) -> Result<usize, HistoryError> {
        let before = self.records.len();
        self.records
            .retain(|record| !record_matches(record, filter));
        Ok(before - self.records.len())
    }

    fn prune_before(&mut self, cutoff_ms: u64) -> Result<usize, HistoryError> {
        let before = self.records.len();
        self.records
            .retain(|record| record.delivered_at_ms >= cutoff_ms);
        Ok(before - self.records.len())
    }

    fn clear(&mut self) -> Result<(), HistoryError> {
        self.records.clear();
        Ok(())
    }
}

pub fn query_records(records: &[DeliveryRecord], query: &HistoryQuery) -> HistoryPage {
    let matching: Vec<_> = records
        .iter()
        .filter(|record| record_matches(record, &query.filter))
        .cloned()
        .collect();
    let total = matching.len();
    let offset = query.offset as usize;
    let limit = query.bounded_limit();
    let items = matching.into_iter().skip(offset).take(limit).collect();
    HistoryPage {
        items,
        total,
        has_more: offset.saturating_add(limit) < total,
    }
}

pub fn record_matches(record: &DeliveryRecord, filter: &HistoryFilter) -> bool {
    if filter.cancelled_only {
        // Cancelled tab: only cancelled records (drafts excluded too).
        if !matches!(record.outcome, crate::delivery::DeliveryOutcome::Cancelled) {
            return false;
        }
    } else if matches!(
        record.outcome,
        crate::delivery::DeliveryOutcome::Cancelled | crate::delivery::DeliveryOutcome::Draft
    ) {
        // Dictations tab: hide both cancelled and in-progress drafts.
        return false;
    }

    if filter
        .from_ms
        .is_some_and(|from| record.delivered_at_ms < from)
        || filter
            .until_ms
            .is_some_and(|until| record.delivered_at_ms >= until)
    {
        return false;
    }

    if !filter.app.trim().is_empty()
        && !history_target_label(record).eq_ignore_ascii_case(filter.app.trim())
    {
        return false;
    }

    let search = filter.search.trim().to_ascii_lowercase();
    if search.is_empty() {
        return true;
    }
    let haystack = serde_json::to_string(record)
        .unwrap_or_default()
        .to_ascii_lowercase();
    haystack.contains(&search)
}

pub fn history_target_label(record: &DeliveryRecord) -> String {
    match record.source.as_ref() {
        Some(crate::delivery::DeliverySource::FileTranscription { file_name }) => {
            if file_name.is_empty() {
                "File transcription".to_string()
            } else {
                format!("File · {file_name}")
            }
        }
        _ => record
            .target
            .process_name
            .clone()
            .or_else(|| record.target.window_title.clone())
            .unwrap_or_else(|| "Unknown app".to_string()),
    }
}

/// Cap the in-memory ring to `MAX_HISTORY_ITEMS` (newest-first; truncates the
/// tail). `pub` so the disk-backed stores in `src-tauri` reuse it.
pub fn trim_records(records: &mut Vec<DeliveryRecord>) {
    if records.len() > MAX_HISTORY_ITEMS {
        records.truncate(MAX_HISTORY_ITEMS);
    }
}

/// Pure migration-gate decision. Returns true when it's safe to shred the
/// plaintext `history.json`: the SQLCipher DB row count must equal the number
/// of DISTINCT `delivery_id`s in the JSON. `INSERT OR IGNORE` collapses
/// duplicate ids (and drops any record whose serialization failed), so compare
/// against the distinct count — not `records.len()`. If any record failed to
/// migrate, `row_count < distinct` and the caller keeps the `.bak`.
pub fn migration_verified(records: &[DeliveryRecord], db_row_count: i64) -> bool {
    let distinct: std::collections::HashSet<&str> =
        records.iter().map(|r| r.delivery_id.as_str()).collect();
    db_row_count >= 0 && db_row_count == distinct.len() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::delivery::{DeliveryId, DeliveryOutcome};

    fn rec(id: &str) -> DeliveryRecord {
        DeliveryRecord::for_test(DeliveryId::new(id), "t", DeliveryOutcome::Pasted)
    }

    #[test]
    fn memory_store_push_is_newest_first_and_delete_works() {
        let mut s = MemoryHistoryStore::default();
        s.push(rec("a")).unwrap();
        s.push(rec("b")).unwrap();
        assert_eq!(s.recent()[0].delivery_id, "b");
        assert!(s.delete("a").unwrap());
        assert!(!s.delete("a").unwrap());
        assert_eq!(s.recent().len(), 1);
    }

    #[test]
    fn memory_store_updates_transcript_by_id() {
        let mut s = MemoryHistoryStore::default();
        s.push(rec("a")).unwrap();
        assert!(s.update_transcript("a", "fixed".into()).unwrap());
        assert_eq!(s.recent()[0].transcript, "fixed");
        assert!(!s.update_transcript("missing", "x".into()).unwrap());
    }

    #[test]
    fn trim_caps_at_max() {
        let mut v: Vec<DeliveryRecord> = (0..MAX_HISTORY_ITEMS + 50)
            .map(|i| rec(&format!("id-{i}")))
            .collect();
        trim_records(&mut v);
        assert_eq!(v.len(), MAX_HISTORY_ITEMS);
    }

    #[test]
    fn migration_verified_uses_distinct_ids() {
        let records = vec![rec("dup"), rec("dup"), rec("unique")];
        // 2 distinct ids → DB should have 2 rows (INSERT OR IGNORE collapsed the dup).
        assert!(migration_verified(&records, 2));
        // Fewer rows than distinct → a record failed to migrate → keep .bak.
        assert!(!migration_verified(&records, 1));
        // Negative (query failed) → never safe to shred.
        assert!(!migration_verified(&records, -1));
    }

    #[test]
    fn dictations_tab_hides_cancelled_and_cancelled_tab_shows_only_cancelled() {
        let mut store = MemoryHistoryStore::default();
        let mut cancelled = rec("cancelled-1");
        cancelled.outcome = DeliveryOutcome::Cancelled;
        store.push(cancelled).unwrap();
        store.push(rec("delivered-1")).unwrap();

        // Dictations tab (default): cancelled hidden.
        let dictations = store.query(&HistoryQuery::default()).unwrap();
        assert_eq!(dictations.total, 1, "cancelled record must be hidden by default");
        assert_eq!(dictations.items[0].delivery_id, "delivered-1");

        // Cancelled tab: ONLY the cancelled record (not the delivered one) — this
        // is the fix for the "50 of 59" count bug: the count must equal exactly
        // the rows shown, so the tab is a server-side exclusive scope.
        let cancelled_tab = store
            .query(&HistoryQuery {
                filter: HistoryFilter {
                    cancelled_only: true,
                    ..HistoryFilter::default()
                },
                ..HistoryQuery::default()
            })
            .unwrap();
        assert_eq!(cancelled_tab.total, 1, "cancelled tab shows only cancelled");
        assert_eq!(cancelled_tab.items[0].delivery_id, "cancelled-1");

        // Bulk delete respects the same default: an empty (match-all) filter
        // must not sweep up cancelled records.
        let deleted = store.delete_matching(&HistoryFilter::default()).unwrap();
        assert_eq!(deleted, 1, "default filtered delete must skip cancelled");
        assert_eq!(store.recent().len(), 1);
        assert_eq!(store.recent()[0].outcome, DeliveryOutcome::Cancelled);
    }

    #[test]
    fn draft_records_are_hidden_in_both_tabs() {
        let mut store = MemoryHistoryStore::default();
        store
            .push(DeliveryRecord::new_draft("session-1", "partial so far"))
            .unwrap();
        store.push(rec("delivered-1")).unwrap();

        // Dictations tab: draft hidden.
        let dictations = store.query(&HistoryQuery::default()).unwrap();
        assert_eq!(dictations.total, 1, "draft hidden on the dictations tab");
        assert_eq!(dictations.items[0].delivery_id, "delivered-1");

        // Cancelled tab: drafts are internal crash-recovery snapshots, never a
        // user-visible outcome (a real crash sweeps them to Cancelled first), so
        // a raw draft never surfaces here either.
        let cancelled_tab = store
            .query(&HistoryQuery {
                filter: HistoryFilter {
                    cancelled_only: true,
                    ..HistoryFilter::default()
                },
                ..HistoryQuery::default()
            })
            .unwrap();
        assert_eq!(cancelled_tab.total, 0, "raw draft never surfaces on the cancelled tab");

        let deleted = store.delete_matching(&HistoryFilter::default()).unwrap();
        assert_eq!(deleted, 1, "default filtered delete must skip the draft");
        assert_eq!(store.recent().len(), 1);
        assert_eq!(store.recent()[0].outcome, DeliveryOutcome::Draft);
    }

    #[test]
    fn upsert_draft_replaces_the_same_session_row_instead_of_accumulating() {
        let mut store = MemoryHistoryStore::default();
        store.upsert_draft("session-1", "hello".to_string()).unwrap();
        store.upsert_draft("session-1", "hello world".to_string()).unwrap();
        store.upsert_draft("session-1", "hello world done".to_string()).unwrap();

        // Drafts don't surface through any tab filter, so inspect raw rows.
        let rows = store.recent();
        assert_eq!(rows.len(), 1, "repeated upsert_draft must not accumulate rows");
        assert_eq!(rows[0].transcript, "hello world done");
        assert_eq!(rows[0].outcome, DeliveryOutcome::Draft);
    }

    #[test]
    fn upsert_draft_for_two_sessions_keeps_two_separate_rows() {
        let mut store = MemoryHistoryStore::default();
        store.upsert_draft("session-1", "first session".to_string()).unwrap();
        store.upsert_draft("session-2", "second session".to_string()).unwrap();

        assert_eq!(store.recent().len(), 2);
    }

    #[test]
    fn retention_cutoff_supports_locked_options_and_saturates() {
        const DAY_MS: u64 = 24 * 60 * 60 * 1_000;
        let now = 400 * DAY_MS;
        assert_eq!(HistoryRetention::Forever.cutoff_ms(now), None);
        assert_eq!(
            HistoryRetention::SevenDays.cutoff_ms(now),
            Some(393 * DAY_MS)
        );
        assert_eq!(
            HistoryRetention::ThirtyDays.cutoff_ms(now),
            Some(370 * DAY_MS)
        );
        assert_eq!(
            HistoryRetention::NinetyDays.cutoff_ms(now),
            Some(310 * DAY_MS)
        );
        assert_eq!(HistoryRetention::OneYear.cutoff_ms(now), Some(35 * DAY_MS));
        assert_eq!(HistoryRetention::OneYear.cutoff_ms(DAY_MS), Some(0));
    }

    #[test]
    fn memory_query_paginates_and_bulk_delete_matches_filter() {
        let mut store = MemoryHistoryStore::default();
        for (id, text, timestamp) in [
            ("a", "alpha project", 1_000),
            ("b", "beta project", 2_000),
            ("c", "alpha follow-up", 3_000),
        ] {
            let mut record = rec(id);
            record.transcript = text.to_string();
            record.delivered_at_ms = timestamp;
            store.push(record).unwrap();
        }

        let query = HistoryQuery {
            filter: HistoryFilter {
                search: "alpha".to_string(),
                ..HistoryFilter::default()
            },
            offset: 1,
            limit: 1,
        };
        let page = store.query(&query).unwrap();
        assert_eq!(page.total, 2);
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].delivery_id, "a");
        assert!(!page.has_more);

        let deleted = store.delete_matching(&query.filter).unwrap();
        assert_eq!(deleted, 2);
        assert_eq!(store.recent().len(), 1);
        assert_eq!(store.recent()[0].delivery_id, "b");
    }
}
