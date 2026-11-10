use crate::game::*;
use crate::math::*;
use crate::render::*;

const WHITE: [f32; 4] = [0.95, 0.95, 0.95, 1.0];
const SKIN: [f32; 4] = [0.88, 0.68, 0.52, 1.0];

fn hash(i: u32) -> f32 {
    let mut x = i.wrapping_mul(747796405).wrapping_add(2891336453);
    x = ((x >> ((x >> 28) + 4)) ^ x).wrapping_mul(277803737);
    ((x >> 22) ^ x) as f32 / u32::MAX as f32
}

pub fn draw_scene(r: &mut Renderer, g: &Game) {
    let eye = V3::new(g.cam_x, 21.0, -31.0);
    let at = V3::new(g.cam_x, 0.0, -2.0);
    r.set_camera(eye, at);

    unsafe { r.begin() };
    let r: &Renderer = r;

    draw_stadium(r);
    draw_pitch_lines(r);
    draw_goals(r);

    // bola
    let bp = g.ball.pos;
    r.sphere(
        V3::new(bp.x, 0.03, bp.z),
        V3::new(0.55, 0.03, 0.55),
        [0.03, 0.10, 0.03, 1.0],
        MAT_FLAT,
    );
    r.sphere(bp, V3::new(0.6, 0.6, 0.6), [0.97, 0.97, 0.97, 1.0], MAT_SHINY);
    // "gomos" escuros para dar nocao de rotacao
    let roll = bp.x * 1.7 + bp.z * 1.3;
    for k in 0..3 {
        let a = roll + k as f32 * 2.1;
        let off = V3::new(a.cos() * 0.2, (a * 1.3).sin() * 0.2, a.sin() * 0.2);
        r.sphere(bp + off, V3::new(0.22, 0.22, 0.22), [0.08, 0.08, 0.08, 1.0], MAT_DIFFUSE);
    }

    // jogadores
    for (i, p) in g.players.iter().enumerate() {
        let (shirt, socks) = if p.keeper {
            if p.team == 0 {
                ([0.95, 0.80, 0.10, 1.0], [0.95, 0.80, 0.10, 1.0])
            } else {
                ([0.95, 0.45, 0.05, 1.0], [0.95, 0.45, 0.05, 1.0])
            }
        } else if p.team == 0 {
            ([0.85, 0.08, 0.08, 1.0], [0.85, 0.08, 0.08, 1.0])
        } else {
            ([0.10, 0.25, 0.90, 1.0], [0.10, 0.25, 0.90, 1.0])
        };

        if i == g.controlled {
            r.sphere(
                V3::new(p.pos.x, 0.03, p.pos.z),
                V3::new(1.7, 0.03, 1.7),
                [1.0, 0.9, 0.1, 1.0],
                MAT_FLAT,
            );
        }
        r.sphere(
            V3::new(p.pos.x, 0.045, p.pos.z),
            V3::new(1.1, 0.03, 1.1),
            [0.03, 0.10, 0.03, 1.0],
            MAT_FLAT,
        );
        draw_player(r, p, shirt, socks);
    }

    // barra de forca do chute
    if g.charge > 0.0 {
        let p = &g.players[g.controlled];
        let w = 2.0 * g.charge;
        r.cube(
            V3::new(p.pos.x - 1.0 + w * 0.5, 2.5, p.pos.z),
            V3::new(w.max(0.05), 0.14, 0.14),
            [1.0, 0.3 + 0.7 * (1.0 - g.charge), 0.1, 1.0],
            MAT_FLAT,
        );
    }
}

