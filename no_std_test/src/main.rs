#![no_std]
#![no_main]

extern crate alloc;

use alloc::alloc::{GlobalAlloc, Layout};

struct MyAllocator;
unsafe impl GlobalAlloc for MyAllocator {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        todo!("this is just a compile test")
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        todo!("this is just a compile test")
    }
}

#[global_allocator]
static GLOBAL: MyAllocator = MyAllocator;
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }

#[runtime_call_macro::runtime_call(
    runtime_metadata_path = "polkadot_metadata.scale"
)]
pub mod runtime {}

// Check that it's here:
#[allow(unused)]
use runtime::ty::polkadot_runtime::RuntimeCall;
