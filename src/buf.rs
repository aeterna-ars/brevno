use core::cell::UnsafeCell;
use core::hint;
use core::sync::atomic::{AtomicBool, Ordering};

use buffers::RingBuf;

use super::log::Log;

pub trait LogBuffer {
    type Item;
    fn push(&self, item: Self::Item) -> Result<(), Self::Item>;
    fn pop(&self) -> Option<Self::Item>;
    fn is_full(&self) -> bool;
    fn is_empty(&self) -> bool;
}

struct SpinLock<T> {
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for SpinLock<T> {}
unsafe impl<T: Send> Send for SpinLock<T> {}

impl<T> SpinLock<T> {
    const fn new(value: T) -> Self {
        Self {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(value),
        }
    }

    #[inline(always)]
    fn with_lock<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        while self.lock.compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
            hint::spin_loop();
        }
        let result = f(unsafe { &mut *self.data.get() });
        self.lock.store(false, Ordering::Release);
        result
    }
}

pub struct SpscRingBuf<T, const N: usize>(SpinLock<RingBuf<T, N>>);
pub struct MpscRingBuf<T, const N: usize>(SpinLock<RingBuf<T, N>>);
pub struct MpmcRingBuf<T, const N: usize>(SpinLock<RingBuf<T, N>>);

impl<T, const N: usize> SpscRingBuf<T, N> {
    pub const fn new() -> Self { Self(SpinLock::new(RingBuf::new())) }
    pub fn push(&self, item: T) -> Result<(), T> { self.0.with_lock(|buf| buf.push(item)) }
    pub fn pop(&self) -> Option<T> { self.0.with_lock(|buf| buf.pop()) }
    pub fn len(&self) -> usize { self.0.with_lock(|buf| buf.len()) }
    pub fn is_full(&self) -> bool { self.0.with_lock(|buf| buf.is_full()) }
    pub fn is_empty(&self) -> bool { self.0.with_lock(|buf| buf.is_empty()) }
    pub fn capacity(&self) -> usize { N }
}

impl<T, const N: usize> MpscRingBuf<T, N> {
    pub const fn new() -> Self { Self(SpinLock::new(RingBuf::new())) }
    pub fn push(&self, item: T) -> Result<(), T> { self.0.with_lock(|buf| buf.push(item)) }
    pub fn pop(&self) -> Option<T> { self.0.with_lock(|buf| buf.pop()) }
    pub fn len(&self) -> usize { self.0.with_lock(|buf| buf.len()) }
    pub fn is_full(&self) -> bool { self.0.with_lock(|buf| buf.is_full()) }
    pub fn is_empty(&self) -> bool { self.0.with_lock(|buf| buf.is_empty()) }
    pub fn capacity(&self) -> usize { N }
}

impl<T, const N: usize> MpmcRingBuf<T, N> {
    pub const fn new() -> Self { Self(SpinLock::new(RingBuf::new())) }
    pub fn push(&self, item: T) -> Result<(), T> { self.0.with_lock(|buf| buf.push(item)) }
    pub fn pop(&self) -> Option<T> { self.0.with_lock(|buf| buf.pop()) }
    pub fn len(&self) -> usize { self.0.with_lock(|buf| buf.len()) }
    pub fn is_full(&self) -> bool { self.0.with_lock(|buf| buf.is_full()) }
    pub fn is_empty(&self) -> bool { self.0.with_lock(|buf| buf.is_empty()) }
    pub fn capacity(&self) -> usize { N }
}

impl<const NL: usize, const NB: usize> LogBuffer for SpscRingBuf<Log<NL>, NB> {
    type Item = Log<NL>;
    #[inline(always)] fn push(&self, item: Log<NL>) -> Result<(), Self::Item> { self.push(item) }
    #[inline(always)] fn pop(&self) -> Option<Log<NL>> { self.pop() }
    #[inline(always)] fn is_full(&self) -> bool { self.is_full() }
    #[inline(always)] fn is_empty(&self) -> bool { self.is_empty() }
}

impl<const NL: usize, const NB: usize> LogBuffer for MpscRingBuf<Log<NL>, NB> {
    type Item = Log<NL>;
    #[inline(always)] fn push(&self, item: Log<NL>) -> Result<(), Self::Item> { self.push(item) }
    #[inline(always)] fn pop(&self) -> Option<Log<NL>> { self.pop() }
    #[inline(always)] fn is_full(&self) -> bool { self.is_full() }
    #[inline(always)] fn is_empty(&self) -> bool { self.is_empty() }
}

impl<const NL: usize, const NB: usize> LogBuffer for MpmcRingBuf<Log<NL>, NB> {
    type Item = Log<NL>;
    #[inline(always)] fn push(&self, item: Log<NL>) -> Result<(), Self::Item> { self.push(item) }
    #[inline(always)] fn pop(&self) -> Option<Log<NL>> { self.pop() }
    #[inline(always)] fn is_full(&self) -> bool { self.is_full() }
    #[inline(always)] fn is_empty(&self) -> bool { self.is_empty() }
}

#[cfg(test)]
mod cas_tests {
    use super::*;

    use core::sync::atomic::*;

    extern crate std;

    #[test]
    fn push_pop() {
        let buf: SpscRingBuf<i32, 4> = SpscRingBuf::new();
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
        assert_eq!(buf.pop(), None);

        buf.push(1).unwrap();
        buf.push(2).unwrap();
        buf.push(3).unwrap();
        assert_eq!(buf.len(), 3);
        assert!(!buf.is_full());

        assert_eq!(buf.pop(), Some(1));
        assert_eq!(buf.pop(), Some(2));
        assert_eq!(buf.pop(), Some(3));
        assert_eq!(buf.pop(), None);
        assert!(buf.is_empty());
    }

