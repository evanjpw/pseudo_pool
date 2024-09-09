use crate::error::PseudoPoolError;
use crate::Result;
use crossbeam_channel;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::Duration;
use uuid::Uuid;

const POOL_POLLING_TIMEOUT: Duration = Duration::from_secs(5);

type PoolEntryId = Uuid;

struct PoolEntry<T> {
    pool_entry_id: PoolEntryId,
    payload: Arc<RwLock<T>>,
}

impl<T> PoolEntry<T> {
    fn new(payload: T) -> Self {
        let pool_entry_id = Uuid::new_v4();

        Self {
            pool_entry_id,
            payload: Arc::new(RwLock::new(payload)),
        }
    }
}

impl<T> Clone for PoolEntry<T> {
    fn clone(&self) -> Self {
        Self {
            pool_entry_id: self.pool_entry_id.clone(),
            payload: self.payload.clone(),
        }
    }
}

pub struct ExternalPoolEntry<T> {
    pool_entry: PoolEntry<T>,
    notifier: crossbeam_channel::Sender<PoolEntryId>,
    // Prevent construction
    phantom: PhantomData<()>,
}

impl<T> ExternalPoolEntry<T> {
    fn new(pool_entry: PoolEntry<T>, notifier: crossbeam_channel::Sender<PoolEntryId>) -> Self {
        ExternalPoolEntry {
            pool_entry,
            notifier,
            phantom: PhantomData,
        }
    }

    pub fn get_payload(&self) -> RwLockReadGuard<T> {
        self.pool_entry.payload.read().unwrap()
    }

    pub fn get_payload_mut(&mut self) -> RwLockWriteGuard<T> {
        self.pool_entry.payload.write().unwrap()
    }
}

impl<T> Drop for ExternalPoolEntry<T> {
    fn drop(&mut self) {
        let id = self.pool_entry.pool_entry_id;
        self.notifier.send(id).unwrap()
    }
}

struct InternalPoolEntry<T> {
    pool_entry: PoolEntry<T>,
    in_use: AtomicBool,
}

impl<T> InternalPoolEntry<T> {
    fn new(payload: T) -> Self {
        Self {
            pool_entry: PoolEntry::new(payload),
            in_use: AtomicBool::new(false),
        }
    }
}

pub struct Pool<T> {
    map: HashMap<PoolEntryId, InternalPoolEntry<T>>,
    notification_sender: crossbeam_channel::Sender<PoolEntryId>,
    notification_receiver: crossbeam_channel::Receiver<PoolEntryId>,
}

impl<T> Pool<T> {
    pub fn new() -> Self {
        let (notification_sender, notification_receiver) = crossbeam_channel::unbounded();
        Self {
            map: HashMap::new(),
            notification_sender,
            notification_receiver,
        }
    }

    pub fn new_from_iterable<V: IntoIterator<Item = T>>(vec: V) -> Self {
        let mut pool = Self::new();
        pool.extend_entries(vec);
        pool
    }

    pub fn add_entry(&mut self, payload: T) {
        let entry = InternalPoolEntry::new(payload);
        self.map.insert(entry.pool_entry.pool_entry_id, entry);
    }

    pub fn extend_entries<V: IntoIterator<Item = T>>(&mut self, vec: V) {
        for payload in vec {
            self.add_entry(payload);
        }
    }

    fn get_external_entry(&mut self, entry: PoolEntry<T>) -> ExternalPoolEntry<T> {
        ExternalPoolEntry::new(entry, self.notification_sender.clone())
    }

    pub fn checkout_blocking(&mut self) -> Result<ExternalPoolEntry<T>> {
        loop {
            if let Some(entry) = self.try_checkout() {
                return Ok(entry);
            }

            let entry_id = self
                .notification_receiver
                .recv_timeout(POOL_POLLING_TIMEOUT);

            if let Ok(entry_id) = entry_id {
                self.checkin(entry_id)?;
            } else {
                // TODO: Detect a real error & return it?
            }
        }
    }

