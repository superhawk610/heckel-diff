use heckel_diff::heckel_diff;
use std::fs::File;

fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    let left = File::open("assets/left.txt")?;
    let right = File::open("assets/right.txt")?;
    heckel_diff(&left, &right)?;

    Ok(())
}
