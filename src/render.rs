use std::ffi::c_void;
use std::mem::size_of;

use windows::core::{s, Result, PCSTR};
use windows::Win32::Foundation::{FALSE, HMODULE, HWND, TRUE};
use windows::Win32::Graphics::Direct3D::Fxc::D3DCompile;
use windows::Win32::Graphics::Direct3D::{ID3DBlob, D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST};
use windows::Win32::Graphics::Direct3D10::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;

use crate::math::*;

const MSAA: u32 = 4;

// Materiais: params = [especular, gramado, sem_iluminacao, 0]
pub const MAT_DIFFUSE: [f32; 4] = [0.0, 0.0, 0.0, 0.0];
pub const MAT_SHINY: [f32; 4] = [0.7, 0.0, 0.0, 0.0];
pub const MAT_GRASS: [f32; 4] = [0.0, 1.0, 0.0, 0.0];
pub const MAT_FLAT: [f32; 4] = [0.0, 0.0, 1.0, 0.0];

const SHADER: &str = r#"
cbuffer PerObject : register(b0)
{
    row_major float4x4 World;
    row_major float4x4 ViewProj;
    float4 Color;
    float4 LightDir;
    float4 Eye;
    float4 Params; // x = especular, y = gramado, z = flat
};

struct VSIn  { float3 pos : POSITION; float3 nrm : NORMAL; };
struct PSIn  { float4 pos : SV_POSITION; float3 nrm : NORMAL; float3 wpos : TEXCOORD0; };

PSIn VS(VSIn i)
{
    PSIn o;
    float4 w = mul(float4(i.pos, 1.0), World);
    o.pos  = mul(w, ViewProj);
    o.nrm  = mul(i.nrm, (float3x3)World);
    o.wpos = w.xyz;
    return o;
}

float hash(float2 p)
{
    return frac(sin(dot(p, float2(12.9898, 78.233))) * 43758.5453);
}

float4 PS(PSIn i) : SV_TARGET
{
    float3 col = Color.rgb;

    if (Params.y > 0.5)
    {
        // listras de corte + ruido para parecer gramado
        float s = step(0.5, frac(i.wpos.x / 6.0));
        col *= 0.86 + 0.14 * s;
        col *= 0.92 + 0.16 * hash(floor(i.wpos.xz * 7.0));
    }

    if (Params.z > 0.5)
        return float4(col, 1.0);

    float3 n = normalize(i.nrm);
    float ndl = saturate(dot(n, -LightDir.xyz));
    float hemi = n.y * 0.5 + 0.5;
    float3 amb = lerp(float3(0.16, 0.15, 0.13), float3(0.30, 0.36, 0.46), hemi);

    float3 v = normalize(Eye.xyz - i.wpos);
    float3 h = normalize(v - LightDir.xyz);
    float spec = pow(saturate(dot(n, h)), 48.0) * Params.x;

    float3 c = col * (amb + ndl * float3(1.0, 0.96, 0.88)) + spec;
    return float4(saturate(c), 1.0);
}
"#;

#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    p: [f32; 3],
    n: [f32; 3],
}

#[repr(C)]
struct CBuf {
    world: Mat4,
    vp: Mat4,
    color: [f32; 4],
    light: [f32; 4],
    eye: [f32; 4],
    params: [f32; 4],
}

#[derive(Clone, Copy)]
pub enum Mesh {
    Cube,
    Sphere,
}

pub struct Renderer {
    device: ID3D10Device,
    swap: IDXGISwapChain,
    rtv: ID3D10RenderTargetView,
    dsv: ID3D10DepthStencilView,
    vs: ID3D10VertexShader,
    ps: ID3D10PixelShader,
    layout: ID3D10InputLayout,
    vb: ID3D10Buffer,
    ib: ID3D10Buffer,
    cbuf: ID3D10Buffer,
    rs: ID3D10RasterizerState,
    w: u32,
    h: u32,
    vp: Mat4,
    eye: [f32; 4],
    // (indice inicial, qtd indices, vertice base)
    cube_range: (u32, u32, i32),
    sphere_range: (u32, u32, i32),
}

