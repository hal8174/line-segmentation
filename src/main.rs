use anyhow::Result;
use clap::Parser;
use image::{io::Reader as ImageReader, GrayImage, ImageBuffer, Luma, RgbImage, SubImage};
use std::path::PathBuf;

/// A utility for extracting individual lines of text from an image
#[derive(Parser)]
struct Args {
    /// Path of the image
    image: PathBuf,
    /// Float between 0 and 1 specifying the percentile of brightness values used to separate 'black' and 'white' lines
    #[arg(short,long,default_value_t = 0.3)]
    cutoff: f64,
    /// Float between 0 and 1 specifying the percentage of 'white' lines **before** the text line to be included in the extract.
    #[arg(short,long,default_value_t = 0.7)]
    above: f64,
    /// Float between 0 and 1 specifying the percentage of 'white' lines **after** the text line to be included in the extract.
    #[arg(short,long,default_value_t = 0.6)]
    below: f64,
}

fn draw_rows(rows: Vec<f64>, path: &str) -> Result<()> {
    let mut average: GrayImage = ImageBuffer::new(200, rows.len() as u32);

    for y in 0..average.height() {
        let pixel = Luma([rows[y as usize] as u8]);
        for x in 0..average.width() {
            *average.get_pixel_mut(x, y) = pixel;
        }
    }

    average.save(path)?;

    Ok(())
}

fn extract(i: u32, start: u32, end: u32, img: &RgbImage) -> Result<()> {

    let subimg = SubImage::new(img, 0, start, img.width(), end - start);

    subimg.to_image().save(format!("out/{:02}.png", i))?;

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    let img = ImageReader::open(args.image.clone())?.decode()?.to_rgb8();

    let mut rows = Vec::with_capacity(img.height() as usize);

    for y in 0..img.height() {
        let mut sum = 0;
        for x in 0..img.width() {
            let p = img.get_pixel(x, y);
            sum += p.0[0] as u32 + p.0[1] as u32 + p.0[2] as u32;
        }

        let value = sum as f64 / (img.width() as f64 * 3.0);

        rows.push(value);
    }

    let mut rows_med = rows.clone();
    rows_med.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = rows_med[(rows_med.len() as f64 * args.cutoff) as usize];

    let mut img_view = ImageReader::open(args.image)?.decode()?.to_luma8();

    for y in 0..rows.len() {
        if rows[y] < median {
            rows[y] = 0.0;
            for x in 0..img_view.width() {
                let p = img_view.get_pixel_mut(x, y as u32);
                *p = Luma([p.0[0] / 2])
            }
        } else {
            rows[y] = 255.0;
        }
    }

    std::fs::create_dir_all("out")?;

    let mut white = true;
    let mut blocks = Vec::new();
    let mut start = 0;
    for y in 0..rows.len() {
        if (rows[y] < median) && (white == true) {
            start = y;
            white = false;
        } else if (rows[y] >= median) && (white == false) {
            blocks.push((start, y - 1));
            white = true;
        }
    }

    let start = blocks[0].0 as u32 - ((blocks[0].0 - 0) as f64 * args.above) as u32;
    let end = blocks[0].1 as u32 + ((blocks[1].0 - blocks[0].1) as f64 * args.below) as u32;
    extract(0, start, end, &img)?;

    for i in 1..blocks.len() - 1 {
        let start = blocks[i].0 as u32
            - ((blocks[i].0 - blocks[i - 1].1) as f64 * args.above) as u32;
        let end = blocks[i].1 as u32
            + ((blocks[i + 1].0 - blocks[i].1) as f64 * args.below) as u32;

        extract(i as u32, start, end, &img)?;
    }

    let i = blocks.len() - 1;
    let start =
        blocks[i].0 as u32 - ((blocks[i].0 - blocks[i - 1].1) as f64 * args.above) as u32;
    let end = blocks[i].1 as u32
        + ((img.height() as usize - blocks[i].1) as f64 * args.below) as u32;
    extract(i as u32, start, end, &img)?;

    img_view.save("view.png")?;

    draw_rows(rows, "average.png")?;

    Ok(())
}