    pub fn try_checkout(&mut self) -> Option<ExternalPoolEntry<T>> {
        self.process_checkins();
        for (_, entry) in self.map.iter_mut() {
            if let Ok(in_use) =
                entry
                    .in_use
                    .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            {
                assert!(!in_use);
                let pool_entry = entry.pool_entry.clone();
                return Some(self.get_external_entry(pool_entry));
            }
        }
        None
    }

    fn process_checkins(&mut self) {
        if self.notification_receiver.is_empty() {
            return;
        }
        loop {
            let entry_id = self.notification_receiver.try_recv();
            if let Ok(entry_id) = entry_id {
                self.checkin(entry_id).unwrap()
            } else {
                return;
            }
        }
    }

    fn checkin(&mut self, entry_id: PoolEntryId) -> Result<()> {
        let entry = self.map.get(&entry_id);
        if let Some(entry) = entry {
            entry.in_use.store(false, Ordering::Release);
            Ok(())
        } else {
            Err(PseudoPoolError::InvalidCheckin(entry_id))
        }
    }

    pub fn update_leases(&mut self) -> usize {
        self.process_checkins();
        self.leases()
    }

    pub fn leases(&self) -> usize {
        self.map
            .iter()
            .filter(|(_, entry)| !entry.in_use.load(Ordering::Acquire))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_non_blocking() {
        let mut pool = Pool::new();
        pool.add_entry(String::from("test"));
        pool.add_entry(String::from("test2"));
        pool.add_entry(String::from("test3"));
        pool.add_entry(String::from("test4"));
        pool.add_entry(String::from("test5"));

        assert_eq!(5, pool.leases());

        let l1 = pool.try_checkout().unwrap();
        assert_eq!(pool.leases(), 4);
        let l2 = pool.try_checkout().unwrap();
        assert_eq!(pool.update_leases(), 3);
        assert_ne!(*l1.get_payload(), *l2.get_payload());
        drop(l1);
        assert_eq!(pool.leases(), 3);
        assert_eq!(pool.update_leases(), 4);
        assert_eq!(pool.leases(), 4);
        let l1a = pool.try_checkout().unwrap();
        assert_eq!(pool.leases(), 3);
        let l2_value = (*l2.get_payload()).clone();
        drop(l2);
        assert_eq!(pool.update_leases(), 4);
        let l2a = pool.try_checkout().unwrap();
        assert_eq!(*l2a.get_payload(), l2_value);
        assert_eq!(pool.leases(), 3);
        let l3 = pool.try_checkout().unwrap();
        assert_ne!(*l3.get_payload(), l2_value);
        assert_eq!(pool.leases(), 2);
        let l4 = pool.try_checkout().unwrap();
        assert_eq!(pool.leases(), 1);
        let l5 = pool.try_checkout().unwrap();
        assert_ne!(*l5.get_payload(), *l4.get_payload());
        assert_eq!(pool.leases(), 0);
        let l0 = pool.try_checkout();
        assert!(l0.is_none());
        let l1a_value = (*l1a.get_payload()).clone();
        drop(l1a);
        assert_eq!(pool.leases(), 0);
        let l1_returns = pool.try_checkout().unwrap();
        assert_eq!(pool.leases(), 0);
        assert_eq!(*l1_returns.get_payload(), l1a_value);
    }

    #[test]
    fn test_blocking() {
        let mut pool = Pool::new_from_iterable(vec![String::from("test1"), String::from("test2")]);
        pool.extend_entries(vec![String::from("test3"), String::from("test4")]);
        assert_eq!(pool.leases(), 4);
        let l1 = pool.checkout_blocking().unwrap();
        assert_eq!(pool.update_leases(), 3);
        let l2 = pool.checkout_blocking().unwrap();
        assert_eq!(pool.update_leases(), 2);
        let _l3 = pool.checkout_blocking().unwrap();
        assert_eq!(pool.update_leases(), 1);
        assert_ne!(*l1.get_payload(), *l2.get_payload());
        drop(l1);
        assert_eq!(pool.update_leases(), 2);
        let _l1a = pool.checkout_blocking().unwrap();
        assert_eq!(pool.update_leases(), 1);
        let _l4 = pool.checkout_blocking().unwrap();
        assert_eq!(pool.update_leases(), 0);
        // TODO: Somehow test blocking
    }
}
