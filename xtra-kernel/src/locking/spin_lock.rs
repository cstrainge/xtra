
/// Implementation of a simple spinlock for the xtra kernel.

use core::{ hint::spin_loop, sync::atomic::{ AtomicBool, Ordering } };

use crate::locking::Locking;



/// A simple spinlock implementation that uses an atomic boolean to indicate if the lock is held.
/// This is a basic implementation that can be used to protect shared resources in a multi-threaded
/// environment.
///
/// This is a very simple lock and is not reentrant, meaning that if a thread tries to acquire the
/// lock while it already holds it, it will deadlock.
pub struct SpinLock
{
    locked: AtomicBool  // Indicates if the lock is currently held.
}



impl SpinLock
{
    /// Create a new unlocked spinlock. Meaning that the lock is not held by any thread.
    pub const fn new() -> Self
    {
        SpinLock { locked: AtomicBool::new(false) }
    }
}



/// The locking trait is shared by all locking mechanisms in the kernel.
impl Locking for SpinLock
{
    fn lock(&self)
    {
        // Keep looping until we can acquire the lock.
        while self.locked.swap(true, Ordering::Acquire)
        {
            // While we're waiting we can tell the CPU to lower it's power consumption by telling it
            // we're in a spin loop.
            spin_loop();
        }
    }

    fn unlock(&self)
    {
        // Release the lock by setting the atomic boolean to false.
        self.locked.store(false, Ordering::Release);
    }
}
