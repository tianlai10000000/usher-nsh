//! Allocates process-wide store identities independently of any business module.
use std::sync::Mutex;
use usher_state::StoreId;
const UNAVAILABLE: &str = "state_identity_unavailable";
// IDs belong to the process, not a registry, path or persistent document. Checked
// allocation prevents reuse even when independent registries or sessions are dropped.
static NEXT_STORE_ID: Mutex<u128> = Mutex::new(0);

pub(crate) fn next_store_id() -> Result<StoreId, &'static str> {
    let mut next = NEXT_STORE_ID.lock().map_err(|_| UNAVAILABLE)?;
    *next = next.checked_add(1).ok_or(UNAVAILABLE)?;
    Ok(StoreId::new(*next))
}

#[cfg(test)]
mod tests {
    #[test]
    fn independent_callers_receive_unique_process_identities() {
        let workers: Vec<_> = (0..32)
            .map(|_| std::thread::spawn(super::next_store_id))
            .collect();
        let mut ids: Vec<_> = workers
            .into_iter()
            .map(|worker| worker.join().unwrap().unwrap().value())
            .collect();
        ids.sort_unstable();
        let count = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), count);
    }
}