    #[test]
    fn full() {
        let buf: SpscRingBuf<i32, 2> = SpscRingBuf::new();
        buf.push(1).unwrap();
        buf.push(2).unwrap();
        assert!(buf.is_full());
        assert_eq!(buf.push(3), Err(3));
        buf.pop().unwrap();
        buf.push(3).unwrap();
        assert_eq!(buf.pop(), Some(2));
        assert_eq!(buf.pop(), Some(3));
    }

    #[test]
    fn wrapping() {
        let buf: SpscRingBuf<i32, 4> = SpscRingBuf::new();
        for cycle in 0..100 {
            for i in 0..4 {
                buf.push(cycle * 4 + i).unwrap();
            }
            for i in 0..4 {
                assert_eq!(buf.pop(), Some(cycle * 4 + i));
            }
        }
    }

    #[test]
    fn spsc_two_threads() {
        let buf: &'static SpscRingBuf<i32, 16> = {
            static BUF: SpscRingBuf<i32, 16> = SpscRingBuf::new();
            &BUF
        };

        let producer = std::thread::spawn(move || {
            for i in 0..1000 {
                loop {
                    if buf.push(i).is_ok() {
                        break;
                    }
                    std::thread::yield_now();
                }
            }
        });

        let consumer = std::thread::spawn(move || {
            let mut sum = 0;
            for _ in 0..1000 {
                loop {
                    if let Some(val) = buf.pop() {
                        sum += val;
                        break;
                    }
                    std::thread::yield_now();
                }
            }
            sum
        });

        producer.join().unwrap();
        assert_eq!(consumer.join().unwrap(), 999 * 1000 / 2);
    }

    #[test]
    fn mpsc_four_producers() {
        static BUF: MpscRingBuf<i32, 32> = MpscRingBuf::new();

        let mut handles = std::vec![];
        
        for t in 0..4 {
            handles.push(std::thread::spawn(move || {
                for i in 0..100 {
                    loop {
                        let item = t * 100 + i;
                        match BUF.push(item) {
                            Ok(()) => break,
                            Err(_) => std::thread::yield_now(),
                        }
                    }
                }
            }));
        }

        let consumer = std::thread::spawn(move || {
            let mut received = std::vec![0usize; 400];
            let mut collected = 0;
            
            while collected < 400 {
                match BUF.pop() {
                    Some(val) => {
                        received[val as usize] += 1;
                        collected += 1;
                    }
                    None => std::thread::yield_now(),
                }
            }
            received
        });

        for h in handles {
            h.join().unwrap();
        }
        
        let received = consumer.join().unwrap();
        
        assert_eq!(received.iter().sum::<usize>(), 400);
        for (i, &count) in received.iter().enumerate() {
            assert_eq!(count, 1, "Value {} received {} times", i, count);
        }
    }

    #[test]
    fn mpmc_four_producers_four_consumers() {
        let buf: &'static MpmcRingBuf<i32, 32> = {
            static BUF: MpmcRingBuf<i32, 32> = MpmcRingBuf::new();
            &BUF
        };

        static PRODUCED: AtomicUsize = AtomicUsize::new(0);
        static CONSUMED: AtomicUsize = AtomicUsize::new(0);

        let mut handles = std::vec![];

        for t in 0..4 {
            handles.push(std::thread::spawn(move || {
                for i in 0..100 {
                    loop {
                        if buf.push(t * 100 + i).is_ok() {
                            PRODUCED.fetch_add(1, Ordering::Relaxed);
                            break;
                        }
                        std::thread::yield_now();
                    }
                }
            }));
        }

        for _ in 0..4 {
            handles.push(std::thread::spawn(move || loop {
                if CONSUMED.load(Ordering::Relaxed) >= 400 {
                    break;
                }
                if buf.pop().is_some() {
                    CONSUMED.fetch_add(1, Ordering::Relaxed);
                } else {
                    std::thread::yield_now();
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(PRODUCED.load(Ordering::Relaxed), 400);
        assert_eq!(CONSUMED.load(Ordering::Relaxed), 400);
    }

    #[test]
    fn drop_counter() {
        use core::sync::atomic::{AtomicUsize, Ordering};

        static DROPS: AtomicUsize = AtomicUsize::new(0);

        #[derive(Debug)]
        struct Item;
        impl Drop for Item {
            fn drop(&mut self) {
                DROPS.fetch_add(1, Ordering::Relaxed);
            }
        }

        {
            let buf: SpscRingBuf<Item, 4> = SpscRingBuf::new();
            buf.push(Item).unwrap();
            buf.push(Item).unwrap();
            let item = buf.pop().unwrap();
            drop(item);
            assert_eq!(DROPS.load(Ordering::Relaxed), 1);
        }
        assert_eq!(DROPS.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn zst() {
        let buf: SpscRingBuf<(), 4> = SpscRingBuf::new();
        buf.push(()).unwrap();
        buf.push(()).unwrap();
        assert_eq!(buf.len(), 2);
        assert_eq!(buf.pop(), Some(()));
        assert_eq!(buf.pop(), Some(()));
        assert_eq!(buf.pop(), None);
    }

    #[test]
    fn capacity() {
        assert_eq!(SpscRingBuf::<i32, 16>::new().capacity(), 16);
        assert_eq!(MpscRingBuf::<u64, 256>::new().capacity(), 256);
        assert_eq!(MpmcRingBuf::<u8, 8>::new().capacity(), 8);
    }
}