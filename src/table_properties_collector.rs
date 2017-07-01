// Copyright 2017 PingCAP, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// See the License for the specific language governing permissions and
// limitations under the License.

use crocksdb_ffi::{self, DBEntryType, DBUserCollectedProperties, DBTablePropertiesCollector};
use libc::{c_void, c_char, c_int, uint8_t, uint64_t, size_t};
use std::collections::HashMap;
use std::ffi::CString;
use std::mem;
use std::slice;

/// `TablePropertiesCollector` provides the mechanism for users to collect
/// their own properties that they are interested in. This class is essentially
/// a collection of callback functions that will be invoked during table
/// building. It is constructed with TablePropertiesCollectorFactory. The methods
/// don't need to be thread-safe, as we will create exactly one
/// TablePropertiesCollector object per table and then call it sequentially
pub trait TablePropertiesCollector {
    /// The name of the properties collector.
    fn name(&self) -> &CString;

    /// Will be called when a new key/value pair is inserted into the table.
    fn add_userkey(&mut self,
                   key: &[u8],
                   value: &[u8],
                   entry_type: DBEntryType,
                   seq: u64,
                   file_size: u64);

    /// Will be called when a table has already been built and is ready for
    /// writing the properties block.
    fn finish(&mut self) -> HashMap<Vec<u8>, Vec<u8>>;
}

extern "C" fn name(collector: *mut c_void) -> *const c_char {
    unsafe {
        let collector = &mut *(collector as *mut Box<TablePropertiesCollector>);
        collector.name().as_ptr()
    }
}

extern "C" fn destruct(collector: *mut c_void) {
    unsafe {
        let collector = &mut *(collector as *mut Box<TablePropertiesCollector>);
        Box::from_raw(collector);
    }
}

pub extern "C" fn add_userkey(collector: *mut c_void,
                              key: *const uint8_t,
                              key_len: size_t,
                              value: *const uint8_t,
                              value_len: size_t,
                              entry_type: c_int,
                              seq: uint64_t,
                              file_size: uint64_t) {
    unsafe {
        let collector = &mut *(collector as *mut Box<TablePropertiesCollector>);
        let key = slice::from_raw_parts(key, key_len);
        let value = slice::from_raw_parts(value, value_len);
        collector.add_userkey(key, value, mem::transmute(entry_type), seq, file_size);
    }
}

pub extern "C" fn finish(collector: *mut c_void, props: *mut DBUserCollectedProperties) {
    unsafe {
        let collector = &mut *(collector as *mut Box<TablePropertiesCollector>);
        for (key, value) in collector.finish() {
            crocksdb_ffi::crocksdb_user_collected_properties_add(props,
                                                                 key.as_ptr(),
                                                                 key.len(),
                                                                 value.as_ptr(),
                                                                 value.len());
        }
    }
}

pub unsafe fn new_table_properties_collector(collector: Box<TablePropertiesCollector>)
                                             -> *mut DBTablePropertiesCollector {
    crocksdb_ffi::crocksdb_table_properties_collector_create(
        Box::into_raw(Box::new(collector)) as *mut c_void,
        name,
        destruct,
        add_userkey,
        finish,
    )
}
