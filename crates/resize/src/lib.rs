use image::{ ImageBuffer, RgbImage, Rgb, Luma };
use indicatif::ProgressIterator;
use rayon::prelude::*;

type EnergyMap = ImageBuffer<Luma<f32>, Vec<f32>>; // Type is essentially ImageLuma32
type SeamPixel = (u32, u32);
type Seam = std::collections::HashSet<SeamPixel>;

struct SeamPixelData {
    energy: f32,
    coordinate: SeamPixel,
    previous: Option<SeamPixel>
}

impl SeamPixelData {
    fn new(w: u32, h: u32) -> Self {
        SeamPixelData {
            energy: 0.,
            coordinate: (w, h),
            previous: None
        }
    }
}

struct SeamGrid{ 
    size: (u32, u32),
    buffer: Vec<SeamPixelData> 
}

impl SeamGrid {
    fn new(w: u32, h: u32) -> Self {
        Self {
            size: (w, h),
            buffer: Vec::from((0..h).map(|y| {
                (0..w).map(move |x| SeamPixelData::new(x, y))
            }).flatten().collect::<Vec<_>>())
        }
    }

    fn get_coordinate(&self, x: u32, y: u32) -> &SeamPixelData {
        &self.buffer[((self.size.0 * y) + x) as usize]
    }

    fn get_coordinate_mut(&mut self, x: u32, y: u32) -> &mut SeamPixelData {
        &mut self.buffer[((self.size.0 * y) + x) as usize]
    }
}

pub fn resize_image_width(img: &RgbImage, to_width: u32) -> RgbImage {
    let img_size = img.dimensions();
    let mut new_size = (img_size.0, img_size.1);
    let mut img = img.clone();
    for _ in (0..img_size.0 - to_width).progress() {
        let energy_map = calculate_energy_map(&img, new_size);
        let seam = find_low_energy_seam(&energy_map, new_size);
        img = delete_seam(&img, &seam);
        new_size.0 -= 1;
    }
    return img
}

pub fn calculate_energy_map(img: &RgbImage, (w, h): (u32, u32)) -> EnergyMap {
    let coords: Vec<(u32, u32)> = (0..h)
        .collect::<Vec<_>>()
        .iter()
        .map(|&y| (0..w).map(move |x| (x, y)))
        .flatten()
        .collect();

    let raw_energy_buffer: Vec<f32> = coords.par_iter().map(|(x, y)| {
            let left = match x {
                0 => None,
                _ => img.get_pixel_checked(x.saturating_sub(1) , *y)
            };
            let middle = img.get_pixel(*x, *y);
            let right = img.get_pixel_checked(x + 1, *y);
            let pixel_energy = get_pixel_energy(left, middle, right);
            pixel_energy
    }).collect();

    let energy_map = EnergyMap::from_vec(w, h, raw_energy_buffer).unwrap();
    energy_map
}