unsafe fn compile(src: &str, entry: PCSTR, target: PCSTR) -> Result<ID3DBlob> {
    let mut code: Option<ID3DBlob> = None;
    let mut err: Option<ID3DBlob> = None;
    let r = D3DCompile(
        src.as_ptr() as *const c_void,
        src.len(),
        PCSTR::null(),
        None,
        None,
        entry,
        target,
        0,
        0,
        &mut code,
        Some(&mut err),
    );
    if let Err(e) = r {
        if let Some(eb) = err {
            let msg = std::slice::from_raw_parts(
                eb.GetBufferPointer() as *const u8,
                eb.GetBufferSize(),
            );
            eprintln!("Erro de shader:\n{}", String::from_utf8_lossy(msg));
        }
        return Err(e);
    }
    Ok(code.unwrap())
}

fn build_cube() -> (Vec<Vertex>, Vec<u32>) {
    let faces = [
        (V3::new(1.0, 0.0, 0.0), V3::new(0.0, 1.0, 0.0), V3::new(0.0, 0.0, 1.0)),
        (V3::new(-1.0, 0.0, 0.0), V3::new(0.0, 1.0, 0.0), V3::new(0.0, 0.0, 1.0)),
        (V3::new(0.0, 1.0, 0.0), V3::new(1.0, 0.0, 0.0), V3::new(0.0, 0.0, 1.0)),
        (V3::new(0.0, -1.0, 0.0), V3::new(1.0, 0.0, 0.0), V3::new(0.0, 0.0, 1.0)),
        (V3::new(0.0, 0.0, 1.0), V3::new(1.0, 0.0, 0.0), V3::new(0.0, 1.0, 0.0)),
        (V3::new(0.0, 0.0, -1.0), V3::new(1.0, 0.0, 0.0), V3::new(0.0, 1.0, 0.0)),
    ];
    let mut verts = Vec::new();
    let mut idx = Vec::new();
    for (n, u, v) in faces {
        let base = verts.len() as u32;
        for (su, sv) in [(-1.0f32, -1.0f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            let p = n * 0.5 + u * (0.5 * su) + v * (0.5 * sv);
            verts.push(Vertex { p: [p.x, p.y, p.z], n: [n.x, n.y, n.z] });
        }
        idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (verts, idx)
}

/// Esfera de diametro 1.
fn build_sphere() -> (Vec<Vertex>, Vec<u32>) {
    let stacks = 14u32;
    let slices = 24u32;
    let mut verts = Vec::new();
    let mut idx = Vec::new();
    for i in 0..=stacks {
        let phi = std::f32::consts::PI * i as f32 / stacks as f32;
        for j in 0..=slices {
            let th = 2.0 * std::f32::consts::PI * j as f32 / slices as f32;
            let n = [phi.sin() * th.cos(), phi.cos(), phi.sin() * th.sin()];
            verts.push(Vertex { p: [n[0] * 0.5, n[1] * 0.5, n[2] * 0.5], n });
        }
    }
    for i in 0..stacks {
        for j in 0..slices {
            let a = i * (slices + 1) + j;
            let b = a + slices + 1;
            idx.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
        }
    }
    (verts, idx)
}

impl Renderer {
    pub unsafe fn new(hwnd: HWND, w: u32, h: u32) -> Result<Renderer> {
        // ---- dispositivo + swap chain (D3D10) ----
        let sd = DXGI_SWAP_CHAIN_DESC {
            BufferDesc: DXGI_MODE_DESC {
                Width: w,
                Height: h,
                RefreshRate: DXGI_RATIONAL { Numerator: 60, Denominator: 1 },
                Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                ..Default::default()
            },
            SampleDesc: DXGI_SAMPLE_DESC { Count: MSAA, Quality: 0 },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 1,
            OutputWindow: hwnd,
            Windowed: TRUE,
            SwapEffect: DXGI_SWAP_EFFECT_DISCARD,
            Flags: 0,
        };

        let mut swap: Option<IDXGISwapChain> = None;
        let mut device: Option<ID3D10Device> = None;
        D3D10CreateDeviceAndSwapChain(
            None,
            D3D10_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            0,
            D3D10_SDK_VERSION,
            Some(&sd),
            Some(&mut swap),
            Some(&mut device),
        )?;
        let swap = swap.unwrap();
        let device = device.unwrap();

        // ---- render target ----
        let back: ID3D10Texture2D = swap.GetBuffer(0)?;
        let mut rtv: Option<ID3D10RenderTargetView> = None;
        device.CreateRenderTargetView(&back, None, Some(&mut rtv))?;
        let rtv = rtv.unwrap();

        // ---- depth buffer ----
        let dd = D3D10_TEXTURE2D_DESC {
            Width: w,
            Height: h,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_D32_FLOAT,
            SampleDesc: DXGI_SAMPLE_DESC { Count: MSAA, Quality: 0 },
            Usage: D3D10_USAGE_DEFAULT,
            BindFlags: D3D10_BIND_DEPTH_STENCIL.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags: 0,
        };
        let depth: ID3D10Texture2D = device.CreateTexture2D(&dd, None)?;
        let mut dsv: Option<ID3D10DepthStencilView> = None;
        device.CreateDepthStencilView(&depth, None, Some(&mut dsv))?;
        let dsv = dsv.unwrap();

        // ---- shaders ----
        let vs_blob = compile(SHADER, s!("VS"), s!("vs_4_0"))?;
        let ps_blob = compile(SHADER, s!("PS"), s!("ps_4_0"))?;
        let vs_bytes = std::slice::from_raw_parts(
            vs_blob.GetBufferPointer() as *const u8,
            vs_blob.GetBufferSize(),
        );
        let ps_bytes = std::slice::from_raw_parts(
            ps_blob.GetBufferPointer() as *const u8,
            ps_blob.GetBufferSize(),
        );

        let mut vs: Option<ID3D10VertexShader> = None;
        device.CreateVertexShader(vs_bytes, Some(&mut vs))?;
        let mut ps: Option<ID3D10PixelShader> = None;
        device.CreatePixelShader(ps_bytes, Some(&mut ps))?;

        let elems = [
            D3D10_INPUT_ELEMENT_DESC {
                SemanticName: s!("POSITION"),
                SemanticIndex: 0,
                Format: DXGI_FORMAT_R32G32B32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: 0,
                InputSlotClass: D3D10_INPUT_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
            D3D10_INPUT_ELEMENT_DESC {
                SemanticName: s!("NORMAL"),
                SemanticIndex: 0,
                Format: DXGI_FORMAT_R32G32B32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: 12,
                InputSlotClass: D3D10_INPUT_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
        ];
        let mut layout: Option<ID3D10InputLayout> = None;
        device.CreateInputLayout(&elems, vs_bytes, Some(&mut layout))?;

        // ---- malhas ----
        let (cv, ci) = build_cube();
        let (sv, si) = build_sphere();
        let cube_range = (0u32, ci.len() as u32, 0i32);
        let sphere_range = (ci.len() as u32, si.len() as u32, cv.len() as i32);
        let mut verts = cv;
        verts.extend_from_slice(&sv);
        let mut idx = ci;
        idx.extend_from_slice(&si);

        let vb_desc = D3D10_BUFFER_DESC {
            ByteWidth: (verts.len() * size_of::<Vertex>()) as u32,
            Usage: D3D10_USAGE_DEFAULT,
            BindFlags: D3D10_BIND_VERTEX_BUFFER.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags: 0,
        };
        let vb_data = D3D10_SUBRESOURCE_DATA {
            pSysMem: verts.as_ptr() as *const c_void,
            SysMemPitch: 0,
            SysMemSlicePitch: 0,
        };
        let mut vb: Option<ID3D10Buffer> = None;
        device.CreateBuffer(&vb_desc, Some(&vb_data), Some(&mut vb))?;

        let ib_desc = D3D10_BUFFER_DESC {
            ByteWidth: (idx.len() * size_of::<u32>()) as u32,
            Usage: D3D10_USAGE_DEFAULT,
            BindFlags: D3D10_BIND_INDEX_BUFFER.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags: 0,
        };
        let ib_data = D3D10_SUBRESOURCE_DATA {
            pSysMem: idx.as_ptr() as *const c_void,
            SysMemPitch: 0,
            SysMemSlicePitch: 0,
        };
        let mut ib: Option<ID3D10Buffer> = None;
        device.CreateBuffer(&ib_desc, Some(&ib_data), Some(&mut ib))?;

        let cb_desc = D3D10_BUFFER_DESC {
            ByteWidth: size_of::<CBuf>() as u32,
            Usage: D3D10_USAGE_DEFAULT,
            BindFlags: D3D10_BIND_CONSTANT_BUFFER.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags: 0,
        };
        let mut cbuf: Option<ID3D10Buffer> = None;
        device.CreateBuffer(&cb_desc, None, Some(&mut cbuf))?;

        // ---- rasterizer (sem culling: nao depende da ordem dos vertices) ----
        let rd = D3D10_RASTERIZER_DESC {
            FillMode: D3D10_FILL_SOLID,
            CullMode: D3D10_CULL_NONE,
            FrontCounterClockwise: FALSE,
            DepthBias: 0,
            DepthBiasClamp: 0.0,
            SlopeScaledDepthBias: 0.0,
            DepthClipEnable: TRUE,
            ScissorEnable: FALSE,
            MultisampleEnable: TRUE,
            AntialiasedLineEnable: FALSE,
        };
        let mut rs: Option<ID3D10RasterizerState> = None;
        device.CreateRasterizerState(&rd, Some(&mut rs))?;

        Ok(Renderer {
            device,
            swap,
            rtv,
            dsv,
            vs: vs.unwrap(),
            ps: ps.unwrap(),
            layout: layout.unwrap(),
            vb: vb.unwrap(),
            ib: ib.unwrap(),
            cbuf: cbuf.unwrap(),
            rs: rs.unwrap(),
            w,
            h,
            vp: identity(),
            eye: [0.0; 4],
            cube_range,
            sphere_range,
        })
    }

    pub fn set_camera(&mut self, eye: V3, at: V3) {
        let view = look_at_lh(eye, at, V3::new(0.0, 1.0, 0.0));
        let proj = perspective_lh(
            45.0f32.to_radians(),
            self.w as f32 / self.h as f32,
            1.0,
            300.0,
        );
        self.vp = mul(&view, &proj);
        self.eye = [eye.x, eye.y, eye.z, 1.0];
    }

    pub unsafe fn begin(&self) {
        let d = &self.device;
        d.OMSetRenderTargets(Some(&[Some(self.rtv.clone())]), &self.dsv);
        let vp = D3D10_VIEWPORT {
            TopLeftX: 0,
            TopLeftY: 0,
            Width: self.w,
            Height: self.h,
            MinDepth: 0.0,
            MaxDepth: 1.0,
        };
        d.RSSetViewports(Some(&[vp]));
        d.RSSetState(&self.rs);

        let sky = [0.45f32, 0.66, 0.92, 1.0];
        d.ClearRenderTargetView(&self.rtv, &sky);
        d.ClearDepthStencilView(&self.dsv, D3D10_CLEAR_DEPTH.0 as u32, 1.0, 0);

        d.IASetInputLayout(&self.layout);
        d.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
        let stride = size_of::<Vertex>() as u32;
        let offset = 0u32;
        let vb = Some(self.vb.clone());
        d.IASetVertexBuffers(
            0,
            1,
            Some(&vb as *const Option<ID3D10Buffer>),
            Some(&stride as *const u32),
            Some(&offset as *const u32),
        );
        d.IASetIndexBuffer(&self.ib, DXGI_FORMAT_R32_UINT, 0);
        d.VSSetShader(&self.vs);
        d.PSSetShader(&self.ps);
        d.VSSetConstantBuffers(0, Some(&[Some(self.cbuf.clone())]));
        d.PSSetConstantBuffers(0, Some(&[Some(self.cbuf.clone())]));
    }

    pub unsafe fn end(&self) -> Result<()> {
        self.swap.Present(1, DXGI_PRESENT(0)).ok()
    }

    pub fn draw(&self, mesh: Mesh, world: &Mat4, color: [f32; 4], params: [f32; 4]) {
        let l = V3::new(-0.35, -0.85, 0.4).norm();
        let cb = CBuf {
            world: *world,
            vp: self.vp,
            color,
            light: [l.x, l.y, l.z, 0.0],
            eye: self.eye,
            params,
        };
        let (start, count, base) = match mesh {
            Mesh::Cube => self.cube_range,
            Mesh::Sphere => self.sphere_range,
        };
        unsafe {
            self.device.UpdateSubresource(
                &self.cbuf,
                0,
                None,
                &cb as *const CBuf as *const c_void,
                0,
                0,
            );
            self.device.DrawIndexed(count, start, base);
        }
    }

    /// Caixa centrada em `c` com tamanho `size`.
    pub fn cube(&self, c: V3, size: V3, color: [f32; 4], mat: [f32; 4]) {
        let m = mul(&scale(size), &translate(c));
        self.draw(Mesh::Cube, &m, color, mat);
    }

    /// Esfera/elipsoide centrada em `c` com diametros `size`.
    pub fn sphere(&self, c: V3, size: V3, color: [f32; 4], mat: [f32; 4]) {
        let m = mul(&scale(size), &translate(c));
        self.draw(Mesh::Sphere, &m, color, mat);
    }
}