fn draw_player(r: &Renderer, p: &Player, shirt: [f32; 4], socks: [f32; 4]) {
    let yaw = p.face.x.atan2(p.face.z);
    let sp = (p.vel.len() / 7.5).min(1.0);
    let sw = p.phase.sin() * 0.5 * sp;

    let part = |mesh: Mesh, local: V3, size: V3, color: [f32; 4]| {
        let m = mul(
            &mul(&mul(&scale(size), &translate(local)), &rot_y(yaw)),
            &translate(p.pos),
        );
        r.draw(mesh, &m, color, MAT_DIFFUSE);
    };

    // pernas
    part(Mesh::Cube, V3::new(-0.16, 0.38, sw), V3::new(0.2, 0.76, 0.24), socks);
    part(Mesh::Cube, V3::new(0.16, 0.38, -sw), V3::new(0.2, 0.76, 0.24), socks);
    // calcao
    part(Mesh::Cube, V3::new(0.0, 0.84, 0.0), V3::new(0.5, 0.26, 0.3), WHITE);
    // tronco
    part(Mesh::Cube, V3::new(0.0, 1.25, 0.0), V3::new(0.54, 0.62, 0.32), shirt);
    // bracos
    part(Mesh::Cube, V3::new(-0.38, 1.22, -sw), V3::new(0.13, 0.55, 0.14), SKIN);
    part(Mesh::Cube, V3::new(0.38, 1.22, sw), V3::new(0.13, 0.55, 0.14), SKIN);
    // cabeca
    part(Mesh::Sphere, V3::new(0.0, 1.74, 0.0), V3::new(0.34, 0.38, 0.34), SKIN);
    part(Mesh::Sphere, V3::new(0.0, 1.82, -0.03), V3::new(0.35, 0.28, 0.35), [0.12, 0.08, 0.05, 1.0]);
}

fn line(r: &Renderer, cx: f32, cz: f32, lx: f32, lz: f32) {
    r.cube(V3::new(cx, 0.02, cz), V3::new(lx, 0.02, lz), WHITE, MAT_FLAT);
}

fn draw_pitch_lines(r: &Renderer) {
    // gramado e arredores
    r.cube(V3::new(0.0, -0.5, 0.0), V3::new(HALF_L * 2.0 + 6.0, 1.0, HALF_W * 2.0 + 6.0), [0.10, 0.45, 0.12, 1.0], MAT_GRASS);
    r.cube(V3::new(0.0, -0.53, 0.0), V3::new(240.0, 1.0, 200.0), [0.22, 0.22, 0.24, 1.0], MAT_DIFFUSE);

    let t = 0.14;
    // bordas
    line(r, 0.0, HALF_W, HALF_L * 2.0 + t, t);
    line(r, 0.0, -HALF_W, HALF_L * 2.0 + t, t);
    line(r, HALF_L, 0.0, t, HALF_W * 2.0 + t);
    line(r, -HALF_L, 0.0, t, HALF_W * 2.0 + t);
    // meio de campo
    line(r, 0.0, 0.0, t, HALF_W * 2.0);
    // areas
    for s in [-1.0f32, 1.0] {
        let x = s * HALF_L;
        // grande area 12 x 18
        line(r, x - s * 12.0, 0.0, t, 18.0 + t);
        line(r, x - s * 6.0, 9.0, 12.0, t);
        line(r, x - s * 6.0, -9.0, 12.0, t);
        // pequena area 4.5 x 10
        line(r, x - s * 4.5, 0.0, t, 10.0 + t);
        line(r, x - s * 2.25, 5.0, 4.5, t);
        line(r, x - s * 2.25, -5.0, 4.5, t);
        // marca do penalti
        r.cube(V3::new(x - s * 8.0, 0.02, 0.0), V3::new(0.3, 0.02, 0.3), WHITE, MAT_FLAT);
    }
    // circulo central
    let segs = 40;
    let radius = 5.0f32;
    for k in 0..segs {
        let a = k as f32 / segs as f32 * std::f32::consts::TAU;
        let c = V3::new(a.cos() * radius, 0.02, a.sin() * radius);
        let seg_len = std::f32::consts::TAU * radius / segs as f32 + 0.05;
        // tangente = (-sin, cos); yaw que alinha o eixo Z do cubo com a tangente
        let yaw = (-a.sin()).atan2(a.cos());
        let m = mul(
            &mul(&scale(V3::new(t, 0.02, seg_len)), &rot_y(yaw)),
            &translate(c),
        );
        r.draw(Mesh::Cube, &m, WHITE, MAT_FLAT);
    }
    r.cube(V3::new(0.0, 0.02, 0.0), V3::new(0.3, 0.02, 0.3), WHITE, MAT_FLAT);
}

