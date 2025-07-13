use anyhow::{Context, Result};
use clap::Parser;
use resvg::{render, usvg};
use tiny_skia;

enum ImgFormat {
    Svg,
    Png,
}

#[derive(Parser, Debug)]
#[command(
    name = "latex-image",
    version = "1.0",
    about = "A tool to render LaTeX math formulas to SVG or PNG",
    long_about = None
)]

struct Args {
    /// LaTeX math formula to render
    #[arg(short, long)]
    formula: String,

    /// Output file path(*.png or *.svg)
    #[arg(short, long, default_value = None)]
    output: Option<String>,

    /// Font size
    #[arg(short = 's', long, default_value_t = 30.0)]
    font_size: f32,

    /// Background color in hex format (e.g. FFFFFF for white)
    #[arg(short = 'b', long, default_value = "transparent")]
    background: String,

    /// Text color in hex format (e.g. 000000 for black)
    #[arg(short = 'c', long, default_value = "000000")]
    text_color: String,
}

fn render_math_to_svg(latex: &str) -> Result<String> {
    mathjax_svg::convert_to_svg(latex)
        .with_context(|| format!("Failed to convert LaTeX to SVG: {}", latex))
}

fn save_svg(svg_data: &str, output_path: &str) -> Result<()> {
    std::fs::write(output_path, svg_data)
        .with_context(|| format!("Failed to write SVG to {}", output_path))
}

fn render_svg_to_png(
    svg_data: &str,
    output_path: &str,
    font_size: f32,
    text_color: &str,
    background: &str,
) -> Result<()> {
    let opt = usvg::Options {
        font_size,
        ..usvg::Options::default()
    };
    let rtree = usvg::Tree::from_data(svg_data.as_bytes(), &opt)
        .with_context(|| "Failed to parse SVG data")?;

    let pixmap_size = rtree.size().to_int_size();
    let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height())
        .with_context(|| "Failed to create pixmap")?;

    render(
        &rtree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );

    if let Some(color) = parse_hex_color(text_color) {
        let text_color = tiny_skia::PremultipliedColorU8::from_rgba(
            (color.red() * 255.0) as u8,
            (color.green() * 255.0) as u8,
            (color.blue() * 255.0) as u8,
            (color.alpha() * 255.0) as u8,
        )
        .unwrap_or(tiny_skia::PremultipliedColorU8::from_rgba(255, 255, 255, 255).unwrap());

        let bg_color = if background != "transparent" {
            parse_hex_color(background).and_then(|color| {
                Some(
                    tiny_skia::PremultipliedColorU8::from_rgba(
                        (color.red() * 255.0) as u8,
                        (color.green() * 255.0) as u8,
                        (color.blue() * 255.0) as u8,
                        (color.alpha() * 255.0) as u8,
                    )
                    .unwrap_or(tiny_skia::PremultipliedColorU8::from_rgba(0, 0, 0, 255).unwrap()),
                )
            })
        } else {
            None
        };

        for pixel in pixmap.pixels_mut() {
            if pixel.alpha() > 0 {
                *pixel = text_color.clone();
            } else {
                if background != "transparent" && bg_color.is_some() {
                    *pixel = bg_color.unwrap().clone();
                }
            }
        }
    }

    pixmap
        .save_png(output_path)
        .with_context(|| format!("Failed to save PNG to {}", output_path))?;

    Ok(())
}

fn parse_hex_color(hex: &str) -> Option<tiny_skia::Color> {
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(tiny_skia::Color::from_rgba8(r, g, b, 255))
    } else if hex.len() == 8 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
        Some(tiny_skia::Color::from_rgba8(r, g, b, a))
    } else {
        None
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    let svg_data = render_math_to_svg(&args.formula)?;

    if args.output.is_none() {
        println!("{}", svg_data.to_string());
        return Ok(());
    }

    let output = args.output.unwrap();

    let format = if output.to_lowercase().ends_with(".png") {
        ImgFormat::Png
    } else if output.to_lowercase().ends_with(".svg") {
        ImgFormat::Svg
    } else {
        anyhow::bail!("Unsupported output format");
    };

    match format {
        ImgFormat::Svg => save_svg(&svg_data, &output)?,
        ImgFormat::Png => render_svg_to_png(
            &svg_data,
            &output,
            args.font_size,
            &args.text_color,
            &args.background,
        )?,
    }

    println!("Successfully rendered to {}", output);
    Ok(())
}
