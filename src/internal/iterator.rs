//! Iterators for matches in the RPM database

use librpm_sys;
#[cfg(feature = "regex")]
use regex::Regex;
use std::{ptr, os::raw::c_void};
use streaming_iterator::StreamingIterator;

use internal::{header::Header, tag::Tag, ts::GlobalTS};

/// Iterator over the matches from a database query
pub(crate) struct MatchIterator {
    /// Pointer to librpm's match iterator.
    ptr: *mut librpm_sys::rpmdbMatchIterator_s,

    /// Hold the lock on the global transaction set while reading data.
    /// This ensures nothing else can make calls to librpm while we are iterating over its data
    #[allow(dead_code)]
    txn: GlobalTS,

    /// Next item in the iterator
    next_item: Option<Header>,
}

impl MatchIterator {
    /// Create a new `MatchIterator` for the current RPM database, searching
    /// by the (optionally) given search key.
    pub(crate) fn new(tag: Tag, key_opt: Option<&str>) -> Self {
        let mut txn = GlobalTS::create();
        let next_item = None;

        if let Some(key) = key_opt {
            if !key.is_empty() {
                let ptr = unsafe {
                    librpm_sys::rpmtsInitIterator(
                        txn.as_mut_ptr(),
                        tag as librpm_sys::rpm_tag_t,
                        key.as_ptr() as *const c_void,
                        key.len(),
                    )
                };

                return Self {
                    ptr,
                    txn,
                    next_item,
                };
            }
        }

        let ptr = unsafe {
            librpm_sys::rpmtsInitIterator(
                txn.as_mut_ptr(),
                tag as librpm_sys::rpm_tag_t,
                ptr::null(),
                0,
            )
        };

        Self {
            ptr,
            txn,
            next_item,
        }
    }
}

/// Use a StreamingIterator to ensure that headers do not outlive `rpmdbNextIterator` calls.
impl StreamingIterator for MatchIterator {
    type Item = Header;

    fn advance(&mut self) {
        let header_ptr = unsafe { librpm_sys::rpmdbNextIterator(self.ptr) };

        if header_ptr.is_null() {
            self.next_item = None
        } else {
            self.next_item = Some(Header::new(header_ptr))
        }
    }

    fn get(&self) -> Option<&Header> {
        self.next_item.as_ref()
    }
}

impl Drop for MatchIterator {
    fn drop(&mut self) {
        unsafe {
            librpm_sys::rpmdbFreeIterator(self.ptr);
        }
    }
}
