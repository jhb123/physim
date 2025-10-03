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
            name: CString::new(self.name).unwrap().into_raw(),
            plugin: CString::new(self.plugin).unwrap().into_raw(),
            version: CString::new(self.version).unwrap().into_raw(),
            license: CString::new(self.license).unwrap().into_raw(),
            author: CString::new(self.author).unwrap().into_raw(),
            blurb: CString::new(self.blurb).unwrap().into_raw(),
            repo: CString::new(self.repo).unwrap().into_raw(),
        }
    }

    /// Convert from FFI into owned Rust ElementMeta (borrows strings)
    pub unsafe fn from_ffi_borrowed(meta: &ElementMetaFFI) -> Self {
        ElementMeta {
            kind: meta.kind,
            name: CStr::from_ptr(meta.name).to_str().unwrap().to_owned(),
            plugin: CStr::from_ptr(meta.plugin).to_str().unwrap().to_owned(),
            version: CStr::from_ptr(meta.version).to_str().unwrap().to_owned(),
            license: CStr::from_ptr(meta.license).to_str().unwrap().to_owned(),
            author: CStr::from_ptr(meta.author).to_str().unwrap().to_owned(),
            blurb: CStr::from_ptr(meta.blurb).to_str().unwrap().to_owned(),
            repo: CStr::from_ptr(meta.repo).to_str().unwrap().to_owned(),
        }
    }

    /// Convert from FFI and take ownership (consumes the FFI struct)
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
#[no_mangle]
pub unsafe extern "C" fn host_alloc_string(s: *const c_char) -> *mut c_char {
    if s.is_null() {
        return std::ptr::null_mut();
    }
    let cstr = CStr::from_ptr(s);
    CString::new(cstr.to_bytes()).unwrap().into_raw()
}

#[no_mangle]
pub unsafe extern "C" fn host_free_string(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

// Type definitions for plugin functions
pub type PluginGetMetaFn = unsafe extern "C" fn(
    alloc: unsafe extern "C" fn(*const c_char) -> *mut c_char,
) -> ElementMetaFFI;
