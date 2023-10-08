use anyhow::Result;
use clap::Parser;
use image::{io::Reader as ImageReader, GrayImage, ImageBuffer, Luma, RgbImage};
use std::path::PathBuf;

#[derive(Parser)]
struct Args {
    image: PathBuf,
    percentile: f64,
    percent_above: f64,
    percent_below: f64,
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
    let mut extract: RgbImage = ImageBuffer::new(img.width(), end - start);

    for y in 0..extract.height() {
        for x in 0..extract.width() {
            *extract.get_pixel_mut(x, y) = img.get_pixel(x, start + y).clone();
        }
    }

    extract.save(format!("out/{:02}.png", i))?;

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
    let median = rows_med[(rows_med.len() as f64 * args.percentile) as usize];

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

    let start = blocks[0].0 as u32 - ((blocks[0].0 - 0) as f64 * args.percent_above) as u32;
    let end = blocks[0].1 as u32 + ((blocks[1].0 - blocks[0].1) as f64 * args.percent_below) as u32;
    extract(0, start, end, &img)?;

    for i in 1..blocks.len() - 1 {
        let start = blocks[i].0 as u32
            - ((blocks[i].0 - blocks[i - 1].1) as f64 * args.percent_above) as u32;
        let end = blocks[i].1 as u32
            + ((blocks[i + 1].0 - blocks[i].1) as f64 * args.percent_below) as u32;

        extract(i as u32, start, end, &img)?;
    }

    let i = blocks.len() - 1;
    let start =
        blocks[i].0 as u32 - ((blocks[i].0 - blocks[i - 1].1) as f64 * args.percent_above) as u32;
    let end = blocks[i].1 as u32
        + ((img.height() as usize - blocks[i].1) as f64 * args.percent_below) as u32;
    extract(i as u32, start, end, &img)?;

    img_view.save("view.png")?;

    draw_rows(rows, "average.png")?;

    Ok(())
}
