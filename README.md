# Futebol - Rust + Direct3D 10 (Windows)

Jogo de futebol 5x5 feito do zero: janela Win32, Direct3D 10 via crate `windows`,
shaders HLSL (vs_4_0 / ps_4_0), MSAA 4x e iluminação por pixel.

## Como rodar

1. Instale o Rust para Windows (https://rustup.rs) com o toolchain **MSVC**
   (precisa das "Build Tools do Visual Studio" com C++).
2. Na pasta do projeto:

```
cargo run --release
```

## Controles

| Tecla | Ação |
|---|---|
| W A S D / setas | Mover o jogador (o mais perto da bola é selecionado automaticamente) |
| Espaço (segurar e soltar) | Chutar - quanto mais segura, mais forte e mais alto |
| Enter | Reiniciar após o fim da partida |
| Esc | Sair |

Você joga de **vermelho** (ataca para a direita). O placar e o tempo aparecem na barra de título.
A partida dura 3 minutos.

## Estrutura

- `src/main.rs`  - janela Win32, entrada de teclado, loop principal
- `src/render.rs` - dispositivo D3D10, swap chain, shaders, malhas (cubo/esfera)
- `src/game.rs`  - física da bola, regras, gols, IA dos jogadores
- `src/scene.rs` - desenho do campo, linhas, traves, estádio, jogadores
- `src/math.rs`  - vetores e matrizes

## Próximos passos sugeridos

- Texto/HUD com placar na tela (DirectWrite não existe em D3D10 puro; dá para usar D2D ou uma fonte em textura)
- Modelos 3D e animações carregados de arquivos (glTF)
- Sombras reais (shadow map), texturas, torcida animada
- Regras de lateral, escanteio e falta
- Passe com tecla dedicada, troca manual de jogador, dois jogadores locais
