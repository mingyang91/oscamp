#![feature(new_uninit)]
#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

extern crate alloc;
#[cfg(feature = "axstd")]
extern crate axstd as std;

#[macro_use]
extern crate axlog;

mod loader;
mod syscall;
mod task;

use alloc::sync::{Arc, Weak};
use axhal::arch::UspaceContext;
use axhal::mem::VirtAddr;
use axhal::paging::MappingFlags;
use axhal::trap::{register_trap_handler, PAGE_FAULT};
use axmm::{kernel_aspace, AddrSpace};
use axstd::io;
use axsync::Mutex;
use core::mem::MaybeUninit;
use loader::load_user_app;

const USER_STACK_SIZE: usize = 0x10000;
const KERNEL_STACK_SIZE: usize = 0x40000; // 256 KiB
const APP_ENTRY: usize = 0x1000;

#[register_trap_handler(PAGE_FAULT)]
fn handle_page_fault(vaddr: VirtAddr, flags: MappingFlags, is_user: bool) -> bool {
    unsafe { USPACE.upgrade() }
        .expect("No user address space!")
        .lock()
        .handle_page_fault(vaddr, flags | MappingFlags::USER)
}

static mut USPACE: Weak<Mutex<AddrSpace>> = Weak::new();

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    // A new address space for user app.
    let mut uspace = axmm::new_user_aspace().unwrap();

    // Load user app binary file into address space.
    if let Err(e) = load_user_app("/sbin/origin", &mut uspace) {
        panic!("Cannot load app! {:?}", e);
    }

    // Init user stack.
    let ustack_top = init_user_stack(&mut uspace, false).unwrap();
    ax_println!("New user address space: {:#x?}", uspace);

    let aspace = Arc::new(Mutex::new(uspace));
    unsafe { USPACE = Arc::downgrade(&aspace) };

    // Let's kick off the user process.
    let user_task = task::spawn_user_task(aspace, UspaceContext::new(APP_ENTRY.into(), ustack_top));

    // Wait for user process to exit ...
    let exit_code = user_task.join();
    ax_println!("monolithic kernel exit [{:?}] normally!", exit_code);
}

fn init_user_stack(uspace: &mut AddrSpace, populating: bool) -> io::Result<VirtAddr> {
    let ustack_top = uspace.end();
    let ustack_vaddr = ustack_top - crate::USER_STACK_SIZE;
    ax_println!(
        "Mapping user stack: {:#x?} -> {:#x?}",
        ustack_vaddr,
        ustack_top
    );
    uspace
        .map_alloc(
            ustack_vaddr,
            crate::USER_STACK_SIZE,
            MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
            populating,
        )
        .unwrap();
    Ok(ustack_top)
}
