/// This module provides ElementMeta and tools for passing
/// it across a FFI. This needs to work with plugins written
/// in Rust and C and for convenience, the metadata is
/// allocated on the heap. Because C and Rust store things
/// on the heap in different ways, we need to pass an
/// allocator function that will allow us to manage freeing
/// the memory in the main Rust application.
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::plugin::ElementKind;

fn cstring_escape_null(t: &str) -> CString {
    CString::new(t.replace("\0", "")).expect("Failed to make Cstring")
}

/// set by library authors, determined at compile time
#[derive(Debug, Clone)]
pub struct ElementMeta {
    pub kind: crate::plugin::ElementKind,
    pub name: String,
    pub plugin: String,
    pub version: String,
    pub license: String,
    pub author: String,
    pub blurb: String,
    pub repo: String,
}

impl ElementMeta {
    /// Convert owned Rust ElementMeta into FFI
    pub fn into_ffi(self) -> ElementMetaFFI {
        ElementMetaFFI {
            kind: self.kind,
            name: cstring_escape_null(&self.name).into_raw(),
            plugin: cstring_escape_null(&self.plugin).into_raw(),
            version: cstring_escape_null(&self.version).into_raw(),
            license: cstring_escape_null(&self.license).into_raw(),
            author: cstring_escape_null(&self.author).into_raw(),
            blurb: cstring_escape_null(&self.blurb).into_raw(),
            repo: cstring_escape_null(&self.repo).into_raw(),
        }
    }

    /// Convert from FFI into owned Rust ElementMeta (borrows strings)
    /// # Safety
    /// Consult [`CStr::from_ptr`]
    pub unsafe fn from_ffi_borrowed(meta: &ElementMetaFFI) -> Self {
        ElementMeta {
            kind: meta.kind,
            name: CStr::from_ptr(meta.name).to_string_lossy().to_string(),
            plugin: CStr::from_ptr(meta.plugin).to_string_lossy().to_string(),
            version: CStr::from_ptr(meta.version).to_string_lossy().to_string(),
            license: CStr::from_ptr(meta.license).to_string_lossy().to_string(),
            author: CStr::from_ptr(meta.author).to_string_lossy().to_string(),
            blurb: CStr::from_ptr(meta.blurb).to_string_lossy().to_string(),
            repo: CStr::from_ptr(meta.repo).to_string_lossy().to_string(),
        }
    }

    /// Convert from FFI and take ownership (consumes the FFI struct)
    /// # Safety
    /// Consult [`CStr::from_ptr`]
    pub unsafe fn from_ffi_owned(meta: ElementMetaFFI) -> Self {
        let result = Self::from_ffi_borrowed(&meta);
        meta.free();
        result
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kind: ElementKind,
        name: &str,
        plugin: &str,
        version: &str,
        license: &str,
        author: &str,
        blurb: &str,
        repo: &str,
    ) -> Self {
        Self {
            kind,
            name: name.to_string(),
            plugin: plugin.to_string(),
            version: version.to_string(),
            license: license.to_string(),
            author: author.to_string(),
            blurb: blurb.to_string(),
            repo: repo.to_string(),
        }
    }
}

/// FFI-compatible version
#[repr(C)]
#[derive(Debug)]
pub struct ElementMetaFFI {
    pub kind: crate::plugin::ElementKind,
    pub name: *mut c_char,
    pub plugin: *mut c_char,
    pub version: *mut c_char,
    pub license: *mut c_char,
    pub author: *mut c_char,
    pub blurb: *mut c_char,
    pub repo: *mut c_char,
}

impl ElementMetaFFI {
    /// Free all string memory - called by host
    /// # Safety
    ///  Consult [`CString::from_raw`]
    pub unsafe fn free(self) {
        if !self.name.is_null() {
            drop(CString::from_raw(self.name));
        }
        if !self.plugin.is_null() {
            drop(CString::from_raw(self.plugin));
        }
        if !self.version.is_null() {
            drop(CString::from_raw(self.version));
        }
        if !self.license.is_null() {
            drop(CString::from_raw(self.license));
        }
        if !self.author.is_null() {
            drop(CString::from_raw(self.author));
        }
        if !self.blurb.is_null() {
            drop(CString::from_raw(self.blurb));
        }
        if !self.repo.is_null() {
            drop(CString::from_raw(self.repo));
        }
    }
}

/// Host allocator functions to pass to plugins
/// # Safety
///  Consult [`CStr::from_ptr`]
#[no_mangle]
pub unsafe extern "C" fn host_alloc_string(s: *const c_char) -> *mut c_char {
    if s.is_null() {
        return std::ptr::null_mut();
    }
    let cstr = CStr::from_ptr(s);
    let mut bytes = cstr.to_bytes().to_vec();
    for b in bytes.iter_mut() {
        if *b == 0 {
            *b = 0xFF;
        }
    }
    CString::new(bytes)
        .expect("we replaced all null bytes with 0xFF, so CString::new cannot fail.")
        .into_raw()
}

/// # Safety
///  Consult [`CString::from_raw`]
#[no_mangle]
pub unsafe extern "C" fn host_free_string(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

pub type RustStringAllocFn = unsafe extern "C" fn(*const c_char) -> *mut c_char;

// Type definitions for plugin functions
pub type PluginGetMetaFn = unsafe extern "C" fn(alloc: RustStringAllocFn) -> ElementMetaFFI;
