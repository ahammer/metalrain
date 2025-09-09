//! Unified SDF Atlas CLI
//!
//! Consolidates former tools:
//!   - gen_sdf_atlas (stub)        -> removed
//!   - sdf_atlas_build (generator) -> build subcommand
//!   - sdf_atlas_gen (json pack)   -> (not retained; flow covered by build)
//!   - sdf_atlas_inspect           -> inspect subcommand
//!
//! Subcommands:
//!   build   Generate atlas PNG + JSON from implicit shape catalog (circle, triangle, square, glyphs)
//!   inspect Basic statistics about an existing atlas
//!   schema  Print embedded schema documentation
//!
//! Example build:
//!   cargo run --bin sdf_atlas -- build \
//!       --out-stem assets/shapes/sdf_atlas \
//!       --tile-size 64 --padding-px 6 --distance-span-factor 0.5

use std::path::PathBuf;
use clap::{Parser, Subcommand, Args};
use anyhow::Result;
use ball_matcher::sdf_atlas::{BuildConfig, build_atlas, write_outputs, inspect};

#[derive(Parser, Debug)]
#[command(author, version, about="Unified SDF atlas tool", long_about=None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Build a procedural SDF atlas (PNG + JSON)
    Build(BuildArgs),
    /// Inspect an existing atlas JSON (and optional PNG) for summary statistics
    Inspect(InspectArgs),
    /// Print the atlas JSON schema documentation
    Schema,
}

#[derive(Args, Debug)]
struct BuildArgs {
    #[arg(long, default_value_t=64)] tile_size: u32,
    #[arg(long, default_value_t=0)] padding_px: u32,
    #[arg(long, default_value_t=0.5)] distance_span_factor: f32,
    #[arg(long, default_value="sdf_r8")] channel_mode: String,
    /// Output path stem (without extension). If omitted defaults to assets/shapes/sdf_atlas
    #[arg(long, default_value="")] out_stem: String,
    #[arg(long)] out_png: Option<PathBuf>,
    #[arg(long)] out_json: Option<PathBuf>,
    #[arg(long, default_value="assets/fonts/DroidSansMono.ttf")] font: PathBuf,
    #[arg(long, default_value_t=1)] supersamples: u32,
    #[arg(long)] json_only: bool,
    #[arg(long)] png_only: bool,
    #[arg(long)] stdout_json: bool,
    #[arg(long)] overwrite: bool,
}

#[derive(Args, Debug)]
struct InspectArgs {
    #[arg(long)] atlas_json: PathBuf,
    #[arg(long)] atlas_png: Option<PathBuf>,
}

fn cmd_build(a: BuildArgs) -> Result<()> {
    if a.tile_size < 16 { anyhow::bail!("tile-size too small (<16)"); }
    let cfg = BuildConfig {
        tile_size: a.tile_size,
        padding_px: a.padding_px,
        distance_span_factor: a.distance_span_factor,
        channel_mode: a.channel_mode,
        out_stem: PathBuf::from(a.out_stem),
        out_png: a.out_png,
        out_json: a.out_json,
        json_only: a.json_only,
        png_only: a.png_only,
        stdout_json: a.stdout_json,
        overwrite: a.overwrite,
        font_path: a.font,
        supersamples: a.supersamples,
    };
    let artifact = build_atlas(&cfg)?;
    write_outputs(&artifact, &cfg)?;
    println!("Built atlas: {} (json {})", artifact.png_path.display(), artifact.json_path.display());
    Ok(())
}

fn cmd_inspect(a: InspectArgs) -> Result<()> {
    let res = inspect(&a.atlas_json, a.atlas_png.as_deref())?;
    println!("Atlas: {}x{} tilesize={} shapes={} mode={} dist_range={}", res.atlas_dim.0, res.atlas_dim.1, res.tile_size, res.shape_count, res.channel_mode, res.distance_range);
    Ok(())
}

fn cmd_schema() -> Result<()> {
    // Embed schema file (keeps single source of truth)
    const SCHEMA:&str = include_str!("../../assets/shapes/sdf_atlas_schema.md");
    println!("{SCHEMA}");
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Build(a) => cmd_build(a),
        Commands::Inspect(a) => cmd_inspect(a),
        Commands::Schema => cmd_schema(),
    }
}