pub fn find_low_energy_seam(energy_map: &EnergyMap, (w, h): (u32, u32)) -> Seam {
    let mut seams_energies = SeamGrid::new(w, h);

    for ix in 0..w {
        seams_energies.buffer[ix as usize].energy = energy_map.get_pixel(ix, 0).0[0];
    }

    let coords: Vec<(u32, u32)> = (1..h)
        .collect::<Vec<_>>()
        .iter()
        .map(|&y| (0..w).map(move |x| (x, y)))
        .flatten()
        .collect();

    coords.iter().for_each(|(x, y)| {
        let mut min_prev_energy = f32::INFINITY;

        let x = *x as i32;
        let mut min_prev_x = x;

        for i in x-1..x+1 {
            if i >= 0 && i < w as i32 && 
               seams_energies.get_coordinate(i as u32, y-1).energy < min_prev_energy
            {
                min_prev_energy = seams_energies.get_coordinate(i as u32, y-1).energy;
                min_prev_x = i;
            }

        }

        let energy = min_prev_energy + energy_map.get_pixel(x as u32, *y).0[0];
        let previous = Some((min_prev_x as u32, y-1));

        // ((x as u32, *y), energy, previous)
        let seam_pixel_data = seams_energies.get_coordinate_mut(x as u32, *y);
        seam_pixel_data.energy = energy;
        seam_pixel_data.previous = previous;
    });

    // Find the lowest minimum energy seam value in the final row of the SeamEnergyGrid
    //
    let mut min_energy = f32::INFINITY;
    let mut min_energy_coord = None;
    for x in 0..w {
        if seams_energies.get_coordinate(x, h - 1).energy < min_energy {
            min_energy = seams_energies.get_coordinate(x, h - 1).energy ;
            min_energy_coord = Some((x, h - 1))
        }
    }

    let mut seam = Seam::new();
    if min_energy_coord.is_none() {
        return seam
    }

    let (last_min_x, last_min_y) = min_energy_coord.unwrap();

    let mut current_seam = Some(seams_energies.get_coordinate(last_min_x, last_min_y));

    while current_seam.is_some() {
        seam.insert(current_seam.unwrap().coordinate);
        let previous_min_coord = current_seam.unwrap().previous;

        match previous_min_coord {
            None => { current_seam = None }
            Some((x, y)) => {
                current_seam = Some(seams_energies.get_coordinate(x, y))
            }
        }
    }

    seam
}

pub fn delete_seam(img: &RgbImage, seam: &Seam) -> RgbImage {
    let (w, h) = img.dimensions();

    let resized_buffer = img
        .enumerate_pixels()
        .filter(|(x, y, _pixel)| {
            !seam.contains(&(x.clone(), y.clone()))        
        })
        .map(|(_x, _y, pixel)| {
            pixel
        });

    let buf: Vec<u8> = resized_buffer
        .flat_map(|rgb| rgb.0.iter()).cloned().collect();

    let new_img = RgbImage::from_raw(w - 1, h, buf).unwrap();

    new_img
}

fn get_pixel_energy(left: Option<&Rgb<u8>>, middle: &Rgb<u8>, right: Option<&Rgb<u8>>) -> f32 {

    let Rgb([m_r, m_g, m_b]) = middle;
    
    let left_energy = left.map(|pix| {
        let Rgb([l_r, l_g, l_b]) = pix;

        (*l_r as i32 - *m_r as i32).pow(2) + 
        (*l_g as i32 - *m_g as i32).pow(2) + 
        (*l_b as i32 - *m_b as i32).pow(2)
    });

    let right_energy = right.map(|pix| {
        let Rgb([r_r, r_g, r_b]) = pix;

        (*r_r as i32 - *m_r as i32).pow(2) + 
        (*r_g as i32 - *m_g as i32).pow(2) + 
        (*r_b as i32 - *m_b as i32).pow(2)
    });

    let energy_sum = match (left_energy, right_energy) {
        (Some(l), Some(r)) => l + r,
        (None, Some(r)) => r,
        (Some(l), None) => l,
        (None, None) => 0
    };

    (energy_sum as f32).sqrt()
}


#[cfg(test)]
mod tests {
    use super::get_pixel_energy;
    use image::Rgb;

    #[test]
    fn test_get_pixel_energy() {
        let left = None;

        let (mr, mg, mb) = (10, 10, 10);
        let (rr, rg, rb) = (20, 20, 20);

        let r: Rgb<u8> = Rgb([rr, rg, rb]);

        let middle: &Rgb<u8> = &Rgb([mr, mg, mb]);
        let right = Some(&r);

        let energy = get_pixel_energy(left, middle, right);

        let energy_ = (
            (rr as i32 - mr as i32).pow(2) +
            (rg as i32 - mg as i32).pow(2) +
            (rb as i32 - mb as i32).pow(2)
        ) as f32;

        assert_eq!(energy, energy_.sqrt());
    }
}
