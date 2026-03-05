const JAVA_MULTIPLIER: u64 = 0x5DEECE66D;
const JAVA_ADDEND: u64 = 0xB;
const JAVA_MASK: u64 = (1u64 << 48) - 1;

const SIMPLEX_F2: f64 = 0.366_025_403_784_438_6;
const SIMPLEX_G2: f64 = 0.211_324_865_405_187_13;
const SIMPLEX_F3: f64 = 1.0 / 3.0;
const SIMPLEX_G3: f64 = 1.0 / 6.0;

const GRAD3: [[f64; 3]; 12] = [
    [1.0, 1.0, 0.0],
    [-1.0, 1.0, 0.0],
    [1.0, -1.0, 0.0],
    [-1.0, -1.0, 0.0],
    [1.0, 0.0, 1.0],
    [-1.0, 0.0, 1.0],
    [1.0, 0.0, -1.0],
    [-1.0, 0.0, -1.0],
    [0.0, 1.0, 1.0],
    [0.0, -1.0, 1.0],
    [0.0, 1.0, -1.0],
    [0.0, -1.0, -1.0],
];

#[derive(Debug, Clone)]
pub struct PerlinNoise {
    permutations: [u8; 512],
    offset_x: f64,
    offset_y: f64,
    offset_z: f64,
}

impl PerlinNoise {
    pub fn new(seed: i64) -> Self {
        let permutations = build_permutation(seed);
        let mut random = JavaRandom::new(
            seed.wrapping_mul(31)
                .wrapping_add(0x9E3779B97F4A7C15u64 as i64),
        );

        Self {
            permutations,
            offset_x: random.next_f64() * 256.0,
            offset_y: random.next_f64() * 256.0,
            offset_z: random.next_f64() * 256.0,
        }
    }

    pub fn sample2d(&self, x: f64, z: f64) -> f64 {
        self.sample3d(x, 0.0, z)
    }

    pub fn sample3d(&self, x: f64, y: f64, z: f64) -> f64 {
        let x = x + self.offset_x;
        let y = y + self.offset_y;
        let z = z + self.offset_z;

        let x_floor = x.floor();
        let y_floor = y.floor();
        let z_floor = z.floor();

        let xi = (x_floor as i32 & 255) as usize;
        let yi = (y_floor as i32 & 255) as usize;
        let zi = (z_floor as i32 & 255) as usize;

        let xf = x - x_floor;
        let yf = y - y_floor;
        let zf = z - z_floor;

        let u = fade(xf);
        let v = fade(yf);
        let w = fade(zf);

        let xi1 = (xi + 1) & 255;
        let yi1 = (yi + 1) & 255;
        let zi1 = (zi + 1) & 255;

        let aaa =
            self.permutations[self.permutations[self.permutations[xi] as usize + yi] as usize + zi];
        let aba = self.permutations
            [self.permutations[self.permutations[xi] as usize + yi1] as usize + zi];
        let aab = self.permutations
            [self.permutations[self.permutations[xi] as usize + yi] as usize + zi1];
        let abb = self.permutations
            [self.permutations[self.permutations[xi] as usize + yi1] as usize + zi1];
        let baa = self.permutations
            [self.permutations[self.permutations[xi1] as usize + yi] as usize + zi];
        let bba = self.permutations
            [self.permutations[self.permutations[xi1] as usize + yi1] as usize + zi];
        let bab = self.permutations
            [self.permutations[self.permutations[xi1] as usize + yi] as usize + zi1];
        let bbb = self.permutations
            [self.permutations[self.permutations[xi1] as usize + yi1] as usize + zi1];

        let x1 = lerp(u, grad(aaa, xf, yf, zf), grad(baa, xf - 1.0, yf, zf));
        let x2 = lerp(
            u,
            grad(aba, xf, yf - 1.0, zf),
            grad(bba, xf - 1.0, yf - 1.0, zf),
        );
        let y1 = lerp(v, x1, x2);

        let x3 = lerp(
            u,
            grad(aab, xf, yf, zf - 1.0),
            grad(bab, xf - 1.0, yf, zf - 1.0),
        );
        let x4 = lerp(
            u,
            grad(abb, xf, yf - 1.0, zf - 1.0),
            grad(bbb, xf - 1.0, yf - 1.0, zf - 1.0),
        );
        let y2 = lerp(v, x3, x4);

        lerp(w, y1, y2)
    }
}

#[derive(Debug, Clone)]
pub struct SimplexNoise {
    perm: [u8; 512],
    perm_mod12: [u8; 512],
}

