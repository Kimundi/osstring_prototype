// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


// This file isn't compiled normally, but is included into the two
// OS-specific os_str modules.


// Inner comments aren't valid in an include.
// //! A type that can represent all platform-native strings, but is cheaply
// //! interconvertable with Rust strings.
// //!
// //! The need for this type arises from the fact that:
// //!
// //! * On Unix systems, strings are often arbitrary sequences of non-zero
// //!   bytes, in many cases interpreted as UTF-8.
// //!
// //! * On Windows, strings are often arbitrary sequences of non-zero 16-bit
// //!   values, interpreted as UTF-16 when it is valid to do so.
// //!
// //! * In Rust, strings are always valid UTF-8, but may contain zeros.
// //!
// //! The types in this module bridge this gap by simultaneously representing Rust
// //! and platform-native string values, and in particular allowing a Rust string
// //! to be converted into an "OS" string with no cost.
// //!
// //! **Note**: At the moment, these types are extremely bare-bones, usable only
// //! for conversion to/from various other string types. Eventually these types
// //! will offer a full-fledged string API.

use std::borrow::{Borrow, Cow, ToOwned};
use std::ffi::CString;
use std::fmt::{self, Debug};
use std::mem;
use std::string::String;
use std::ops;
use std::cmp;
use std::hash::{Hash, Hasher};
use std::vec::Vec;

// #[cfg(unix)]
// use unix::{Buf, Slice};
// #[cfg(windows)]
// use windows::{Buf, Slice};
use sys_common::{AsInner, IntoInner, FromInner};

/// Owned, mutable OS strings.
#[derive(Clone)]
pub struct OsString {
    inner: Buf
}

/// Slices into OS strings.
pub struct OsStr {
    inner: Slice
}

impl OsString {
    /// Constructs a new empty `OsString`.
    pub fn new() -> OsString {
        OsString { inner: Buf::from_string(String::new()) }
    }

    /// Constructs an `OsString` from a byte sequence.
    ///
    /// # Platform behavior
    ///
    /// On Unix systems, any byte sequence can be successfully
    /// converted into an `OsString`.
    ///
    /// On Windows system, only UTF-8 byte sequences will successfully
    /// convert; non UTF-8 data will produce `None`.
    pub fn from_bytes<B>(bytes: B) -> Option<OsString> where B: Into<Vec<u8>> {
        Self::_from_bytes(bytes.into())
    }

    fn _from_bytes(vec: Vec<u8>) -> Option<OsString> {
        if_unix_windows! {
            {
                use unix::OsStringExt;
                Some(OsString::from_vec(vec))
            }
            {
                String::from_utf8(vec).ok().map(OsString::from)
            }
        }
    }


    /// Converts to an `OsStr` slice.
    pub fn as_os_str(&self) -> &OsStr {
        self
    }

    /// Converts the `OsString` into a `String` if it contains valid Unicode data.
    ///
    /// On failure, ownership of the original `OsString` is returned.
    pub fn into_string(self) -> Result<String, OsString> {
        self.inner.into_string().map_err(|buf| OsString { inner: buf} )
    }

    /// Extends the string with the given `&OsStr` slice.
    pub fn push<T: AsRef<OsStr>>(&mut self, s: T) {
        self.inner.push_slice(&s.as_ref().inner)
    }
}

impl From<String> for OsString {
    fn from(s: String) -> OsString {
        OsString { inner: Buf::from_string(s) }
    }
}

impl<'a, T: ?Sized + AsRef<OsStr>> From<&'a T> for OsString {
    fn from(s: &'a T) -> OsString {
        s.as_ref().to_os_string()
    }
}

impl ops::Index<ops::RangeFull> for OsString {
    type Output = OsStr;

    #[inline]
    fn index(&self, _index: ops::RangeFull) -> &OsStr {
        OsStr::from_inner(self.inner.as_slice())
    }
}

impl ops::Deref for OsString {
    type Target = OsStr;

    #[inline]
    fn deref(&self) -> &OsStr {
        &self[..]
    }
}

impl Debug for OsString {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        fmt::Debug::fmt(&**self, formatter)
    }
}

impl PartialEq for OsString {
    fn eq(&self, other: &OsString) -> bool {
        &**self == &**other
    }
}

impl PartialEq<str> for OsString {
    fn eq(&self, other: &str) -> bool {
        &**self == other
    }
}

impl PartialEq<OsString> for str {
    fn eq(&self, other: &OsString) -> bool {
        &**other == self
    }
}

impl Eq for OsString {}

