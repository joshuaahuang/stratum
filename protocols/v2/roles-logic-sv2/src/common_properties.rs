//! Traits that implements very basic properties that every implementation should implements
use crate::selectors::{
    DownstreamMiningSelector, DownstreamSelector, NullDownstreamMiningSelector,
};
use common_messages_sv2::{has_requires_std_job, Protocol, SetupConnection};
use mining_sv2::{Extranonce, Target};
use std::{collections::HashMap, fmt::Debug as D};

/// What define a mining downstream node at the very basic
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub struct CommonDownstreamData {
    pub header_only: bool,
    pub work_selection: bool,
    pub version_rolling: bool,
}

/// SetupConnection sugared
#[derive(Debug, Copy, Clone)]
pub struct PairSettings {
    pub protocol: Protocol,
    pub min_v: u16,
    pub max_v: u16,
    pub flags: u32,
}

pub trait IsUpstream<Down: IsDownstream, Sel: DownstreamSelector<Down> + ?Sized> {
    fn get_version(&self) -> u16;
    fn get_flags(&self) -> u32;
    fn get_supported_protocols(&self) -> Vec<Protocol>;
    fn is_pairable(&self, pair_settings: &PairSettings) -> bool {
        let protocol = pair_settings.protocol;
        let min_v = pair_settings.min_v;
        let max_v = pair_settings.max_v;
        let flags = pair_settings.flags;

        let check_version = self.get_version() >= min_v && self.get_version() <= max_v;
        let check_flags = SetupConnection::check_flags(protocol, flags, self.get_flags());
        check_version && check_flags
    }
    fn get_id(&self) -> u32;
    fn get_mapper(&mut self) -> Option<&mut RequestIdMapper>;
    fn get_remote_selector(&mut self) -> &mut Sel;
}

/// Channel to be opened with the upstream nodes.
#[derive(Debug, Clone, Copy)]
pub enum UpstreamChannel {
    // nominal hash rate
    Standard(f32),
    Group,
    Extended,
}

/// Channel to be opened with the downstream nodes.
#[derive(Debug, Clone)]
pub enum DownstreamChannel {
    // channel id, target, extranonce prefix, group channel id
    Standard(StandardChannel),
    Group(u32),
    Extended(u32),
}

impl DownstreamChannel {
    /// Returns the group id for a given Standard, Group, or Extended, channel.
    pub fn group_id(&self) -> u32 {
        match self {
            DownstreamChannel::Standard(s) => s.group_id,
            DownstreamChannel::Group(id) => *id,
            // Extended channels do not have group ids better to fail whenever why try to get a
            // group id on an extended channel
            DownstreamChannel::Extended(_) => panic!("Extended channels do not have group ids"),
        }
    }

    /// Returns the channel id for a given Standard, Group, or Extended, channel.
    pub fn channel_id(&self) -> u32 {
        match self {
            DownstreamChannel::Standard(s) => s.channel_id,
            DownstreamChannel::Group(id) => *id,
            DownstreamChannel::Extended(id) => *id,
        }
    }
}

#[derive(Debug, Clone)]
/// Standard channels are intended to be used by end mining devices.
pub struct StandardChannel {
    /// Newly assigned identifier of the channel, stable for the whole lifetime of the connection.
    /// e.g. it is used for broadcasting new jobs by `NewExtendedMiningJob`
    pub channel_id: u32,
    /// Identifier of the group where the standard channel belongs
    pub group_id: u32,
    /// Initial target for the mining channel
    pub target: Target,
    /// Extranonce bytes which need to be added to the coinbase to form a fully valid submission:
    /// (full coinbase = coinbase_tx_prefix + extranonce_prefix + extranonce + coinbase_tx_suffix).
    pub extranonce: Extranonce,
}

/// General properties that every Sv2 compatible mining upstream nodes must implement.
pub trait IsMiningUpstream<Down: IsMiningDownstream, Sel: DownstreamMiningSelector<Down> + ?Sized>:
    IsUpstream<Down, Sel>
{
    fn total_hash_rate(&self) -> u64;
    fn add_hash_rate(&mut self, to_add: u64);
    fn get_opened_channels(&mut self) -> &mut Vec<UpstreamChannel>;
    fn update_channels(&mut self, c: UpstreamChannel);
    fn is_header_only(&self) -> bool {
        has_requires_std_job(self.get_flags())
    }
}

/// General properties that every Sv2 compatible mining downstream nodes must implement.
pub trait IsDownstream {
    fn get_downstream_mining_data(&self) -> CommonDownstreamData;
}

pub trait IsMiningDownstream: IsDownstream {
    fn is_header_only(&self) -> bool {
        self.get_downstream_mining_data().header_only
    }
}

/// Implemented for the NullDownstreamMiningSelector
impl<Down: IsDownstream + D> IsUpstream<Down, NullDownstreamMiningSelector> for () {
    fn get_version(&self) -> u16 {
        unreachable!("Null upstream do not have a version");
    }

    fn get_flags(&self) -> u32 {
        unreachable!("Null upstream do not have flags");
    }

    fn get_supported_protocols(&self) -> Vec<Protocol> {
        unreachable!("Null upstream do not support any protocol");
    }
    fn get_id(&self) -> u32 {
        unreachable!("Null upstream do not have an ID");
    }

