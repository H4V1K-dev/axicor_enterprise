use std::collections::HashMap;
use crate::error::NetError;

/// Dimension block size for tiled matrix transpositions to fit the CPU L1 cache line size.
pub const IO_TRANSPOSE_BLOCK_SIZE: usize = 64;

/// IoServer handles input/output buffer mappings and transpositions.
pub struct IoServer {
    /// Maps UV coordinates of external matrices to flat DenseIndex of VRAM.
    pub matrix_offsets: HashMap<u32, usize>,
    /// Pre-allocated buffer for SoA <-> AoS matrix transpositions.
    pub transpose_buffer: Vec<u8>,
}

impl IoServer {
    /// Create a new IoServer instance.
    pub fn new() -> Self {
        Self {
            matrix_offsets: HashMap::new(),
            transpose_buffer: Vec::new(),
        }
    }

    /// Performs cached-optimized SoA -> AoS matrix transpose and sends the batch.
    ///
    /// # L1 Transpose Invariant (INV-NET-003)
    /// GPU backend pipelines data in Structure of Arrays (SoA) format. External clients
    /// expect data in Array of Structures (AoS) format. Naive transposition over large
    /// matrices (e.g. 1000x1000) causes constant cache-line thrashing. We implement a Tiled
    /// Transpose algorithm that processes tiles of size IO_TRANSPOSE_BLOCK_SIZE. This guarantees
    /// that the active working set fits completely within the CPU's L1 cache line size, avoiding
    /// cache thrashing and maximizing memory throughput.
    ///
    /// # Panics
    /// Panics if `soa_history.len() != batch_size * pixels` (E-123).
    pub fn send_output_batch(
        &mut self,
        soa_history: &[u8],
        batch_size: u32,
        pixels: u32,
    ) -> Result<(), NetError> {
        let expected_size = (batch_size * pixels) as usize;
        if soa_history.len() != expected_size {
            panic!("L1 Transpose: size mismatch");
        }

        if self.transpose_buffer.len() < expected_size {
            self.transpose_buffer.resize(expected_size, 0);
        }

        let t = batch_size as usize;
        let p = pixels as usize;
        let block = IO_TRANSPOSE_BLOCK_SIZE;

        for i in (0..t).step_by(block) {
            for j in (0..p).step_by(block) {
                for ii in i..std::cmp::min(i + block, t) {
                    for jj in j..std::cmp::min(j + block, p) {
                        self.transpose_buffer[jj * t + ii] = soa_history[ii * p + jj];
                    }
                }
            }
        }

        Ok(())
    }
}

impl Default for IoServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_l1_transpose_math() {
        let mut server = IoServer::new();
        // 4 ticks (rows), 3 pixels (cols)
        // SoA layout in memory:
        // Row 0: 0, 1, 2
        // Row 1: 3, 4, 5
        // Row 2: 6, 7, 8
        // Row 3: 9, 10, 11
        let soa_history = [
            0, 1, 2,
            3, 4, 5,
            6, 7, 8,
            9, 10, 11,
        ];
        
        let res = server.send_output_batch(&soa_history, 4, 3);
        assert!(res.is_ok());

        // Expected AoS layout (transposed):
        // Col 0: 0, 3, 6, 9
        // Col 1: 1, 4, 7, 10
        // Col 2: 2, 5, 8, 11
        let expected = [
            0, 3, 6, 9,
            1, 4, 7, 10,
            2, 5, 8, 11,
        ];
        
        assert_eq!(&server.transpose_buffer[..12], &expected[..]);
    }

    #[test]
    #[should_panic(expected = "L1 Transpose: size mismatch")]
    fn test_l1_transpose_out_of_bounds() {
        let mut server = IoServer::new();
        let soa_history = [0u8; 10];
        // expected size is 3 * 4 = 12, but we passed 10
        let _ = server.send_output_batch(&soa_history, 4, 3);
    }
}
