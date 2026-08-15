//! 附件明文的压缩-解压缩层。子模块为内部实现细节，对外只暴露压缩/解压 guard。

mod codec;
mod engine;
mod execute;
mod guard;
mod param;
mod route;

pub use guard::{GuardOutput, compress, decompress};
