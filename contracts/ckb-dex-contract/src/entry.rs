// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

// Import heap related library from `alloc`
// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use ckb_std::{
  default_alloc,
  high_level::load_transaction,
};
use crate::error::Error;

mod signature;
mod order;

// Alloc 4K fast HEAP + 2M HEAP to receives PrefilledData
default_alloc!(4 * 1024, 2048 * 1024, 64);

pub fn main() -> Result<(), Error> {
  let witnesses = match load_transaction() {
    Ok(tx) => tx.witnesses(),
    Err(err) => return Err(err.into()),
  };

  match witnesses.get(0) {
    Some(_) => signature::validate(),
    None => order::validate(),
  }

}
