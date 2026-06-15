/// Waiting strategy profile for thread suspension in BSP barriers and event loops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitStrategy {
    /// `std::hint::spin_loop()` (~1 ns latency, 100% CPU usage).
    Aggressive,
    /// `std::thread::yield_now()` (~1–15 us latency, shares CPU with OS).
    Balanced,
    /// `std::thread::sleep(Duration::from_millis(1))` (~1–5 ms latency, ~0% CPU usage).
    Eco,
}

impl WaitStrategy {
    /// Suspends the current thread according to the selected waiting strategy profile.
    pub fn wait(&self) {
        match self {
            Self::Aggressive => std::hint::spin_loop(),
            Self::Balanced => std::thread::yield_now(),
            Self::Eco => std::thread::sleep(std::time::Duration::from_millis(1)),
        }
    }
}
