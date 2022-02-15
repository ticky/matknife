#[macro_use]
extern crate log;

use anyhow::{bail, Result};
use image::{GenericImage, GenericImageView, ImageBuffer, Pixel};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
/// Split a Unity-style combined metallic and smoothness texture image
/// into Pixar USD-style separate images for metallic and roughness.
struct Split {
    /// The texture file to split
    ///
    /// Must be a greyscale image with an alpha channel, where black means
    /// non-metallic and white means metallic, and completely transparent
    /// means perfectly rough and completely opaque means perfectly smooth
    #[structopt(parse(from_os_str))]
    file: PathBuf,
}

fn split(options: Split) -> Result<()> {
    debug!("{:?}", options);

    println!("Splitting {:?} into two files...", options.file);

    let mut image = image::open(options.file.clone())?;

    if !image.color().has_alpha() {
        bail!("Input image does not have an alpha channel!");
    }

    let file_stem = options
        .file
        .file_stem()
        .expect("Could not determine file name");

    let (width, height) = image.dimensions();
    let mut alpha_image: ImageBuffer<image::Luma<u8>, Vec<_>> = ImageBuffer::new(width, height);

    for y_position in 0..height {
        for x_position in 0..width {
            let mut output_pixel = image::Luma::<u8>([0x00]);

            let input_pixel = image.get_pixel(x_position, y_position).map_with_alpha(
                |channel| channel,
                |alpha| {
                    output_pixel = image::Luma::<u8>([0xff - alpha]);
                    0xff
                },
            );

            image.put_pixel(x_position, y_position, input_pixel);
            alpha_image.put_pixel(x_position, y_position, output_pixel);
        }
    }

    let mut filename: String = file_stem.to_string_lossy().to_string();

    if let Some(basename) = filename.strip_suffix("MetallicSmoothness") {
        filename = basename.to_string();
    }

    debug!("filename: {:?}", filename);

    let metallic_path = options
        .file
        .with_file_name(format!("{}{}", filename, "Metallic.png"));

    println!("Writing metallic texture to: {:?}", metallic_path);
    image.save(metallic_path)?;

    let roughness_path = options
        .file
        .with_file_name(format!("{}{}", filename, "Roughness.png"));

    println!("Writing roughness texture to: {:?}", roughness_path);
    alpha_image.save(roughness_path)?;

    Ok(())
}

#[derive(Debug, StructOpt)]
/// Merge Pixar USD-style separate images for metallic and roughness
/// into a Unity-style combined metallic and smoothness texture image.
struct Merge {
    /// The metallic file
    ///
    /// Must be a greyscale image where black means non-metallic,
    /// and white means metallic
    #[structopt(parse(from_os_str))]
    metallic_file: PathBuf,

    /// The roughness file
    ///
    /// Must be a greyscale image where white means perfectly rough,
    /// and black means perfectly smooth
    #[structopt(parse(from_os_str))]
    roughness_file: PathBuf,
}

fn merge(options: Merge) -> Result<()> {
    debug!("{:?}", options);

    let mut metallic_image = image::open(options.metallic_file.clone())?;
    let roughness_image = image::open(options.roughness_file.clone())?;

    println!(
        "Merging {:?} and {:?} into one file...",
        options.metallic_file, options.roughness_file
    );

    if metallic_image.dimensions() != roughness_image.dimensions() {
        bail!("Input images are not the same size!");
    }

    let (width, height) = metallic_image.dimensions();

    for y_position in 0..height {
        for x_position in 0..width {
            let mut value: u8 = 0x00;

            roughness_image
                .get_pixel(x_position, y_position)
                .map(|channel| {
                    value = channel;
                    channel
                });

            let new_pixel = metallic_image
                .get_pixel(x_position, y_position)
                .map_with_alpha(|_channel| 0x00, |_alpha| 0xff - value);

            metallic_image.put_pixel(x_position, y_position, new_pixel);
        }
    }

    let file_stem = options
        .metallic_file
        .file_stem()
        .expect("Could not determine file name");

    let mut filename: String = file_stem.to_string_lossy().to_string();

    if let Some(basename) = filename.strip_suffix("Metallic") {
        filename = basename.to_string();
    }

    debug!("filename: {:?}", filename);

    let merged_path = options
        .metallic_file
        .with_file_name(format!("{}{}", filename, "MetallicSmoothness.png"));

    println!("Writing metallic+smoothness file to: {:?}", merged_path);

    metallic_image.save(merged_path)?;

    Ok(())
}

/// Convert physically-based rendering textures between Unity-style combined
/// metallic and smoothness file and Pixar USD-style separate metallic and
/// roughness files
#[derive(Debug, StructOpt)]
enum Args {
    Split(Split),
    Merge(Merge),
}

fn main() -> Result<()> {
    env_logger::init();

    let args = Args::from_args();

    debug!("args: {:?}", args);

    match args {
        Args::Split(options) => split(options),
        Args::Merge(options) => merge(options),
    }
}
