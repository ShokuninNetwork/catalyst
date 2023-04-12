use anyhow::{Result, Ok};
use xtask_wasm::{Dist, WasmOpt};

fn main() -> Result<()> {
    let mut dist = Dist::default()
    .dist_dir_path("./dist")
    .app_name("wasm-logic");
    dist.release = true;
    dist.no_default_features = true;
    dist.features = vec!["wasm".to_string()];
    dist.run("catalyst-wasm-logic")?;
    WasmOpt::level(4)
    .shrink(4)
    .optimize("./dist/wasm-logic.wasm")?;
    Ok(())
}