//! Minimal reader for `NumPy` `.npy` v1.0 files: float32, 2-D, C-contiguous only.
//!
//! Deliberately hand-rolled rather than pulling in a general-purpose npy crate;
//! the subset of the format this project needs is about 60 lines.

use crate::IndexError;

/// A parsed float32 2-D array from an `.npy` file.
pub struct NpyArray {
    pub shape: (usize, usize),
    pub data: Vec<f32>,
}

const MAGIC: &[u8] = b"\x93NUMPY";

pub fn parse_npy_f32_2d(bytes: &[u8]) -> Result<NpyArray, IndexError> {
    if bytes.len() < 10 || &bytes[0..6] != MAGIC {
        return Err(IndexError::Corrupt("bad npy magic".into()));
    }
    let major = bytes[6];
    let (header_len, header_start) = if major == 1 {
        (u16::from_le_bytes([bytes[8], bytes[9]]) as usize, 10)
    } else {
        if bytes.len() < 12 {
            return Err(IndexError::Corrupt("truncated npy header".into()));
        }
        (
            u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize,
            12,
        )
    };
    let header_end = header_start + header_len;
    if bytes.len() < header_end {
        return Err(IndexError::Corrupt("truncated npy header".into()));
    }
    let header = std::str::from_utf8(&bytes[header_start..header_end])
        .map_err(|_| IndexError::Corrupt("npy header is not valid utf8".into()))?;

    if !header.contains("'descr': '<f4'") {
        return Err(IndexError::Corrupt(format!(
            "unsupported npy dtype, expected '<f4': {header}"
        )));
    }
    if !header.contains("'fortran_order': False") {
        return Err(IndexError::Corrupt(
            "npy array is fortran-ordered, expected C-contiguous".into(),
        ));
    }

    let shape = parse_shape(header)?;
    let &[n, dim] = shape.as_slice() else {
        return Err(IndexError::Corrupt(format!(
            "expected a 2-D array, got shape {shape:?}"
        )));
    };

    let data_bytes = &bytes[header_end..];
    let expected_len = n * dim * 4;
    if data_bytes.len() != expected_len {
        return Err(IndexError::Corrupt(format!(
            "npy data is {} bytes, expected {expected_len} for shape ({n}, {dim})",
            data_bytes.len()
        )));
    }
    let data = data_bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();

    Ok(NpyArray {
        shape: (n, dim),
        data,
    })
}

fn parse_shape(header: &str) -> Result<Vec<usize>, IndexError> {
    let key = "'shape':";
    let after_key = header
        .find(key)
        .map(|i| &header[i + key.len()..])
        .ok_or_else(|| IndexError::Corrupt("npy header missing 'shape'".into()))?;
    let open = after_key
        .find('(')
        .ok_or_else(|| IndexError::Corrupt("malformed npy shape tuple".into()))?;
    let close = after_key
        .find(')')
        .ok_or_else(|| IndexError::Corrupt("malformed npy shape tuple".into()))?;
    after_key[open + 1..close]
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.parse::<usize>()
                .map_err(|_| IndexError::Corrupt(format!("bad npy shape entry: {s}")))
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::cast_possible_truncation)] // hand-built byte-literal fixtures, sizes are tiny
mod tests {
    use super::*;

    // A byte-literal npy v1.0 header for a (2, 3) float32 C-contiguous array,
    // built by hand per the .npy format spec: magic, version, 2-byte header
    // length (little-endian), then the ASCII header dict padded to a 64-byte
    // boundary with spaces and a trailing newline.
    fn header_bytes(descr: &str, fortran_order: &str, shape: &str) -> Vec<u8> {
        let dict =
            format!("{{'descr': '{descr}', 'fortran_order': {fortran_order}, 'shape': {shape}, }}");
        let prefix_len = 6 + 2 + 2; // magic + version + header-length field
        let unpadded_len = dict.len() + 1; // + newline
        let total_before_pad = prefix_len + unpadded_len;
        let pad = (64 - total_before_pad % 64) % 64;
        let mut dict_padded = dict.into_bytes();
        dict_padded.extend(std::iter::repeat_n(b' ', pad));
        dict_padded.push(b'\n');

        let mut out = Vec::new();
        out.extend_from_slice(b"\x93NUMPY");
        out.push(1); // major version
        out.push(0); // minor version
        out.extend_from_slice(&(dict_padded.len() as u16).to_le_bytes());
        out.extend_from_slice(&dict_padded);
        out
    }

    fn valid_npy_bytes() -> Vec<u8> {
        let mut bytes = header_bytes("<f4", "False", "(2, 3)");
        // seq A (row 0): [1.0, 2.0, 3.0]; seq B (row 1): [4.0, 5.0, 6.0]
        for v in [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        bytes
    }

    #[test]
    fn parses_shape_and_exact_values() {
        let bytes = valid_npy_bytes();
        let arr = parse_npy_f32_2d(&bytes).expect("valid npy should parse");
        assert_eq!(arr.shape, (2, 3));
        assert_eq!(arr.data, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn rejects_bad_magic() {
        let mut bytes = valid_npy_bytes();
        bytes[0] = 0x00;
        assert!(parse_npy_f32_2d(&bytes).is_err());
    }

    #[test]
    fn rejects_non_float32_dtype() {
        let mut bytes = header_bytes("<f8", "False", "(2, 3)");
        for v in [1.0f64, 2.0, 3.0, 4.0, 5.0, 6.0] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        assert!(parse_npy_f32_2d(&bytes).is_err());
    }

    #[test]
    fn rejects_fortran_order() {
        let mut bytes = header_bytes("<f4", "True", "(2, 3)");
        for v in [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        assert!(parse_npy_f32_2d(&bytes).is_err());
    }

    #[test]
    fn rejects_data_length_mismatch() {
        let mut bytes = header_bytes("<f4", "False", "(2, 3)");
        // Only 5 values instead of the 6 the shape declares.
        for v in [1.0f32, 2.0, 3.0, 4.0, 5.0] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        assert!(parse_npy_f32_2d(&bytes).is_err());
    }

    #[test]
    fn rejects_non_2d_shape() {
        let mut bytes = header_bytes("<f4", "False", "(6,)");
        for v in [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        assert!(parse_npy_f32_2d(&bytes).is_err());
    }
}
