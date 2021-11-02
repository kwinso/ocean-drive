use std::sync::{Arc, Mutex, MutexGuard};

pub fn lock_ref_when_free<T>(r: &Arc<Mutex<T>>) -> MutexGuard<T> {
    loop {
        if let Ok(v) = r.try_lock() {
            return v;
        }

        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
