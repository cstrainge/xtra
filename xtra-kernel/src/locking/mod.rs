
/// The locking trait is shared by all locking mechanisms in the kernel.
pub trait Locking
{
    /// Acquire the lock. This method will block until the lock is acquired.
    fn lock(&self);

    /// Release the lock. This method should be called after the critical section is done.
    fn unlock(&self);
}



// Simple auto-locking and unlocking mechanism that can be used to ensure that the lock is released
// when the guard goes out of scope. This is useful for ensuring that locks are always released
// even if an error occurs in the critical section.
pub struct LockGuard<'a, T: Locking>
{
    lock: &'a T  // The lock that is being held.
}



impl<'a, T: Locking> LockGuard<'a, T>
{
    /// Create a new lock guard that acquires the lock and will automatically release it when it
    /// goes out of scope.
    pub fn new(lock: &'a T) -> Self
    {
        lock.lock();  // Acquire the lock.
        LockGuard { lock }
    }
}



impl<'a, T: Locking> Drop for LockGuard<'a, T>
{
    /// Release the lock when the guard goes out of scope.
    fn drop(&mut self)
    {
        self.lock.unlock();  // Release the lock.
    }
}



/// The spinlock module provides a simple spinlock implementation for the xtra kernel.
pub mod spin_lock;
