use std::ops::{Add, Mul, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl V3 {
    pub const ZERO: V3 = V3 { x: 0.0, y: 0.0, z: 0.0 };

    pub const fn new(x: f32, y: f32, z: f32) -> V3 {
        V3 { x, y, z }
    }
    pub fn dot(self, o: V3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    pub fn cross(self, o: V3) -> V3 {
        V3::new(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }
    pub fn len(self) -> f32 {
        self.dot(self).sqrt()
    }
    pub fn norm(self) -> V3 {
        let l = self.len();
        if l < 1e-5 {
            V3::ZERO
        } else {
            self * (1.0 / l)
        }
    }
    /// Remove a componente vertical.
    pub fn flat(self) -> V3 {
        V3::new(self.x, 0.0, self.z)
    }
}

impl Add for V3 {
    type Output = V3;
    fn add(self, o: V3) -> V3 {
        V3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}
impl Sub for V3 {
    type Output = V3;
    fn sub(self, o: V3) -> V3 {
        V3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}
impl Mul<f32> for V3 {
    type Output = V3;
    fn mul(self, s: f32) -> V3 {
        V3::new(self.x * s, self.y * s, self.z * s)
    }
}

/// Matrizes row-major, convencao de vetor-linha (v * M), como no D3DX.
pub type Mat4 = [[f32; 4]; 4];

pub fn identity() -> Mat4 {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// Aplica `a` e depois `b`.
pub fn mul(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut r = [[0.0f32; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            for k in 0..4 {
                r[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    r
}

pub fn scale(s: V3) -> Mat4 {
    let mut m = identity();
    m[0][0] = s.x;
    m[1][1] = s.y;
    m[2][2] = s.z;
    m
}

pub fn translate(t: V3) -> Mat4 {
    let mut m = identity();
    m[3][0] = t.x;
    m[3][1] = t.y;
    m[3][2] = t.z;
    m
}

/// Rotacao em torno de Y (mundo left-handed): +Z gira em direcao a +X.
pub fn rot_y(a: f32) -> Mat4 {
    let (s, c) = a.sin_cos();
    let mut m = identity();
    m[0][0] = c;
    m[0][2] = -s;
    m[2][0] = s;
    m[2][2] = c;
    m
}

pub fn perspective_lh(fovy: f32, aspect: f32, zn: f32, zf: f32) -> Mat4 {
    let h = 1.0 / (fovy * 0.5).tan();
    let w = h / aspect;
    let q = zf / (zf - zn);
    [
        [w, 0.0, 0.0, 0.0],
        [0.0, h, 0.0, 0.0],
        [0.0, 0.0, q, 1.0],
        [0.0, 0.0, -zn * q, 0.0],
    ]
}

pub fn look_at_lh(eye: V3, at: V3, up: V3) -> Mat4 {
    let z = (at - eye).norm();
    let x = up.cross(z).norm();
    let y = z.cross(x);
    [
        [x.x, y.x, z.x, 0.0],
        [x.y, y.y, z.y, 0.0],
        [x.z, y.z, z.z, 0.0],
        [-x.dot(eye), -y.dot(eye), -z.dot(eye), 1.0],
    ]
}
