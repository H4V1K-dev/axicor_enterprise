use std::sync::atomic::{AtomicUsize, Ordering};
use std::cell::UnsafeCell;
use crate::error::TransportError;

/// Individual container cell inside the RingBuffer.
pub struct Cell<T> {
    /// Holds the slot item value.
    pub value: UnsafeCell<Option<T>>,
    /// Tracks the monotonic sequence index of the cell.
    pub sequence: AtomicUsize,
}

unsafe impl<T: Send> Send for Cell<T> {}
unsafe impl<T: Send> Sync for Cell<T> {}

/// Thread-safe, lock-free RingBuffer.
pub struct RingBuffer<T> {
    /// Flat buffer containing standard cells.
    pub buffer: Vec<Cell<T>>,
    /// Read cursor index.
    pub head: AtomicUsize,
    /// Write cursor index.
    pub tail: AtomicUsize,
    /// Bitmask for wrapping index ranges.
    pub mask: usize,
}

unsafe impl<T: Send> Send for RingBuffer<T> {}
unsafe impl<T: Send> Sync for RingBuffer<T> {}

impl<T> RingBuffer<T> {
    /// Create a new RingBuffer of the given capacity.
    ///
    /// # Invariant assertions (INV-TRANS-001)
    /// Under INV-TRANS-001, the capacity must strictly be a power of two.
    /// This is an hardware/architectural optimization which allows the queue logic
    /// to wrap indexes using `index & (capacity - 1)` bitmask math inside 1 CPU cycle
    /// instead of using the expensive integer modulo `%` division, which is forbidden.
    ///
    /// # Errors
    /// Returns `TransportError::InvalidCapacity` if `capacity` is not a power of two.
    pub fn new(capacity: usize) -> Result<Self, TransportError> {
        if !capacity.is_power_of_two() {
            return Err(TransportError::InvalidCapacity);
        }

        let mut buffer = Vec::with_capacity(capacity);
        for i in 0..capacity {
            buffer.push(Cell {
                value: UnsafeCell::new(None),
                sequence: AtomicUsize::new(i),
            });
        }

        Ok(Self {
            buffer,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            mask: capacity - 1,
        })
    }

    /// Pushes an item into the RingBuffer in a thread-safe, lock-free manner.
    ///
    /// # Errors
    /// Returns `TransportError::QueueFull` if the buffer is full.
    pub fn push(&self, item: T) -> Result<(), TransportError> {
        let mut pos = self.tail.load(Ordering::Relaxed);
        loop {
            let cell = &self.buffer[pos & self.mask];
            let seq = cell.sequence.load(Ordering::Acquire);
            let diff = seq as isize - pos as isize;

            if diff == 0 {
                match self.tail.compare_exchange_weak(
                    pos,
                    pos + 1,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        unsafe {
                            *cell.value.get() = Some(item);
                        }
                        cell.sequence.store(pos + 1, Ordering::Release);
                        return Ok(());
                    }
                    Err(actual) => pos = actual,
                }
            } else if diff < 0 {
                return Err(TransportError::QueueFull);
            } else {
                pos = self.tail.load(Ordering::Relaxed);
            }
        }
    }

    /// Pops an item from the RingBuffer in a thread-safe, lock-free manner.
    pub fn pop(&self) -> Option<T> {
        let mut pos = self.head.load(Ordering::Relaxed);
        loop {
            let cell = &self.buffer[pos & self.mask];
            let seq = cell.sequence.load(Ordering::Acquire);
            let diff = seq as isize - (pos + 1) as isize;

            if diff == 0 {
                match self.head.compare_exchange_weak(
                    pos,
                    pos + 1,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        let item = unsafe {
                            (*cell.value.get()).take()
                        };
                        cell.sequence.store(pos + self.mask + 1, Ordering::Release);
                        return item;
                    }
                    Err(actual) => pos = actual,
                }
            } else if diff < 0 {
                return None;
            } else {
                pos = self.head.load(Ordering::Relaxed);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_power_of_two_init() {
        // E-117: Init capacity must be power of two
        let rb = RingBuffer::<i32>::new(1024);
        assert!(rb.is_ok());

        let rb_bad = RingBuffer::<i32>::new(1000);
        assert_eq!(rb_bad.err(), Some(TransportError::InvalidCapacity));
    }

    #[test]
    fn test_ring_buffer_overflow_protection() {
        // E-118: Pushing into a filled queue returns QueueFull
        let rb = RingBuffer::<i32>::new(2).unwrap();
        assert!(rb.push(10).is_ok());
        assert!(rb.push(20).is_ok());

        let res = rb.push(30);
        assert_eq!(res.err(), Some(TransportError::QueueFull));

        // Ensure state is not corrupted, popping should still yield correct items
        assert_eq!(rb.pop(), Some(10));
        assert_eq!(rb.pop(), Some(20));
        assert_eq!(rb.pop(), None);
    }

    #[test]
    fn test_ring_buffer_empty_pop() {
        // E-119: Popping empty queue returns None
        let rb = RingBuffer::<i32>::new(4).unwrap();
        assert_eq!(rb.pop(), None);
    }
}