impl SimplexNoise {
    pub fn new(seed: i64) -> Self {
        let perm = build_permutation(seed);
        let mut perm_mod12 = [0u8; 512];

        for index in 0..perm.len() {
            perm_mod12[index] = perm[index] % 12;
        }

        Self { perm, perm_mod12 }
    }

    pub fn sample2d(&self, xin: f64, yin: f64) -> f64 {
        let s = (xin + yin) * SIMPLEX_F2;
        let i = (xin + s).floor();
        let j = (yin + s).floor();

        let t = (i + j) * SIMPLEX_G2;
        let x0 = xin - (i - t);
        let y0 = yin - (j - t);

        let (i1, j1) = if x0 > y0 {
            (1usize, 0usize)
        } else {
            (0usize, 1usize)
        };

        let x1 = x0 - i1 as f64 + SIMPLEX_G2;
        let y1 = y0 - j1 as f64 + SIMPLEX_G2;
        let x2 = x0 - 1.0 + 2.0 * SIMPLEX_G2;
        let y2 = y0 - 1.0 + 2.0 * SIMPLEX_G2;

        let ii = (i as i32 & 255) as usize;
        let jj = (j as i32 & 255) as usize;

        let gi0 = usize::from(self.perm_mod12[ii + usize::from(self.perm[jj])]);
        let gi1 = usize::from(self.perm_mod12[ii + i1 + usize::from(self.perm[jj + j1])]);
        let gi2 = usize::from(self.perm_mod12[ii + 1 + usize::from(self.perm[jj + 1])]);

        let mut n0 = 0.0;
        let mut n1 = 0.0;
        let mut n2 = 0.0;

        let t0 = 0.5 - x0 * x0 - y0 * y0;
        if t0 > 0.0 {
            let t0_sq = t0 * t0;
            n0 = t0_sq * t0_sq * dot2(GRAD3[gi0], x0, y0);
        }

        let t1 = 0.5 - x1 * x1 - y1 * y1;
        if t1 > 0.0 {
            let t1_sq = t1 * t1;
            n1 = t1_sq * t1_sq * dot2(GRAD3[gi1], x1, y1);
        }

        let t2 = 0.5 - x2 * x2 - y2 * y2;
        if t2 > 0.0 {
            let t2_sq = t2 * t2;
            n2 = t2_sq * t2_sq * dot2(GRAD3[gi2], x2, y2);
        }

        70.0 * (n0 + n1 + n2)
    }

    pub fn sample3d(&self, xin: f64, yin: f64, zin: f64) -> f64 {
        let s = (xin + yin + zin) * SIMPLEX_F3;
        let i = (xin + s).floor();
        let j = (yin + s).floor();
        let k = (zin + s).floor();

        let t = (i + j + k) * SIMPLEX_G3;
        let x0 = xin - (i - t);
        let y0 = yin - (j - t);
        let z0 = zin - (k - t);

        let (i1, j1, k1, i2, j2, k2) = if x0 >= y0 {
            if y0 >= z0 {
                (1usize, 0usize, 0usize, 1usize, 1usize, 0usize)
            } else if x0 >= z0 {
                (1usize, 0usize, 0usize, 1usize, 0usize, 1usize)
            } else {
                (0usize, 0usize, 1usize, 1usize, 0usize, 1usize)
            }
        } else if y0 < z0 {
            (0usize, 0usize, 1usize, 0usize, 1usize, 1usize)
        } else if x0 < z0 {
            (0usize, 1usize, 0usize, 0usize, 1usize, 1usize)
        } else {
            (0usize, 1usize, 0usize, 1usize, 1usize, 0usize)
        };

        let x1 = x0 - i1 as f64 + SIMPLEX_G3;
        let y1 = y0 - j1 as f64 + SIMPLEX_G3;
        let z1 = z0 - k1 as f64 + SIMPLEX_G3;
        let x2 = x0 - i2 as f64 + 2.0 * SIMPLEX_G3;
        let y2 = y0 - j2 as f64 + 2.0 * SIMPLEX_G3;
        let z2 = z0 - k2 as f64 + 2.0 * SIMPLEX_G3;
        let x3 = x0 - 1.0 + 3.0 * SIMPLEX_G3;
        let y3 = y0 - 1.0 + 3.0 * SIMPLEX_G3;
        let z3 = z0 - 1.0 + 3.0 * SIMPLEX_G3;

        let ii = (i as i32 & 255) as usize;
        let jj = (j as i32 & 255) as usize;
        let kk = (k as i32 & 255) as usize;

        let gi0 = self.hash3(ii, jj, kk);
        let gi1 = self.hash3(ii + i1, jj + j1, kk + k1);
        let gi2 = self.hash3(ii + i2, jj + j2, kk + k2);
        let gi3 = self.hash3(ii + 1, jj + 1, kk + 1);

        let mut n0 = 0.0;
        let mut n1 = 0.0;
        let mut n2 = 0.0;
        let mut n3 = 0.0;

        let t0 = 0.6 - x0 * x0 - y0 * y0 - z0 * z0;
        if t0 > 0.0 {
            let t0_sq = t0 * t0;
            n0 = t0_sq * t0_sq * dot3(GRAD3[gi0], x0, y0, z0);
        }

        let t1 = 0.6 - x1 * x1 - y1 * y1 - z1 * z1;
        if t1 > 0.0 {
            let t1_sq = t1 * t1;
            n1 = t1_sq * t1_sq * dot3(GRAD3[gi1], x1, y1, z1);
        }

        let t2 = 0.6 - x2 * x2 - y2 * y2 - z2 * z2;
        if t2 > 0.0 {
            let t2_sq = t2 * t2;
            n2 = t2_sq * t2_sq * dot3(GRAD3[gi2], x2, y2, z2);
        }

        let t3 = 0.6 - x3 * x3 - y3 * y3 - z3 * z3;
        if t3 > 0.0 {
            let t3_sq = t3 * t3;
            n3 = t3_sq * t3_sq * dot3(GRAD3[gi3], x3, y3, z3);
        }

        32.0 * (n0 + n1 + n2 + n3)
    }

