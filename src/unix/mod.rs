// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

/// The underlying OsString/OsStr implementation on Unix systems: just
/// a `Vec<u8>`/`[u8]`.

use slice_searcher::SliceSearcher;
use split_bytes;
use utf8_sections::Utf8Sections;

use std::borrow::Cow;
use std::fmt::{self, Debug};
use std::vec::Vec;
use std::str;
use core::str::pattern::Pattern;
use std::string::String;
use std::mem;

#[derive(Clone, Hash)]
pub struct Buf {
    pub inner: Vec<u8>
}

pub struct Slice {
    pub inner: [u8]
}

impl Debug for Slice {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.to_string_lossy().fmt(formatter)
    }
}

impl Debug for Buf {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.as_slice().fmt(formatter)
    }
}

impl Buf {
    pub fn from_string(s: String) -> Buf {
        Buf { inner: s.into_bytes() }
    }

    pub fn as_slice(&self) -> &Slice {
        unsafe { mem::transmute(&*self.inner) }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Buf { inner: Vec::with_capacity(capacity) }
    }

    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    pub fn reserve(&mut self, additional: usize) {
        self.inner.reserve(additional)
    }

    fn reserve_exact(&mut self, additional: usize) {
        self.inner.reserve_exact(additional)
    }

    pub fn into_string(self) -> Result<String, Buf> {
        String::from_utf8(self.inner).map_err(|p| Buf { inner: p.into_bytes() } )
    }

    pub fn into_string_lossy(self) -> String {
        self.into_string().unwrap_or_else(|buf| buf.as_slice().to_string_lossy().into_owned())
    }

    pub fn push_slice(&mut self, s: &Slice) {
        self.inner.push_all(&s.inner)
    }

    pub fn clear(&mut self) {
        self.inner.clear()
    }
}

impl Slice {
    fn from_u8_slice(s: &[u8]) -> &Slice {
        unsafe { mem::transmute(s) }
    }

    pub fn from_str(s: &str) -> &Slice {
        Slice::from_u8_slice(s.as_bytes())
    }

    pub fn to_str(&self) -> Option<&str> {
        str::from_utf8(&self.inner).ok()
    }

    pub fn to_string_lossy(&self) -> Cow<str> {
        String::from_utf8_lossy(&self.inner)
    }

    pub fn to_owned(&self) -> Buf {
        Buf { inner: self.inner.to_vec() }
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn contains_os(&self, needle: &Slice) -> bool {
        SliceSearcher::new(&self.inner, &needle.inner).next().is_some()
    }

    pub fn starts_with_os(&self, needle: &Slice) -> bool {
        self.inner.starts_with(&needle.inner)
    }

    pub fn ends_with_os(&self, needle: &Slice) -> bool {
        self.inner.ends_with(&needle.inner)
    }

    pub fn utf8_sections<'a>(&'a self) -> Utf8Sections<'a> {
        Utf8Sections::new(&self.inner)
    }

    pub fn split<'a, P>(&'a self, pat: P) -> Split<'a, P> where P: Pattern<'a> + Clone {
        Split { inner: split_bytes::Split::new(&self.inner, pat) }
    }

    pub fn starts_with_str(&self, prefix: &str) -> bool {
        self.inner.starts_with(prefix.as_bytes())
    }

    pub fn remove_prefix_str(&self, prefix: &str) -> Option<&Slice> {
        if self.inner.starts_with(prefix.as_bytes()) {
            Some(Self::from_u8_slice(&self.inner[prefix.len()..]))
        } else {
            None
        }
    }

    pub fn slice_shift_char(&self) -> Option<(char, &Slice)> {
        let utf8_prefix = match str::from_utf8(&self.inner) {
            Ok(s) => s,
            Err(e) => str::from_utf8(&self.inner[0..e.valid_up_to()]).unwrap()
        };
        utf8_prefix.chars().next()
            .map(|first|
                 (first, Self::from_u8_slice(&self.inner[first.len_utf8()..])))
    }

    pub fn split_off_str(&self, boundary: char) -> Option<(&str, &Slice)> {
        let utf8_prefix = match str::from_utf8(&self.inner) {
            Ok(s) => s,
            Err(e) => str::from_utf8(&self.inner[0..e.valid_up_to()]).unwrap()
        };
        utf8_prefix.find(boundary)
            .map(|b| (&utf8_prefix[0..b],
                      Self::from_u8_slice(&self.inner[b + boundary.len_utf8()..])))
    }
}

pub struct Split<'a, P> where P: Pattern<'a> {
    inner: split_bytes::Split<'a, P>,
}

impl<'a, P> Clone for Split<'a, P> where P: Pattern<'a> + Clone, P::Searcher: Clone {
    fn clone(&self) -> Self { Split { inner: self.inner.clone() } }
}

impl<'a, P> Iterator for Split<'a, P> where P: Pattern<'a> + Clone {
    type Item = &'a Slice;

    fn next(&mut self) -> Option<&'a Slice> {
        self.inner.next().map(Slice::from_u8_slice)
    }
}


pub mod os_str {
    use super::{Buf, Slice, Split as ImplSplit};

    macro_rules! is_windows { () => { false } }
    macro_rules! if_unix_windows { (unix $u:block windows $w:block) => { $u } }

    include!("../os_str_def.rs");
}
pub use self::os_str::{OsStr, OsString};

pub mod os_str_ext;
pub use self::os_str_ext::{OsStrExt, OsStringExt};
