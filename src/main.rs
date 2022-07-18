use image::{ ImageBuffer, RgbImage, Rgb, Luma };

type EnergyMap = ImageBuffer<Luma<f32>, Vec<f32>>; // Type is essentially ImageLuma32
type SeamPixel = (u32, u32);

struct SeamPixelData {
    energy: f32,
    coordinate: SeamPixel,
    previous: Option<SeamPixel>
}

impl SeamPixelData {
    fn new(w: u32, h: u32) -> Self {
        SeamPixelData {
            energy: 0.,
            coordinate: (0, 0),
            previous: None
        }
    }
}

struct Seam(Vec<SeamPixel>);

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

fn main() {
    let mut img = image::open("")
        .unwrap().to_rgb8();

    let size = img.dimensions();

    let resize_width = size.0 / 2;

    println!("size: {:?}", size);
    // let pixel = img[coord.into()];

    for y in 0..size.1 {
        img.put_pixel(500, y, [0, 0, 0 ].into());
    }

    img.save("").unwrap();

}

fn resize_image_width(img: &RgbImage, to_width: u32) -> &RgbImage {
    let img_size = img.dimensions();

    let mut new_size = (to_width, img_size.1);

    // let energyMap = new();
    for y in 0..img_size.1 {
        // img.put_pixel(500, y, [0, 0, 0 ].into());
        let energy_map = calculate_energy_map(img, new_size);
        let seam = find_low_energy_seam(energy_map, new_size);

        delete_seam(img, seam);

        new_size.0 -= 1;
    }

    return img
}


fn calculate_energy_map(img: &RgbImage, (w, h): (u32, u32)) -> EnergyMap {
    let mut energy_map = EnergyMap::new(w, h);
    for y in 0..=h {
        for x in 0..=w {
            let left = img.get_pixel_checked(x - 1, y);
            let middle = img.get_pixel(x, y);
            let right = img.get_pixel_checked(x + 1, y);
            let pixel_energy = get_pixel_energy(left, middle, right);
            energy_map.put_pixel(x, y, pixel_energy);
        }
    }
    energy_map
}

fn find_low_energy_seam(energy_map: EnergyMap, (w, h): (u32, u32)) -> Seam {
    let mut seams_energies = SeamGrid::new(w, h);

    for ix in 0..w {
        seams_energies.buffer[ix as usize].energy = energy_map.get_pixel(0, ix).0[0];
    }

    for y in 1..h {
        for x in 0..w {
            let mut min_prev_energy = f32::INFINITY;
            let mut min_prev_x = x;

            for i in x-1..x+1 {
                if i >= 0 && 
                   i < w && 
                   seams_energies.get_coordinate(i, y-1).energy < min_prev_energy
                {
                    min_prev_energy = seams_energies.get_coordinate(i, y-1).energy;
                    min_prev_x = i;
                }

            }

            let seam_pixel_data = seams_energies.get_coordinate_mut(x, y);
            seam_pixel_data.energy = min_prev_energy + energy_map.get_pixel(x, y).0[0];
            seam_pixel_data.previous = Some((min_prev_x, y-1))
        }
    }

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

    let mut seam = Seam(Vec::new());
    if min_energy_coord.is_none() {
        return seam
    }

    let (last_min_x, last_min_y) = min_energy_coord.unwrap();

    let mut current_seam = Some(seams_energies.get_coordinate(last_min_x, last_min_y));

    while current_seam.is_some() {
        seam.0.push(current_seam.unwrap().coordinate);
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

fn delete_seam(img: &RgbImage, seam: Seam) -> RgbImage {
    let (w, h) = img.dimensions();
    let img = img.clone();

    let resized_buffer_flags: Vec<_> = img
        .enumerate_pixels()
        .map(|(x, y, pixel)| {
            if seam.0.contains(&(x.clone(), y.clone())) {
                false
            } else {
                true
            }
        })
        .collect();

    let resized_buffer = img
        .into_raw()
        .into_iter()
        .zip(resized_buffer_flags.into_iter())
        .filter(|(b, f)| *f)
        .map(|(b, g)| b)
        .collect();

    let resized_image = ImageBuffer::from_raw(w - 1, h, resized_buffer).unwrap();
    
    resized_image
}

fn get_pixel_energy(left: Option<&Rgb<u8>>, middle: &Rgb<u8>, right: Option<&Rgb<u8>>) -> Luma<f32> {

    let Rgb([m_R, m_G, m_B]) = middle;
    
    let left_energy = left.map(|pix| {
        let Rgb([l_R, l_G, l_B]) = pix;
        (l_R - m_R).pow(2) + (l_G - m_G).pow(2) + (l_B - m_B).pow(2)
    });

    let right_energy = right.map(|pix| {
        let Rgb([r_R, r_G, r_B]) = pix;
        (r_R - m_R).pow(2) + (r_G - m_G).pow(2) + (r_B - m_B).pow(2)
    });

    let energy_sum = left_energy.zip(right_energy).map(|(a, b)| a + b).unwrap();

    Luma([ (energy_sum as f32).sqrt() ])
}
