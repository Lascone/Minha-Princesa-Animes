import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppSettings } from "../types";
import { Icon } from "./Icon";
import { UpdateSection } from "./UpdateSection";

export function SettingsPanel() {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [saved, setSaved] = useState(false);
  const [ffmpegOk, setFfmpegOk] = useState<boolean | null>(null);
  const [ffmpegSource, setFfmpegSource] = useState<string | null>(null);

  useEffect(() => {
    invoke<AppSettings>("get_settings").then(setSettings);
    invoke<{ available: boolean; source: string }>("get_ffmpeg_info").then((info) => {
      setFfmpegOk(info.available);
      setFfmpegSource(info.source);
    });
  }, []);

  const update = (patch: Partial<AppSettings>) => {
    setSettings((s) => (s ? { ...s, ...patch } : s));
    setSaved(false);
  };

  const save = async () => {
    if (!settings) return;
    await invoke("save_settings", { settings });
    const info = await invoke<{ available: boolean; source: string }>("get_ffmpeg_info");
    setFfmpegOk(info.available);
    setFfmpegSource(info.source);
    setSaved(true);
  };

  const pickFolder = async () => {
    const folder = await invoke<string | null>("pick_download_folder");
    if (folder) update({ downloadFolder: folder });
  };

  if (!settings) return <div className="loading">Carregando...</div>;

  return (
    <div className="settings-panel">
      <h2>
        <Icon name="fa-gear" /> Configurações
      </h2>

      <div className="setting-group">
        <label>Pasta de downloads</label>
        <div className="setting-row">
          <input
            type="text"
            value={settings.downloadFolder}
            onChange={(e) => update({ downloadFolder: e.target.value })}
          />
          <button type="button" className="btn-ghost" onClick={pickFolder}>
            Escolher
          </button>
        </div>
      </div>

      <div className="setting-group">
        <label>Template de nomenclatura</label>
        <input
          type="text"
          value={settings.namingTemplate}
          onChange={(e) => update({ namingTemplate: e.target.value })}
        />
        <small>
          Variáveis: {"{anime}"}, {"{season}"}, {"{episode}"}, {"{title}"}
        </small>
      </div>

      <div className="setting-group">
        <label>Downloads simultâneos</label>
        <input
          type="number"
          min={1}
          max={10}
          value={settings.maxConcurrent}
          onChange={(e) =>
            update({ maxConcurrent: Math.min(10, Math.max(1, Number(e.target.value))) })
          }
        />
        <small>Quantos episódios podem baixar ao mesmo tempo (1 a 10).</small>
      </div>

      <div className="setting-group">
        <label>FFmpeg</label>
        <input
          type="text"
          value={settings.ffmpegPath}
          onChange={(e) => update({ ffmpegPath: e.target.value })}
          placeholder="Automático (deixe vazio)"
        />
        <small>
          Deixe vazio para o app usar o FFmpeg incluído ou instalar automaticamente.
        </small>
        {ffmpegOk === false && (
          <small className="error-text">
            FFmpeg não encontrado. O app tentará baixar na primeira execução ou ao baixar um episódio HLS.
          </small>
        )}
        {ffmpegOk === true && (
          <small className="success-text">
            FFmpeg OK{ffmpegSource ? ` (${ffmpegSource})` : ""}
          </small>
        )}
      </div>

      <div className="setting-group checkbox-group">
        <label>
          <input
            type="checkbox"
            checked={settings.overwrite}
            onChange={(e) => update({ overwrite: e.target.checked })}
          />
          Sobrescrever arquivos existentes
        </label>
      </div>

      <UpdateSection />

      <button type="button" className="btn-primary" onClick={save}>
        Salvar configurações
      </button>
      {saved && <span className="saved-msg">Salvo!</span>}
    </div>
  );
}
