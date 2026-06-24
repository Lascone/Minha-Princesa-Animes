# Atualizações automáticas (GitHub Releases)

O **Minha Princesa Animes** usa o [updater oficial do Tauri 2](https://v2.tauri.app/plugin/updater/) com releases no GitHub. Cada build de produção é assinada digitalmente; o app só instala pacotes com assinatura válida.

## Como funciona para o usuário

1. Abra **Configurações → Atualizações**
2. Clique em **Verificar atualizações**
3. Se houver versão nova, clique em **Instalar** — o app baixa, instala e reinicia

O app consulta:

`https://github.com/Lascone/Minha-Princesa-Animes/releases/latest/download/latest.json`

> Builds de desenvolvimento (`npm run tauri dev`) **não** recebem auto-update.

---

## 1. Chaves de assinatura (uma vez)

O Tauri exige par de chaves **minisign** para assinar instaladores.

### Gerar chaves (Windows PowerShell)

```powershell
cd "caminho\do\projeto"
npm run tauri signer generate -- --ci -w "$env:USERPROFILE\.tauri\minha-princesa-animes.key"
```

Isso cria:

| Arquivo | Uso |
|---------|-----|
| `%USERPROFILE%\.tauri\minha-princesa-animes.key` | **Privada** — NUNCA commitar |
| `%USERPROFILE%\.tauri\minha-princesa-animes.key.pub` | **Pública** — já está em `src-tauri/tauri.conf.json` |

**Guarde a chave privada em local seguro.** Se perder, usuários com o app instalado não poderão receber updates assinados.

### Chave pública no projeto

O conteúdo de `.key.pub` fica em `tauri.conf.json`:

```json
"plugins": {
  "updater": {
    "pubkey": "dW50cnVzdGVkIGNvbW1lbnQ6...",
    "endpoints": [
      "https://github.com/Lascone/Minha-Princesa-Animes/releases/latest/download/latest.json"
    ]
  }
}
```

---

## 2. GitHub — o que configurar

Repositório: **https://github.com/Lascone/Minha-Princesa-Animes**

### A) Secret da chave privada (obrigatório para CI)

1. GitHub → repositório → **Settings**
2. **Secrets and variables** → **Actions**
3. **New repository secret**

| Nome | Valor |
|------|--------|
| `TAURI_SIGNING_PRIVATE_KEY` | Conteúdo **inteiro** do arquivo `minha-princesa-animes.key` (copie e cole o texto) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Deixe vazio ou omita se a chave não tiver senha |

> **Não** use Personal Access Token para assinar updates. O PAT é só para o GitHub Actions publicar releases.

### B) Personal Access Token (PAT) — quando precisa?

O workflow usa `GITHUB_TOKEN` automático do Actions para criar releases. **Na maioria dos casos você não precisa criar PAT manualmente.**

Crie um PAT **somente se**:

- o workflow falhar por permissão ao publicar release, ou
- você quiser publicar de outra máquina via API

#### Como criar PAT (Fine-grained ou Classic)

1. GitHub → foto de perfil → **Settings**
2. **Developer settings** → **Personal access tokens**
3. **Generate new token (classic)** recomendado para simplicidade
4. Marque escopo: **`repo`** (acesso completo a repositórios privados/públicos)
5. Gere e **copie o token** (só aparece uma vez)

Se usar em secret customizado:

| Nome | Valor |
|------|--------|
| `GH_PAT` | `ghp_xxxxxxxx...` |

E no workflow troque `GITHUB_TOKEN` por `${{ secrets.GH_PAT }}` no `tauri-action`.

---

## 3. Publicar uma nova versão

### Passo a passo

1. Atualize a versão em **dois lugares** (devem ser iguais):
   - `src-tauri/tauri.conf.json` → `"version": "0.2.0"`
   - `package.json` → `"version": "0.2.0"`

2. Commit e tag:

```powershell
git add .
git commit -m "release: v0.2.0"
git tag v0.2.0
git push origin main
git push origin v0.2.0
```

3. O workflow **Release** (`.github/workflows/release.yml`) roda automaticamente e:
   - compila o app Windows
   - assina os instaladores (`.exe`, `.msi`)
   - gera `latest.json` + arquivos `.sig`
   - cria um **Release draft** no GitHub

4. No GitHub → **Releases** → abra o draft → revise os assets → **Publish release**

### Build local assinado (opcional)

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content "$env:USERPROFILE\.tauri\minha-princesa-animes.key" -Raw
npm run tauri build
```

Artefatos em `src-tauri/target/release/bundle/` incluindo `.sig` para updater.

---

## 4. Primeiro push do repositório

Se ainda não subiu o código:

```powershell
cd "caminho\do\projeto"
git init
git add .
git commit -m "first commit: Minha Princesa Animes"
git branch -M main
git remote add origin https://github.com/Lascone/Minha-Princesa-Animes.git
git push -u origin main
```

Depois configure o secret `TAURI_SIGNING_PRIVATE_KEY` e faça o primeiro release com tag `v0.1.0`.

---

## 5. Solução de problemas

| Problema | Causa provável |
|----------|----------------|
| "Update not found" | Ainda não existe release publicado com `latest.json` |
| Erro de assinatura | `pubkey` no app ≠ par da chave que assinou o build |
| CI falha no signing | Secret `TAURI_SIGNING_PRIVATE_KEY` ausente ou conteúdo errado |
| Dev build não atualiza | Normal — updater só funciona em instalador de produção assinado |

---

## Referências

- [Tauri Updater](https://v2.tauri.app/plugin/updater/)
- [Tauri + GitHub Actions](https://v2.tauri.app/distribute/pipelines/github/)
