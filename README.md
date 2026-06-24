# Minha Princesa Animes

App desktop para **baixar, organizar e assistir** animes — biblioteca pessoal com fila de downloads, catálogo e player integrado.

Repositório: [github.com/Lascone/Minha-Princesa-Animes](https://github.com/Lascone/Minha-Princesa-Animes)

## Funcionalidades

- **Colar link** — analisa anime/filme e lista temporadas/episódios
- **Catálogo** — navegue, busque e use **Analisar** direto no card
- **Biblioteca** — downloads agrupados por anime, pausar/retomar, busca
- **Player** — assistir localmente com avanço ao próximo episódio
- **Atualizações** — verifica novas versões via GitHub Releases (Configurações)
- **FFmpeg** — incluído no instalador ou baixado automaticamente

## Requisitos (desenvolvimento)

- Node.js 18+
- Rust ([rustup](https://rustup.rs))
- FFmpeg (ou deixe o app instalar)

```powershell
.\preparar.ps1 -Dev
```

## Build do instalador

```powershell
.\preparar.ps1 -Build
```

Instalador em `src-tauri/target/release/bundle/` — executável **Minha Princesa Animes.exe**.

## Atualizações automáticas

Guia completo (chaves, GitHub Secrets, publicar release):

→ **[docs/ATUALIZACOES.md](docs/ATUALIZACOES.md)**

Resumo:

1. Gere chaves: `npm run tauri signer generate -- --ci -w "$env:USERPROFILE\.tauri\minha-princesa-animes.key"`
2. No GitHub → Settings → Secrets → `TAURI_SIGNING_PRIVATE_KEY` = conteúdo da `.key`
3. Suba versão em `tauri.conf.json` + `package.json`, tag `v0.2.0`, push
4. Actions publica release draft → você publica no GitHub

## Estrutura

```
src/           React (UI)
src-tauri/     Rust (downloads, parser, SQLite)
docs/          Documentação (updates, etc.)
```

## Licença

Uso pessoal. Conteúdo de vídeo é de responsabilidade do usuário.