    fn get_mapper(&mut self) -> Option<&mut RequestIdMapper> {
        unreachable!("Null upstream do not have a mapper")
    }

    fn get_remote_selector(&mut self) -> &mut NullDownstreamMiningSelector {
        unreachable!("Null upstream do not have a selector")
    }
}

/// Implemented for the NullDownstreamMiningSelector
impl IsDownstream for () {
    fn get_downstream_mining_data(&self) -> CommonDownstreamData {
        unreachable!("Null downstream do not have mining data");
    }
}

impl<Down: IsMiningDownstream + D> IsMiningUpstream<Down, NullDownstreamMiningSelector> for () {
    fn total_hash_rate(&self) -> u64 {
        unreachable!("Null selector do not have hash rate");
    }

    fn add_hash_rate(&mut self, _to_add: u64) {
        unreachable!("Null selector can not add hash rate");
    }
    fn get_opened_channels(&mut self) -> &mut Vec<UpstreamChannel> {
        unreachable!("Null selector can not open channels");
    }

    fn update_channels(&mut self, _: UpstreamChannel) {
        unreachable!("Null selector can not update channels");
    }
}

impl IsMiningDownstream for () {}

/// Proxies likely need to change the request ids of the downsteam's messages. They also need to
/// remember the original id to patch the upstream's response with it.
#[derive(Debug, Default, PartialEq)]
pub struct RequestIdMapper {
    /// Mapping of upstream id -> downstream ids
    request_ids_map: HashMap<u32, u32>,
    next_id: u32,
}

impl RequestIdMapper {
    /// Builds a new `RequestIdMapper` initialized with an empty hashmap and initializes `next_id`
    /// to `0`.
    pub fn new() -> Self {
        Self {
            request_ids_map: HashMap::new(),
            next_id: 0,
        }
    }

    /// Updates the `RequestIdMapper` with a new upstream/downstream mapping.
    pub fn on_open_channel(&mut self, id: u32) -> u32 {
        let new_id = self.next_id;
        self.next_id += 1;

        self.request_ids_map.insert(new_id, id);
        new_id
    }

    /// Removes a upstream/downstream mapping from the `RequsetIdMapper`.
    pub fn remove(&mut self, upstream_id: u32) -> Option<u32> {
        self.request_ids_map.remove(&upstream_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use binary_sv2::u256_from_int;

    #[test]
    fn builds_request_id_mapper() {
        let expect = RequestIdMapper {
            request_ids_map: HashMap::<u32, u32>::new(),
            next_id: 0,
        };
        let actual = RequestIdMapper::new();

        assert_eq!(expect, actual);
    }

    #[test]
    fn updates_request_id_mapper_on_open_channel() {
        let id = 0;
        let mut expect = RequestIdMapper {
            request_ids_map: HashMap::<u32, u32>::new(),
            next_id: id,
        };
        let new_id = expect.next_id;
        expect.next_id += 1;
        expect.request_ids_map.insert(new_id, id);

        let mut actual = RequestIdMapper::new();
        actual.on_open_channel(0);

        assert_eq!(expect, actual);
    }

    #[test]
    fn removes_id_from_request_id_mapper() {
        let mut request_id_mapper = RequestIdMapper::new();
        request_id_mapper.on_open_channel(0);
        assert!(!request_id_mapper.request_ids_map.is_empty());

        request_id_mapper.remove(0);
        assert!(request_id_mapper.request_ids_map.is_empty());
    }

    #[test]
    fn downstream_channel_returns_group_id_on_receiving_standard_channel() {
        let expect = 0;

        let channel = DownstreamChannel::Standard(StandardChannel {
            channel_id: 0,
            group_id: 0,
            target: u256_from_int(45_u32).into(),
            extranonce: mining_sv2::Extranonce::new(),
        });
        let actual = channel.group_id();

        assert_eq!(expect, actual);
    }

    #[test]
    fn downstream_channel_returns_group_id_on_receiving_group_channel() {
        let id = 0;
        let expect = id;

        let channel = DownstreamChannel::Group(id);
        let actual = channel.group_id();

        assert_eq!(expect, actual);
    }

    #[ignore]
    #[test]
    fn downstream_channel_returns_group_id_on_receiving_extended_channel() {
        todo!();
    }

    #[test]
    fn downstream_channel_returns_channel_id_on_receiving_standard_channel() {
        let expect = 0;

        let channel = DownstreamChannel::Standard(StandardChannel {
            channel_id: 0,
            group_id: 0,
            target: u256_from_int(45_u32).into(),
            extranonce: mining_sv2::Extranonce::new(),
        });
        let actual = channel.channel_id();

        assert_eq!(expect, actual);
    }

    #[test]
    fn downstream_channel_returns_channel_id_on_receiving_group_channel() {
        let id = 0;
        let expect = id;

        let channel = DownstreamChannel::Group(id);
        let actual = channel.channel_id();

        assert_eq!(expect, actual);
    }

    #[ignore]
    #[test]
    fn downstream_channel_returns_channel_id_on_receiving_extended_channel() {
        todo!();
    }
}
