use serde::Serialize;
use std::collections::HashSet;
use std::iter;
use strum;
use strum::{EnumIter, EnumString};
use uuid::Uuid;

#[derive(Serialize, Clone)]
pub struct Player {
    uuid: Uuid,
    name: String,
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Clone)]
pub struct BreakCount(u64);

#[derive(PartialEq, PartialOrd, Eq, Ord, Clone)]
pub struct BuildCount(u64);

#[derive(PartialEq, PartialOrd, Eq, Ord, Clone)]
pub struct PlayTicks(u64);

#[derive(PartialEq, PartialOrd, Eq, Ord, Clone)]
pub struct VoteCount(u64);

pub trait AggregatedPlayerAttribution: Ord + Clone {}

impl AggregatedPlayerAttribution for BreakCount {}
impl AggregatedPlayerAttribution for BuildCount {}
impl AggregatedPlayerAttribution for PlayTicks {}
impl AggregatedPlayerAttribution for VoteCount {}

#[derive(Clone)]
pub struct AttributionRecord<Attribution: AggregatedPlayerAttribution> {
    pub player: Player,
    pub attribution: Attribution,
}

#[derive(Clone)]
pub struct RankedAttributionRecord<Attribution: AggregatedPlayerAttribution> {
    pub rank: u32,
    pub attribution_record: AttributionRecord<Attribution>,
}

pub struct Ranking<Attribution: AggregatedPlayerAttribution> {
    /// invariant: these records are sorted and given "ranks" so that
    ///  - `.rank` is increasing
    ///  - for each index i, either
    ///    - `sorted_ranked_records[i].rank.0` equals `i + 1` (i.e. there is only one record with rank i + 1), or
    ///    - there is some r < i such that `sorted_ranked_records[j].rank.0 == r + 1` for all r ≤ j ≤ i (i.e. there are ties)
    sorted_ranked_records: Vec<RankedAttributionRecord<Attribution>>,
}

pub struct RankingSlice<Attribution: AggregatedPlayerAttribution>(
    pub Vec<RankedAttributionRecord<Attribution>>,
);

impl<Attribution: AggregatedPlayerAttribution + Clone> Default for Ranking<Attribution> {
    fn default() -> Self {
        Ranking {
            sorted_ranked_records: vec![],
        }
    }
}

impl<Attribution: AggregatedPlayerAttribution + Clone> Ranking<Attribution> {
    pub fn hydrate_record_set(&mut self, records: HashSet<AttributionRecord<Attribution>>) {
        struct ScanState<Attribution> {
            next_item_index: usize,
            previous_attribution: Attribution,
            previous_item_rank: u32,
        }

        let mut records = records.into_iter().collect::<Vec<_>>();
        records.sort_by_key(|ar| ar.attribution.clone());
        records.reverse();

        let (first_record, tail_records) = match records.as_slice() {
            [first, rest @ ..] => (first, rest),
            [] => {
                self.sorted_ranked_records = vec![];
                return;
            }
        };

        let first_ranked_record = RankedAttributionRecord {
            rank: 1,
            attribution_record: first_record.clone(),
        };

        let initial_scan_state = ScanState {
            next_item_index: 0,
            previous_attribution: first_record.attribution.clone(),
            previous_item_rank: 1,
        };

        let ranked_tail_records = tail_records.iter().scan(initial_scan_state, |st, record| {
            let next_rank = if st.previous_attribution == record.attribution {
                st.previous_item_rank
            } else {
                assert!(st.previous_attribution < record.attribution);
                (st.next_item_index as u32) + 1
            };

            let next_ranked_record = RankedAttributionRecord {
                rank: next_rank,
                attribution_record: record.clone(),
            };

            st.next_item_index += 1;
            st.previous_attribution = record.attribution.clone();
            st.previous_item_rank = next_rank;

            Some(next_ranked_record)
        });

        self.sorted_ranked_records = iter::once(first_ranked_record)
            .chain(ranked_tail_records)
            .collect()
    }

    pub fn paginate(&self, offset: usize, limit: usize) -> RankingSlice<Attribution> {
        RankingSlice(self.sorted_ranked_records.as_slice()[offset..(offset + limit)].to_vec())
    }
}

pub trait AttributionRecordProvider<Attribution: AggregatedPlayerAttribution> {
    fn get_all_attribution_records(self) -> Vec<AttributionRecord<Attribution>>;
}

#[derive(Debug, PartialEq, EnumString, EnumIter)]
#[strum(serialize_all = "snake_case")]
pub enum AggregationTimeRange {
    All,
    LastOneYear,
    LastOneMonth,
    LastOneWeek,
    LastOneDay,
}
