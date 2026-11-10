mod game;
mod math;
mod render;
mod scene;

use std::time::Instant;

use windows::core::{w, Result, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use game::{Game, Input};
use math::V3;
use render::Renderer;

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

fn down(vk: VIRTUAL_KEY) -> bool {
    unsafe { (GetAsyncKeyState(vk.0 as i32) as u16 & 0x8000) != 0 }
}

fn read_input() -> Input {
    let mut d = V3::ZERO;
    // Camera olha para +Z: D = +X (direita), W = +Z (para o fundo)
    if down(VK_W) || down(VK_UP) {
        d.z += 1.0;
    }
    if down(VK_S) || down(VK_DOWN) {
        d.z -= 1.0;
    }
    if down(VK_D) || down(VK_RIGHT) {
        d.x += 1.0;
    }
    if down(VK_A) || down(VK_LEFT) {
        d.x -= 1.0;
    }
    Input {
        dir: d.norm(),
        shoot_held: down(VK_SPACE),
        restart: down(VK_RETURN),
    }
}

fn set_title(hwnd: HWND, text: &str) {
    let mut wide: Vec<u16> = text.encode_utf16().collect();
    wide.push(0);
    unsafe {
        let _ = SetWindowTextW(hwnd, PCWSTR(wide.as_ptr()));
    }
}

unsafe fn run() -> Result<()> {
    let inst: HINSTANCE = GetModuleHandleW(None)?.into();
    let class = w!("FutebolD3D10");

    let wc = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(wndproc),
        hInstance: inst,
        hCursor: LoadCursorW(None, IDC_ARROW)?,
        lpszClassName: class,
        ..Default::default()
    };
    RegisterClassW(&wc);

    let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
    let mut rc = RECT { left: 0, top: 0, right: WIDTH as i32, bottom: HEIGHT as i32 };
    AdjustWindowRect(&mut rc, style, false)?;

    let hwnd = CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        class,
        w!("Futebol Rust + Direct3D 10"),
        style | WS_VISIBLE,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        rc.right - rc.left,
        rc.bottom - rc.top,
        None,
        None,
        inst,
        None,
    )?;

    let mut renderer = Renderer::new(hwnd, WIDTH, HEIGHT)?;
    let mut game = Game::new();

    let mut last = Instant::now();
    let mut title_timer = 0.0f32;
    let mut msg = MSG::default();

    loop {
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            if msg.message == WM_QUIT {
                return Ok(());
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        if down(VK_ESCAPE) {
            return Ok(());
        }

        let now = Instant::now();
        let dt = (now - last).as_secs_f32().min(0.05);
        last = now;

        game.update(dt, &read_input());

        scene::draw_scene(&mut renderer, &game);
        renderer.end()?;

        title_timer += dt;
        if title_timer > 0.25 {
            title_timer = 0.0;
            let secs = game.time_left.ceil() as u32;
            let status = if game.over {
                "  |  FIM DE JOGO - Enter reinicia"
            } else if game.pause > 0.0 {
                "  |  GOOOOL!"
            } else {
                ""
            };
            set_title(
                hwnd,
                &format!(
                    "Futebol Rust + D3D10  |  Vermelho {} x {} Azul  |  {:02}:{:02}{}",
                    game.score[0],
                    game.score[1],
                    secs / 60,
                    secs % 60,
                    status
                ),
            );
        }
    }
}

fn main() {
    if let Err(e) = unsafe { run() } {
        eprintln!("Erro: {e}");
    }
}