impl PartialOrd for OsString {
    #[inline]
    fn partial_cmp(&self, other: &OsString) -> Option<cmp::Ordering> {
        (&**self).partial_cmp(&**other)
    }
    #[inline]
    fn lt(&self, other: &OsString) -> bool { &**self < &**other }
    #[inline]
    fn le(&self, other: &OsString) -> bool { &**self <= &**other }
    #[inline]
    fn gt(&self, other: &OsString) -> bool { &**self > &**other }
    #[inline]
    fn ge(&self, other: &OsString) -> bool { &**self >= &**other }
}

impl PartialOrd<str> for OsString {
    #[inline]
    fn partial_cmp(&self, other: &str) -> Option<cmp::Ordering> {
        (&**self).partial_cmp(other)
    }
}

impl Ord for OsString {
    #[inline]
    fn cmp(&self, other: &OsString) -> cmp::Ordering {
        (&**self).cmp(&**other)
    }
}

impl Hash for OsString {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        (&**self).hash(state)
    }
}

impl OsStr {
    /// Coerces into an `OsStr` slice.
    pub fn new<S: AsRef<OsStr> + ?Sized>(s: &S) -> &OsStr {
        s.as_ref()
    }

    fn from_inner(inner: &Slice) -> &OsStr {
        unsafe { mem::transmute(inner) }
    }

    /// Returns true if the string contains no bytes.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Yields a `&str` slice if the `OsStr` is valid unicode.
    ///
    /// This conversion may entail doing a check for UTF-8 validity.
    pub fn to_str(&self) -> Option<&str> {
        self.inner.to_str()
    }

    /// Converts an `OsStr` to a `Cow<str>`.
    ///
    /// Any non-Unicode sequences are replaced with U+FFFD REPLACEMENT CHARACTER.
    pub fn to_string_lossy(&self) -> Cow<str> {
        self.inner.to_string_lossy()
    }

    /// Copies the slice into an owned `OsString`.
    pub fn to_os_string(&self) -> OsString {
        OsString { inner: self.inner.to_owned() }
    }

    /// Yields this `OsStr` as a byte slice.
    ///
    /// # Platform behavior
    ///
    /// On Unix systems, this is a no-op.
    ///
    /// On Windows systems, this returns `None` unless the `OsStr` is
    /// valid unicode, in which case it produces UTF-8-encoded
    /// data. This may entail checking validity.
    pub fn to_bytes(&self) -> Option<&[u8]> {
        if is_windows!() {
            self.to_str().map(|s| s.as_bytes())
        } else {
            Some(self.bytes())
        }
    }

    /// Creates a `CString` containing this `OsStr` data.
    ///
    /// Fails if the `OsStr` contains interior nulls.
    ///
    /// This is a convenience for creating a `CString` from
    /// `self.to_bytes()`, and inherits the platform behavior of the
    /// `to_bytes` method.
    pub fn to_cstring(&self) -> Option<CString> {
        self.to_bytes().and_then(|b| CString::new(b).ok())
    }

    /// Gets the underlying byte representation.
    ///
    /// Note: it is *crucial* that this API is private, to avoid
    /// revealing the internal, platform-specific encodings.
    fn bytes(&self) -> &[u8] {
        unsafe { mem::transmute(&self.inner) }
    }
}

impl PartialEq for OsStr {
    fn eq(&self, other: &OsStr) -> bool {
        self.bytes().eq(other.bytes())
    }
}

impl PartialEq<str> for OsStr {
    fn eq(&self, other: &str) -> bool {
        *self == *OsStr::new(other)
    }
}

impl PartialEq<OsStr> for str {
    fn eq(&self, other: &OsStr) -> bool {
        *other == *OsStr::new(self)
    }
}

impl Eq for OsStr {}

impl PartialOrd for OsStr {
    #[inline]
    fn partial_cmp(&self, other: &OsStr) -> Option<cmp::Ordering> {
        self.bytes().partial_cmp(other.bytes())
    }
    #[inline]
    fn lt(&self, other: &OsStr) -> bool { self.bytes().lt(other.bytes()) }
    #[inline]
    fn le(&self, other: &OsStr) -> bool { self.bytes().le(other.bytes()) }
    #[inline]
    fn gt(&self, other: &OsStr) -> bool { self.bytes().gt(other.bytes()) }
    #[inline]
    fn ge(&self, other: &OsStr) -> bool { self.bytes().ge(other.bytes()) }
}

impl PartialOrd<str> for OsStr {
    #[inline]
    fn partial_cmp(&self, other: &str) -> Option<cmp::Ordering> {
        self.partial_cmp(OsStr::new(other))
    }
}

// FIXME (#19470): cannot provide PartialOrd<OsStr> for str until we
// have more flexible coherence rules.

