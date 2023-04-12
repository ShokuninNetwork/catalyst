use anyhow::{Result, Ok};
use xtask_wasm::{Dist, WasmOpt};

fn main() -> Result<()> {
    let mut dist = Dist::default()
    .dist_dir_path("./dist")
    .app_name("wasm-logic");
    dist.release = true;
    dist.run("catalyst-wasm-logic")?;
    WasmOpt::level(1)
    .shrink(2)
    .optimize("./dist/wasm-logic.wasm")?;
    Ok(())
}