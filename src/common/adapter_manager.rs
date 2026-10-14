/// Implements common functionality for adapters across platforms.
// btleplug Source Code File
//
// Copyright 2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.
//
// Some portions of this file are taken and/or modified from Rumble
// (https://github.com/mwylde/rumble), using a dual MIT/Apache License under the
// following copyright:
//
// Copyright (c) 2014 The Rust Project Developers
use crate::api::{CentralEvent, Peripheral};
use crate::platform::PeripheralId;
use dashmap::{mapref::one::RefMut, DashMap};
use futures::stream::{Stream, StreamExt};
use log::trace;
use std::pin::Pin;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

#[cfg(any(target_os = "android", test))]
fn prune_cache<K: Eq + std::hash::Hash, V>(
    cache: &DashMap<K, V>,
    limit: usize,
    mut is_reclaimable: impl FnMut(&V) -> bool,
) {
    let mut remaining = cache.len().saturating_sub(limit);
    if remaining == 0 {
        return;
    }
    cache.retain(|_, value| {
        if remaining > 0 && is_reclaimable(value) {
            remaining -= 1;
            false
        } else {
            true
        }
    });
}

#[derive(Debug)]
pub struct AdapterManager<PeripheralType>
where
    PeripheralType: Peripheral,
{
    peripherals: DashMap<PeripheralId, PeripheralType>,
    events_channel: broadcast::Sender<CentralEvent>,
}

#[cfg(test)]
mod cache_tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn discovery_cache_releases_only_unowned_entries_above_limit() {
        let cache = DashMap::new();
        let idle = Arc::new(1);
        let idle_weak = Arc::downgrade(&idle);
        cache.insert(1, idle);
        let held = Arc::new(2);
        cache.insert(2, held.clone());
        let connected = Arc::new(3);
        cache.insert(3, connected);

        prune_cache(&cache, 2, |entry| {
            Arc::strong_count(entry) == 1 && **entry != 3
        });

        assert!(idle_weak.upgrade().is_none());
        assert_eq!(cache.len(), 2);
        assert_eq!(**cache.get(&2).unwrap(), 2);
        assert_eq!(**cache.get(&3).unwrap(), 3);
        assert_eq!(*held, 2);
    }

    #[test]
    fn discovery_cache_keeps_results_below_limit_and_all_in_use_entries() {
        let cache = DashMap::new();
        cache.insert(1, Arc::new(1));
        prune_cache(&cache, 2, |_| true);
        assert_eq!(**cache.get(&1).unwrap(), 1);

        let held = Arc::new(2);
        cache.insert(2, held.clone());
        prune_cache(&cache, 0, |entry| Arc::strong_count(entry) == 1);
        assert_eq!(cache.len(), 1);
        assert_eq!(**cache.get(&2).unwrap(), 2);
    }

    #[test]
    fn discovery_cache_stays_bounded_across_many_new_devices() {
        let cache = DashMap::new();
        let mut weak_entries = Vec::new();
        for id in 0..512 {
            let peripheral = Arc::new(id);
            weak_entries.push(Arc::downgrade(&peripheral));
            cache.insert(id, peripheral.clone());
            prune_cache(&cache, 256, |entry| Arc::strong_count(entry) == 1);
        }
        assert_eq!(cache.len(), 256);
        assert_eq!(
            weak_entries
                .iter()
                .filter(|entry| entry.upgrade().is_none())
                .count(),
            256
        );
        assert_eq!(**cache.get(&511).unwrap(), 511);
    }

    #[test]
    fn discovery_cache_reclaims_previously_held_entries_on_next_prune() {
        let cache = DashMap::new();
        let held = Arc::new(7);
        let weak = Arc::downgrade(&held);
        cache.insert(1, held.clone());
        prune_cache(&cache, 0, |entry| Arc::strong_count(entry) == 1);
        assert_eq!(**cache.get(&1).unwrap(), 7);
        drop(held);
        prune_cache(&cache, 0, |entry| Arc::strong_count(entry) == 1);
        assert_eq!(cache.len(), 0);
        assert!(weak.upgrade().is_none());
    }
}

impl<PeripheralType: Peripheral + 'static> Default for AdapterManager<PeripheralType> {
    fn default() -> Self {
        let (broadcast_sender, _) = broadcast::channel(16);
        AdapterManager {
            peripherals: DashMap::new(),
            events_channel: broadcast_sender,
        }
    }
}

impl<PeripheralType> AdapterManager<PeripheralType>
where
    PeripheralType: Peripheral + 'static,
{
    pub fn emit(&self, event: CentralEvent) {
        if let CentralEvent::DeviceDisconnected(ref id) = event {
            self.peripherals.remove(id);
        }

        if let Err(lost) = self.events_channel.send(event) {
            trace!("Lost central event, while nothing subscribed: {:?}", lost);
        }
    }

    pub fn event_stream(&self) -> Pin<Box<dyn Stream<Item = CentralEvent> + Send>> {
        let receiver = self.events_channel.subscribe();
        Box::pin(BroadcastStream::new(receiver).filter_map(|x| async move { x.ok() }))
    }

    pub fn add_peripheral(&self, peripheral: PeripheralType) {
        assert!(
            !self.peripherals.contains_key(&peripheral.id()),
            "Adding a peripheral that's already in the map."
        );
        self.peripherals.insert(peripheral.id(), peripheral);
    }

    pub fn peripherals(&self) -> Vec<PeripheralType> {
        self.peripherals
            .iter()
            .map(|val| val.value().clone())
            .collect()
    }

    #[cfg(target_os = "android")]
    pub(crate) fn prune_peripherals(
        &self,
        limit: usize,
        is_reclaimable: impl FnMut(&PeripheralType) -> bool,
    ) {
        prune_cache(&self.peripherals, limit, is_reclaimable);
    }

    // Only used on windows and macOS/iOS, so turn off deadcode so we don't get warnings on android/linux.
    #[allow(dead_code)]
    pub fn peripheral_mut(
        &self,
        id: &PeripheralId,
    ) -> Option<RefMut<PeripheralId, PeripheralType>> {
        self.peripherals.get_mut(id)
    }

    pub fn peripheral(&self, id: &PeripheralId) -> Option<PeripheralType> {
        self.peripherals.get(id).map(|val| val.value().clone())
    }
}