fn draw_goals(r: &Renderer) {
    let post = 0.12;
    for s in [-1.0f32, 1.0] {
        let x = s * HALF_L;
        for z in [-GOAL_HALF, GOAL_HALF] {
            r.cube(V3::new(x, GOAL_H * 0.5, z), V3::new(post, GOAL_H, post), WHITE, MAT_SHINY);
            // trave de tras
            r.cube(V3::new(x + s * 2.2, GOAL_H * 0.5, z), V3::new(post, GOAL_H, post), WHITE, MAT_DIFFUSE);
        }
        r.cube(V3::new(x, GOAL_H, 0.0), V3::new(post, post, GOAL_HALF * 2.0 + post), WHITE, MAT_SHINY);
        // fundo da rede
        r.cube(V3::new(x + s * 2.2, GOAL_H * 0.5, 0.0), V3::new(0.04, GOAL_H, GOAL_HALF * 2.0), [0.8, 0.8, 0.85, 1.0], MAT_DIFFUSE);
        // teto da rede
        r.cube(V3::new(x + s * 1.1, GOAL_H, 0.0), V3::new(2.2, 0.03, GOAL_HALF * 2.0), [0.8, 0.8, 0.85, 1.0], MAT_DIFFUSE);
    }
}

fn draw_stadium(r: &Renderer) {
    let palette = [
        [0.75, 0.15, 0.15, 1.0],
        [0.15, 0.25, 0.75, 1.0],
        [0.85, 0.85, 0.85, 1.0],
        [0.90, 0.75, 0.15, 1.0],
        [0.30, 0.30, 0.34, 1.0],
        [0.15, 0.55, 0.25, 1.0],
    ];

    // placas de publicidade
    let bz = HALF_W + 3.0;
    let bx = HALF_L + 3.5;
    let mut n = 0u32;
    let mut x = -bx;
    while x < bx {
        for s in [-1.0f32, 1.0] {
            let c = if (n / 2) % 2 == 0 { [0.1, 0.2, 0.7, 1.0] } else { [0.9, 0.9, 0.9, 1.0] };
            r.cube(V3::new(x + 2.0, 0.5, s * bz), V3::new(4.0, 1.0, 0.2), c, MAT_DIFFUSE);
        }
        x += 4.0;
        n += 1;
    }
    let mut z = -bz;
    while z < bz {
        for s in [-1.0f32, 1.0] {
            let c = if (n / 2) % 2 == 0 { [0.7, 0.1, 0.1, 1.0] } else { [0.9, 0.9, 0.9, 1.0] };
            r.cube(V3::new(s * bx, 0.5, z + 2.0), V3::new(0.2, 1.0, 4.0), c, MAT_DIFFUSE);
        }
        z += 4.0;
        n += 1;
    }

    // arquibancadas em degraus com torcida colorida
    let mut id = 0u32;
    for tier in 0..4 {
        let h = 1.0 + tier as f32 * 1.3;
        let off = 5.8 + tier as f32 * 2.2;
        // lados (ao longo de X)
        let mut x = -(HALF_L + 8.0);
        while x < HALF_L + 8.0 {
            for s in [-1.0f32, 1.0] {
                id += 1;
                let c = palette[(hash(id) * palette.len() as f32) as usize % palette.len()];
                r.cube(V3::new(x + 1.75, h * 0.5, s * (HALF_W + off)), V3::new(3.5, h, 2.2), c, MAT_DIFFUSE);
            }
            x += 3.5;
        }
        // fundos (ao longo de Z)
        let mut z = -(HALF_W + 3.0);
        while z < HALF_W + 3.0 {
            for s in [-1.0f32, 1.0] {
                id += 1;
                let c = palette[(hash(id) * palette.len() as f32) as usize % palette.len()];
                r.cube(V3::new(s * (HALF_L + off + 1.0), h * 0.5, z + 1.75), V3::new(2.2, h, 3.5), c, MAT_DIFFUSE);
            }
            z += 3.5;
        }
    }
}
