//! Parameter arrays for the `*_with_params_n` family.
//!
//! Every parameterised entry point in `chdb.h` takes four parallel arrays —
//! name pointers, name lengths, value pointers, value lengths — plus a count.
//! Building them is the same in all five places, so it lives here.

use std::os::raw::c_char;

/// Borrowed views of a caller's parameter slice, laid out for the C ABI.
///
/// Holds no ownership: the pointers reference the caller's `&str` data, which
/// outlives the FFI call because the slice is borrowed for the whole call.
#[allow(dead_code)]
pub(crate) struct ParamArrays<'a> {
    names: Vec<*const c_char>,
    name_lens: Vec<usize>,
    values: Vec<*const c_char>,
    value_lens: Vec<usize>,
    _params: std::marker::PhantomData<&'a [(&'a str, &'a str)]>,
}

#[allow(dead_code)]
impl<'a> ParamArrays<'a> {
    pub(crate) fn new(params: &'a [(&'a str, &'a str)]) -> Self {
        Self {
            names: params
                .iter()
                .map(|(n, _)| n.as_ptr() as *const c_char)
                .collect(),
            name_lens: params.iter().map(|(n, _)| n.len()).collect(),
            values: params
                .iter()
                .map(|(_, v)| v.as_ptr() as *const c_char)
                .collect(),
            value_lens: params.iter().map(|(_, v)| v.len()).collect(),
            _params: std::marker::PhantomData,
        }
    }

    pub(crate) fn count(&self) -> usize {
        self.names.len()
    }

    /// Null when there are no parameters. `Vec::as_ptr` on an empty vector is
    /// dangling-but-aligned, which is fine for Rust and not something to hand
    /// to C alongside a zero count.
    fn ptr<T>(slice: &[T]) -> *const T {
        if slice.is_empty() {
            std::ptr::null()
        } else {
            slice.as_ptr()
        }
    }

    pub(crate) fn names(&self) -> *const *const c_char {
        Self::ptr(&self.names)
    }

    pub(crate) fn name_lens(&self) -> *const usize {
        Self::ptr(&self.name_lens)
    }

    pub(crate) fn values(&self) -> *const *const c_char {
        Self::ptr(&self.values)
    }

    pub(crate) fn value_lens(&self) -> *const usize {
        Self::ptr(&self.value_lens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_slice_yields_null_pointers_and_a_zero_count() {
        let params = ParamArrays::new(&[]);
        assert_eq!(params.count(), 0);
        assert!(params.names().is_null());
        assert!(params.name_lens().is_null());
        assert!(params.values().is_null());
        assert!(params.value_lens().is_null());
    }

    #[test]
    fn lengths_are_byte_lengths_not_character_counts() {
        let input = [("naïve", "日本")];
        let params = ParamArrays::new(&input);

        assert_eq!(params.count(), 1);
        // "naïve" is 6 bytes, "日本" is 6 bytes.
        let name_lens = unsafe { std::slice::from_raw_parts(params.name_lens(), 1) };
        let value_lens = unsafe { std::slice::from_raw_parts(params.value_lens(), 1) };
        assert_eq!(name_lens[0], 6);
        assert_eq!(value_lens[0], 6);
    }

    #[test]
    fn pointers_reference_the_callers_data() {
        let input = [("x", "42")];
        let params = ParamArrays::new(&input);

        let names = unsafe { std::slice::from_raw_parts(params.names(), 1) };
        assert_eq!(names[0], input[0].0.as_ptr() as *const c_char);
    }

    #[test]
    fn a_value_may_contain_an_interior_nul() {
        let input = [("blob", "a\0b")];
        let params = ParamArrays::new(&input);

        let value_lens = unsafe { std::slice::from_raw_parts(params.value_lens(), 1) };
        assert_eq!(value_lens[0], 3, "the NUL is data, so the length is 3");
    }
}