    fn hash3(&self, x: usize, y: usize, z: usize) -> usize {
        usize::from(self.perm_mod12[x + usize::from(self.perm[y + usize::from(self.perm[z])])])
    }
}

#[derive(Debug, Clone)]
struct JavaRandom {
    state: u64,
}

impl JavaRandom {
    fn new(seed: i64) -> Self {
        Self {
            state: (u64::from_ne_bytes(seed.to_ne_bytes()) ^ JAVA_MULTIPLIER) & JAVA_MASK,
        }
    }

    fn next_bits(&mut self, bits: u32) -> u32 {
        self.state = (self
            .state
            .wrapping_mul(JAVA_MULTIPLIER)
            .wrapping_add(JAVA_ADDEND))
            & JAVA_MASK;

        (self.state >> (48 - bits)) as u32
    }

    fn next_i32_bound(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "bound must be positive");

        if (bound & -bound) == bound {
            return (((i64::from(bound)) * i64::from(self.next_bits(31))) >> 31) as i32;
        }

        let bound_i64 = i64::from(bound);

        loop {
            let bits = i64::from(self.next_bits(31));
            let value = bits % bound_i64;
            if bits - value + (bound_i64 - 1) >= 0 {
                return value as i32;
            }
        }
    }

    fn next_f64(&mut self) -> f64 {
        let high = u64::from(self.next_bits(26));
        let low = u64::from(self.next_bits(27));
        ((high << 27) | low) as f64 / ((1u64 << 53) as f64)
    }
}

fn build_permutation(seed: i64) -> [u8; 512] {
    let mut source = [0u8; 256];
    for (index, value) in source.iter_mut().enumerate() {
        *value = u8::try_from(index).expect("index should fit u8");
    }

    let mut random = JavaRandom::new(seed);
    for index in (0..256).rev() {
        let swap_index =
            usize::try_from(random.next_i32_bound((index + 1) as i32)).expect("valid index");
        source.swap(index, swap_index);
    }

    let mut permutations = [0u8; 512];
    for index in 0..512 {
        permutations[index] = source[index & 255];
    }

    permutations
}

fn fade(value: f64) -> f64 {
    value * value * value * (value * (value * 6.0 - 15.0) + 10.0)
}

fn lerp(t: f64, a: f64, b: f64) -> f64 {
    a + t * (b - a)
}

fn grad(hash: u8, x: f64, y: f64, z: f64) -> f64 {
    let h = hash & 15;
    let u = if h < 8 { x } else { y };
    let v = if h < 4 {
        y
    } else if h == 12 || h == 14 {
        x
    } else {
        z
    };

    let a = if h & 1 == 0 { u } else { -u };
    let b = if h & 2 == 0 { v } else { -v };
    a + b
}

fn dot2(gradient: [f64; 3], x: f64, y: f64) -> f64 {
    gradient[0] * x + gradient[1] * y
}

fn dot3(gradient: [f64; 3], x: f64, y: f64, z: f64) -> f64 {
    gradient[0] * x + gradient[1] * y + gradient[2] * z
}
