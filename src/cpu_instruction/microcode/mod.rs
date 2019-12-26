mod error;
pub use error::{MicrocodeError, Result};

mod dex;
mod pha;
mod lda;
mod sta;
mod jmp;
mod eor;
mod stx;
mod adc;
mod sbc;
mod bne;
mod brk;
mod tax;

pub use dex::dex;
pub use pha::pha;
pub use lda::lda;
pub use sta::sta;
pub use jmp::jmp;
pub use eor::eor;
pub use stx::stx;
pub use adc::adc;
pub use sbc::sbc;
pub use bne::bne;
pub use brk::brk;
pub use tax::tax;
