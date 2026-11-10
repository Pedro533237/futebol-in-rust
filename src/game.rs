use crate::math::V3;

pub const HALF_L: f32 = 30.0; // metade do comprimento do campo
pub const HALF_W: f32 = 19.0; // metade da largura
pub const GOAL_HALF: f32 = 3.66; // metade da boca do gol
pub const GOAL_H: f32 = 2.44; // altura do gol
pub const BALL_R: f32 = 0.3;

const SPEED: f32 = 7.5;
const AI_SPEED: f32 = 6.3;
const MATCH_TIME: f32 = 180.0;
const PER_TEAM: usize = 5;

pub struct Input {
    pub dir: V3,
    pub shoot_held: bool,
    pub restart: bool,
}

#[derive(Clone)]
pub struct Player {
    pub pos: V3,
    pub vel: V3,
    pub face: V3,
    pub team: usize,
    pub home: V3,
    pub keeper: bool,
    pub cd: f32,
    pub phase: f32,
}

pub struct Ball {
    pub pos: V3,
    pub vel: V3,
}

pub struct Game {
    pub players: Vec<Player>,
    pub ball: Ball,
    pub score: [u32; 2],
    pub time_left: f32,
    pub controlled: usize,
    pub charge: f32,
    pub pause: f32,
    pub over: bool,
    pub cam_x: f32,
    shoot_prev: bool,
}

fn clampf(v: f32, a: f32, b: f32) -> f32 {
    v.max(a).min(b)
}

impl Game {
    pub fn new() -> Game {
        let formation = [
            (-28.5f32, 0.0f32, true),
            (-19.0, -8.0, false),
            (-19.0, 8.0, false),
            (-9.0, -5.0, false),
            (-9.0, 5.0, false),
        ];
        let mut players = Vec::new();
        for team in 0..2usize {
            let sx = if team == 0 { 1.0 } else { -1.0 };
            for &(x, z, keeper) in &formation {
                let home = V3::new(x * sx, 0.0, z);
                players.push(Player {
                    pos: home,
                    vel: V3::ZERO,
                    face: V3::new(sx, 0.0, 0.0),
                    team,
                    home,
                    keeper,
                    cd: 0.0,
                    phase: 0.0,
                });
            }
        }
        Game {
            players,
            ball: Ball { pos: V3::new(0.0, BALL_R, 0.0), vel: V3::ZERO },
            score: [0, 0],
            time_left: MATCH_TIME,
            controlled: 3,
            charge: 0.0,
            pause: 0.0,
            over: false,
            cam_x: 0.0,
            shoot_prev: false,
        }
    }

    fn reset_kickoff(&mut self) {
        for p in self.players.iter_mut() {
            p.pos = p.home;
            p.vel = V3::ZERO;
            p.cd = 0.0;
        }
        self.ball = Ball { pos: V3::new(0.0, BALL_R, 0.0), vel: V3::ZERO };
        self.charge = 0.0;
    }

    fn closest(&self, team: usize, exclude: Option<usize>) -> usize {
        let mut best = team * PER_TEAM + 1;
        let mut bd = f32::MAX;
        for (i, p) in self.players.iter().enumerate() {
            if p.team != team || p.keeper || Some(i) == exclude {
                continue;
            }
            let d = (p.pos - self.ball.pos).flat().len();
            if d < bd {
                bd = d;
                best = i;
            }
        }
        best
    }

    fn kick(&mut self, dir: V3, power: f32, loft: f32) {
        let d = dir.flat().norm();
        self.ball.vel = V3::new(d.x * power, power * loft, d.z * power);
    }

