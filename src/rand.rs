use std::{debug_assert, mem, sync::Mutex};

use rand_core::{RngCore, SeedableRng};
use rand_hc::Hc128Rng;

pub struct StringGenerator {
    rng: Mutex<Hc128Rng>,
}

impl StringGenerator {
    pub fn new() -> Self {
        #[rustfmt::skip]
        let rng = Hc128Rng::from_seed([
            1,   255, 32,  6,   78,  90,  11,  54,
            28,  100, 64,  237, 58,  91,  121, 169,
            3,   96,  128, 8,   184, 219, 61,  55,
            204, 87,  130, 69,  25,  40,  72,  240,
        ]);

        Self {
            rng: Mutex::new(rng),
        }
    }

    pub fn generate_random_url_segment(&self) -> String {
        #[rustfmt::skip]
        static HEX_NIBBLES: [u8; 16] = [
            b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7',
            b'8', b'9', b'a', b'b', b'c', b'd', b'e', b'f',
        ];

        const BYTE_COUNT: usize = 8;
        const ASCII_COUNT: usize = BYTE_COUNT * 2;

        let mut bytes = [0u8; BYTE_COUNT];
        let mut ascii = Box::new([0u8; ASCII_COUNT]);

        self.rng.lock().unwrap().fill_bytes(&mut bytes);

        for (i, byte) in bytes.iter().copied().enumerate() {
            let hi = (byte >> 4) & 0xF;
            let lo = (byte >> 0) & 0xF;

            ascii[i * 2 + 0] = HEX_NIBBLES[hi as usize];
            ascii[i * 2 + 1] = HEX_NIBBLES[lo as usize];
        }

        let ascii = boxed_array_to_vec(ascii);

        // Safety:
        // We know that the ascii slice contains valid utf8 since it was initialised with
        // zeros and all the values were set to one of the elements of HEX_NIBBLES.
        unsafe { String::from_utf8_unchecked(ascii) }
    }
}

fn boxed_array_to_vec<T, const N: usize>(mut array: Box<[T; N]>) -> Vec<T> {
    // My reasoning for assuming this is safe is as follows:
    //
    // A Box<[T; N]> has the following layout in memory:
    // Box { ptr }
    //       |
    //       V
    //      [0, 1, 2, 3, 4, ...]
    //
    // And a Vec<T> has the following layout in memory:
    // Vec { ptr, len, cap }
    //       |
    //       V
    //      [0, 1, 2, 3, 4, ...]
    //
    // So the ptr that the box has is the pointer to the first element of the array.
    // Which is what a Vec wants.

    let slice_ptr = array.as_mut_ptr();

    // Make sure to forget the Box since we don't want it deallocating the
    // memory when we return.
    mem::forget(array);

    debug_assert!(N < isize::MAX as usize);

    // Safety:
    // 1. We know the Box was allocated using the global allocator since it doesn't have a generic allocator argument.
    // 2. T is unchanged and therefore has the same alignment.
    // 3. The capacity is set to N which is the same size we used to allocate the data.
    // 4. The length is set to N which is equal to the capacity.
    // 5. All bytes are initialised unless the array in the Box was created with unsafe.
    unsafe { Vec::from_raw_parts(slice_ptr, N, N) }
}