impl Ord for OsStr {
    #[inline]
    fn cmp(&self, other: &OsStr) -> cmp::Ordering { self.bytes().cmp(other.bytes()) }
}

impl Hash for OsStr {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.bytes().hash(state)
    }
}

impl Debug for OsStr {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.inner.fmt(formatter)
    }
}

impl Borrow<OsStr> for OsString {
    fn borrow(&self) -> &OsStr { &self[..] }
}

impl ToOwned for OsStr {
    type Owned = OsString;
    fn to_owned(&self) -> OsString { self.to_os_string() }
}

impl AsRef<OsStr> for OsStr {
    fn as_ref(&self) -> &OsStr {
        self
    }
}

impl AsRef<OsStr> for OsString {
    fn as_ref(&self) -> &OsStr {
        self
    }
}

impl AsRef<OsStr> for str {
    fn as_ref(&self) -> &OsStr {
        OsStr::from_inner(Slice::from_str(self))
    }
}

impl AsRef<OsStr> for String {
    fn as_ref(&self) -> &OsStr {
        (&**self).as_ref()
    }
}

impl FromInner<Buf> for OsString {
    fn from_inner(buf: Buf) -> OsString {
        OsString { inner: buf }
    }
}

impl IntoInner<Buf> for OsString {
    fn into_inner(self) -> Buf {
        self.inner
    }
}

impl AsInner<Slice> for OsStr {
    fn as_inner(&self) -> &Slice {
        &self.inner
    }
}


#[cfg(test)]
mod tests {
    use std::prelude::v1::*;
    use std::borrow::Cow;
    use super::*;

    fn utf8_str() -> &'static str { "aé 💩" }
    fn utf8_osstring() -> OsString {
        OsString::from(utf8_str())
    }

    fn non_utf8_osstring() -> OsString {
        if_unix_windows! {
            {
                use unix::OsStringExt;
                let string = OsString::from_vec(vec![0xFF]);
                assert!(string.to_str().is_none());
                string
            }
            {
                use windows::OsStringExt;
                let string = OsString::from_wide(&[0xD800]);
                assert!(string.to_str().is_none());
                string
            }
        }
    }


    #[test]
    fn osstring_eq_smoke() {
        assert_eq!(OsString::new(), OsString::new());
        let string = OsString::from("abc");
        assert_eq!(utf8_osstring(), utf8_osstring());
        assert!(OsString::new() != utf8_osstring());
        assert!(utf8_osstring() != string);
        assert_eq!(non_utf8_osstring(), non_utf8_osstring());
    }

    #[test]
    fn osstring_from_bytes() {
        assert_eq!(OsString::from_bytes(utf8_str().as_bytes()),
                   Some(OsString::from(utf8_str())));
    }

    #[test]
    fn osstring_into_string() {
        assert_eq!(utf8_osstring().into_string(), Ok(utf8_str().to_string()));
        assert_eq!(non_utf8_osstring().into_string(), Err(non_utf8_osstring()));
    }

    #[test]
    fn osstring_push() {
        let mut string = OsString::new();
        string.push("foo");
        string.push("x");
        string.push(utf8_osstring());
        assert_eq!(string, OsString::from(["foox", utf8_str()].concat()));
        string.push(non_utf8_osstring());
        assert!(string.into_string().is_err());
    }

    #[test]
    fn osstr_is_empty() {
        assert!(OsString::new().is_empty());
        assert!(!utf8_osstring().is_empty());
        assert!(!non_utf8_osstring().is_empty());
    }

    #[test]
    fn osstr_to_str() {
        assert_eq!(utf8_osstring().to_str(), Some(utf8_str()));
        assert_eq!(non_utf8_osstring().to_str(), None);
    }

    #[test]
    fn osstr_to_string_lossy() {
        assert_eq!(utf8_osstring().to_string_lossy(),
                   Cow::Borrowed(utf8_str()));
        assert_eq!(non_utf8_osstring().to_string_lossy(),
                   String::from_utf8_lossy(b"\xFF"));
    }

    #[test]
    fn osstr_to_bytes() {
        assert_eq!(utf8_osstring().to_bytes(), Some(utf8_str().as_bytes()));
        if_unix_windows! {
            {
                assert_eq!(non_utf8_osstring().to_bytes(), Some(&b"\xFF"[..]));
            }
            {
                assert_eq!(non_utf8_osstring().to_bytes(), None);
            }
        }
    }

    #[test]
    fn osstring_compare_str() {
        assert_eq!(&utf8_osstring(), utf8_str());
        assert!(non_utf8_osstring() != *"");
    }

}