    pub fn update(&mut self, dt: f32, inp: &Input) {
        if self.over {
            if inp.restart {
                *self = Game::new();
            }
            return;
        }

        // pausa apos gol: bola continua na rede, jogadores parados
        if self.pause > 0.0 {
            self.pause -= dt;
            self.ball_physics(dt, true);
            self.update_camera(dt);
            if self.pause <= 0.0 {
                self.reset_kickoff();
            }
            return;
        }

        self.time_left -= dt;
        if self.time_left <= 0.0 {
            self.time_left = 0.0;
            self.over = true;
            return;
        }

        // jogador controlado = companheiro mais perto da bola
        if self.charge <= 0.0 {
            self.controlled = self.closest(0, None);
        }
        let chaser = [self.closest(0, Some(self.controlled)), self.closest(1, None)];

        // ---- chute do usuario (segura espaco = carrega, solta = chuta) ----
        if inp.shoot_held {
            self.charge = (self.charge + dt / 0.9).min(1.0);
        }
        if !inp.shoot_held && self.shoot_prev {
            let p = self.players[self.controlled].clone();
            let to = self.ball.pos - p.pos;
            if to.flat().len() < 1.5 && self.ball.pos.y < 1.2 {
                let dir = if inp.dir.len() > 0.1 { inp.dir.norm() } else { p.face };
                let power = 9.0 + self.charge * 17.0;
                self.kick(dir, power, 0.12 + self.charge * 0.18);
                self.players[self.controlled].cd = 0.3;
            }
            self.charge = 0.0;
        }
        self.shoot_prev = inp.shoot_held;

        // ---- jogadores ----
        let ball = self.ball.pos;
        let bvel = self.ball.vel;
        let n = self.players.len();
        for i in 0..n {
            let p = self.players[i].clone();
            let ad = if p.team == 0 { 1.0 } else { -1.0 };
            let goal = V3::new(ad * HALF_L, 0.0, 0.0);
            let own_goal_x = -ad * HALF_L;
            let mut desired = V3::ZERO;
            let mut ai_shot: Option<(V3, f32, f32)> = None;

            if i == self.controlled {
                desired = inp.dir * SPEED;
            } else if p.keeper {
                let in_area = (ball.x - own_goal_x).abs() < 8.0 && ball.z.abs() < 9.0;
                let target = if in_area && (ball.x - own_goal_x).abs() < 6.0 {
                    V3::new(ball.x, 0.0, ball.z)
                } else {
                    V3::new(-ad * (HALF_L - 1.5), 0.0, clampf(ball.z * 0.4, -3.0, 3.0))
                };
                desired = arrive(p.pos, target, AI_SPEED);
                if (ball - p.pos).flat().len() < 1.3 && p.cd <= 0.0 {
                    let dir = V3::new(ad, 0.0, clampf(ball.z, -1.0, 1.0) * -0.4);
                    ai_shot = Some((dir, 17.0, 0.35));
                }
            } else if chaser[p.team] == i {
                let to_goal = (goal - ball).flat().norm();
                let behind = (ball - p.pos).flat().dot(to_goal) > 0.0;
                let pred = ball + bvel.flat() * 0.25;
                let target = if behind {
                    pred + to_goal * 2.5
                } else {
                    pred - to_goal * 1.6
                };
                desired = arrive(p.pos, target, AI_SPEED);
                let dball = (ball - p.pos).flat().len();
                let dgoal = (goal - ball).flat().len();
                if dball < 1.1 && p.cd <= 0.0 && dgoal < 17.0 && behind {
                    let opp_keeper = (1 - p.team) * PER_TEAM;
                    let kz = self.players[opp_keeper].pos.z;
                    let aim = V3::new(goal.x, 0.0, if kz > 0.0 { -2.2 } else { 2.2 });
                    ai_shot = Some(((aim - ball).norm(), (dgoal * 0.9).max(12.0).min(22.0), 0.15));
                }
            } else {
                let tx = clampf(p.home.x + ball.x * 0.45, -HALF_L + 3.0, HALF_L - 3.0);
                let tz = p.home.z + ball.z * 0.35;
                desired = arrive(p.pos, V3::new(tx, 0.0, tz), AI_SPEED * 0.85);
            }

            if let Some((dir, power, loft)) = ai_shot {
                self.kick(dir, power, loft);
                self.players[i].cd = 0.6;
            }

            let pl = &mut self.players[i];
            let k = 1.0 - (-10.0 * dt).exp();
            pl.vel = pl.vel + (desired - pl.vel) * k;
            pl.pos = pl.pos + pl.vel * dt;
            pl.pos.x = clampf(pl.pos.x, -HALF_L - 1.0, HALF_L + 1.0);
            pl.pos.z = clampf(pl.pos.z, -HALF_W - 1.0, HALF_W + 1.0);
            let sp = pl.vel.len();
            if sp > 0.5 {
                pl.face = (pl.face + pl.vel.norm() * (dt * 14.0)).norm();
            }
            pl.phase += sp * dt * 1.5;
            pl.cd = (pl.cd - dt).max(0.0);
        }

        // ---- separacao entre jogadores ----
        for a in 0..n {
            for b in (a + 1)..n {
                let d = self.players[b].pos - self.players[a].pos;
                let dist = d.flat().len();
                if dist < 0.8 && dist > 1e-4 {
                    let push = d.flat().norm() * ((0.8 - dist) * 0.5);
                    self.players[a].pos = self.players[a].pos - push;
                    self.players[b].pos = self.players[b].pos + push;
                }
            }
        }

        // ---- colisao jogador x bola (gera o drible naturalmente) ----
        for i in 0..n {
            let p = &self.players[i];
            let d = (self.ball.pos - p.pos).flat();
            let dist = d.len();
            if dist < 0.8 && self.ball.pos.y < 1.5 && dist > 1e-4 {
                let nrm = d.norm();
                let rel = (self.ball.vel - p.vel).flat().dot(nrm);
                if rel < 0.0 {
                    let e = 0.15;
                    self.ball.vel = self.ball.vel - nrm * (rel * (1.0 + e));
                }
                self.ball.pos = V3::new(
                    p.pos.x + nrm.x * 0.8,
                    self.ball.pos.y,
                    p.pos.z + nrm.z * 0.8,
                );
            }
        }

        // ---- bola e gols ----
        if let Some(team) = self.ball_physics(dt, false) {
            self.score[team] += 1;
            self.pause = 2.5;
        }
        self.update_camera(dt);
    }

