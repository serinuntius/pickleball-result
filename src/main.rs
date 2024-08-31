use anyhow::{Context, Result};
use clap::Parser;
use rayon::prelude::*;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::iter::Peekable;
use std::path::Path;
use svg2pdf::{usvg, ConversionOptions, PageOptions};

/// Command line arguments structure
#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the CSV file containing player names
    #[arg(short, long)]
    csv_path: String,
    /// Path to the master SVG template file
    #[arg(short, long)]
    svg_path: String,
    /// Name of the tournament
    #[arg(short, long)]
    tournament_name: String,
    /// Path for the output PDF files
    #[arg(short, long)]
    output_path: String,
}

/// Main processing function
///
/// # Arguments
///
/// * `args` - Command line arguments
///
/// # Returns
///
/// * `Result<()>` - Ok if processing succeeds, Err otherwise
fn process(args: &Args) -> Result<()> {
    let csv_path = Path::new(&args.csv_path);
    let file = File::open(csv_path).context("Failed to open CSV file")?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines().peekable();

    validate_header(&mut lines)?;

    let master_svg_str =
        std::fs::read_to_string(&args.svg_path).context("Failed to read SVG file")?;

    process_player_groups(&mut lines, &master_svg_str, args)?;

    Ok(())
}

/// Validates the CSV header
///
/// # Arguments
///
/// * `lines` - Iterator over the lines of the CSV file
///
/// # Returns
///
/// * `Result<()>` - Ok if header is valid, Err otherwise
fn validate_header<I>(lines: &mut I) -> Result<()>
where
    I: Iterator<Item = std::io::Result<String>>,
{
    let header = lines.next().context("Failed to read header line")??;

    if header != "player1,player2,player3,player4" {
        anyhow::bail!("Invalid header: {}", header);
    }

    Ok(())
}

/// Processes player groups from CSV
///
/// # Arguments
///
/// * `lines` - Peekable iterator over the lines of the CSV file
/// * `master_svg_str` - String containing the master SVG template
/// * `args` - Command line arguments
///
/// # Returns
///
/// * `Result<()>` - Ok if processing succeeds, Err otherwise
fn process_player_groups<I>(
    lines: &mut Peekable<I>,
    master_svg_str: &str,
    args: &Args,
) -> Result<()>
where
    I: Iterator<Item = std::io::Result<String>>,
{
    let player_groups: Vec<Vec<String>> = lines
        .map(|line| line.map(|l| l.split(',').map(String::from).collect::<Vec<String>>()))
        .collect::<Result<Vec<_>, _>>()?;

    player_groups
        .par_chunks(4)
        .enumerate()
        .try_for_each(|(group_index, chunk)| {
            process_group(
                master_svg_str,
                chunk,
                &args.tournament_name,
                &args.output_path,
                group_index,
            )
        })?;

    Ok(())
}

/// Converts SVG to PDF
///
/// # Arguments
///
/// * `svg_str` - String containing the SVG content
/// * `output_path` - Path where the PDF will be saved
///
/// # Returns
///
/// * `Result<()>` - Ok if conversion succeeds, Err otherwise
fn svg_to_pdf(svg_str: &str, output_path: &str) -> Result<()> {
    let mut options = usvg::Options::default();
    options.fontdb_mut().load_system_fonts();

    let tree = usvg::Tree::from_str(svg_str, &options)?;

    let pdf = svg2pdf::to_pdf(&tree, ConversionOptions::default(), PageOptions::default());
    std::fs::write(output_path, pdf)?;

    Ok(())
}

/// Processes a single group of players
///
/// # Arguments
///
/// * `master_svg_str` - String containing the master SVG template
/// * `player_groups` - Vector of player groups
/// * `tournament_name` - Name of the tournament
/// * `output_path` - Base path for output files
/// * `group_index` - Index of the current group
///
/// # Returns
///
/// * `Result<()>` - Ok if processing succeeds, Err otherwise
fn process_group(
    master_svg_str: &str,
    player_groups: &[Vec<String>],
    tournament_name: &str,
    output_path: &str,
    group_index: usize,
) -> Result<()> {
    let svg_result_str = replace_svg(master_svg_str, player_groups, tournament_name)?;
    let output_path = format!("{}_{}.pdf", output_path, group_index);

    svg_to_pdf(&svg_result_str, &output_path)?;

    Ok(())
}

/// Replaces placeholders in SVG with actual player names and tournament name
///
/// # Arguments
///
/// * `svg_str` - String containing the SVG template
/// * `player_groups` - Vector of player groups
/// * `tournament_name` - Name of the tournament
///
/// # Returns
///
/// * `Result<String>` - Ok with modified SVG string if successful, Err otherwise
fn replace_svg(
    svg_str: &str,
    player_groups: &[Vec<String>],
    tournament_name: &str,
) -> Result<String> {
    if player_groups.is_empty() || player_groups.iter().any(Vec::is_empty) {
        anyhow::bail!("Invalid player groups: {:?}", player_groups);
    }

    let mut svg_str = svg_str.to_string();

    // Embed player names into SVG
    for (group_index, group) in player_groups.iter().enumerate() {
        for (player_index, player_name) in group.iter().enumerate() {
            let player_number = group_index * 4 + player_index + 1;
            let player_number_str = format!(">PLAYER{}<", player_number);
            let player_name_str = format!(">{}<", player_name);
            svg_str = svg_str.replace(&player_number_str, &player_name_str);
        }
    }

    // Embed tournament name into SVG
    svg_str = svg_str.replace("NAME", tournament_name);

    Ok(svg_str)
}

/// Entry point of the program
///
/// # Returns
///
/// * `Result<()>` - Ok if program runs successfully, Err otherwise
fn main() -> Result<()> {
    let args = Args::parse();

    process(&args)
}