    fn update_camera(&mut self, dt: f32) {
        let target = clampf(self.ball.pos.x * 0.8, -18.0, 18.0);
        self.cam_x += (target - self.cam_x) * (1.0 - (-3.0 * dt).exp());
    }

    /// Retorna Some(time) se foi gol. `net` = bola dentro da rede (pos-gol).
    fn ball_physics(&mut self, dt: f32, net: bool) -> Option<usize> {
        let b = &mut self.ball;
        b.vel.y -= 14.0 * dt;
        b.pos = b.pos + b.vel * dt;

        if b.pos.y < BALL_R {
            b.pos.y = BALL_R;
            if b.vel.y < 0.0 {
                b.vel.y = -b.vel.y * 0.55;
                if b.vel.y < 1.2 {
                    b.vel.y = 0.0;
                }
            }
            let f = (-1.1 * dt).exp();
            b.vel.x *= f;
            b.vel.z *= f;
        } else {
            let f = (-0.15 * dt).exp();
            b.vel.x *= f;
            b.vel.z *= f;
        }

        let mut scored = None;

        if net {
            let lim = HALF_L + 2.2;
            if b.pos.x > lim {
                b.pos.x = lim;
                b.vel.x *= -0.2;
            }
            if b.pos.x < -lim {
                b.pos.x = -lim;
                b.vel.x *= -0.2;
            }
            let zl = GOAL_HALF - 0.2;
            if b.pos.z.abs() > zl {
                b.pos.z = clampf(b.pos.z, -zl, zl);
                b.vel.z *= -0.3;
            }
            if b.pos.y > GOAL_H - 0.2 {
                b.pos.y = GOAL_H - 0.2;
                b.vel.y = -b.vel.y.abs() * 0.3;
            }
        } else {
            let in_mouth = b.pos.z.abs() < GOAL_HALF - 0.1 && b.pos.y + BALL_R < GOAL_H;
            if b.pos.x > HALF_L {
                if in_mouth {
                    scored = Some(0);
                } else {
                    b.pos.x = HALF_L;
                    if b.vel.x > 0.0 {
                        b.vel.x *= -0.5;
                    }
                }
            } else if b.pos.x < -HALF_L {
                if in_mouth {
                    scored = Some(1);
                } else {
                    b.pos.x = -HALF_L;
                    if b.vel.x < 0.0 {
                        b.vel.x *= -0.5;
                    }
                }
            }
            let zl = HALF_W + 1.5;
            if b.pos.z.abs() > zl {
                b.pos.z = clampf(b.pos.z, -zl, zl);
                b.vel.z *= -0.5;
            }
        }
        scored
    }
}

/// Velocidade desejada para chegar em `target`, desacelerando perto dele.
fn arrive(pos: V3, target: V3, speed: f32) -> V3 {
    let d = (target - pos).flat();
    let dist = d.len();
    if dist < 0.05 {
        return V3::ZERO;
    }
    d.norm() * (speed * (dist / 1.5).min(1.0))
}
