import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import type { Page } from "../types";
import { AppLogo } from "./AppLogo";
import { Icon } from "./Icon";

const NAV: { id: Page; label: string; icon: string }[] = [
  { id: "home", label: "Início", icon: "fa-house" },
  { id: "catalog", label: "Catálogo", icon: "fa-clapperboard" },
  { id: "downloads", label: "Biblioteca", icon: "fa-download" },
  { id: "settings", label: "Configurações", icon: "fa-gear" },
];

interface SidebarProps {
  current: Page;
  onNavigate: (page: Page) => void;
  activeDownloads: number;
}

export function Sidebar({ current, onNavigate, activeDownloads }: SidebarProps) {
  const [appVersion, setAppVersion] = useState("");

  useEffect(() => {
    getVersion()
      .then((v) => setAppVersion(v))
      .catch(() => setAppVersion(""));
  }, []);

  return (
    <aside className="sidebar">
      <div className="sidebar-brand">
        <AppLogo size={56} className="brand-logo" />
        <p className="brand-tagline">Minha Princesa Animes</p>
      </div>
      <nav className="sidebar-nav">
        {NAV.map((item) => (
          <button
            key={item.id}
            className={`nav-item ${current === item.id ? "active" : ""}`}
            onClick={() => onNavigate(item.id)}
          >
            <span className="nav-icon">
              <Icon name={item.icon} />
            </span>
            <span>{item.label}</span>
            {item.id === "downloads" && activeDownloads > 0 && (
              <span className="badge">{activeDownloads}</span>
            )}
          </button>
        ))}
      </nav>
      <div className="sidebar-footer">
        {appVersion && (
          <p className="sidebar-version">
            <Icon name="fa-heart" /> v{appVersion}
          </p>
        )}
        <p className="sidebar-sources">
          <Icon name="fa-database" /> 5 fontes disponíveis
        </p>
        <a href="https://sushianimes.com.br" target="_blank" rel="noreferrer">
          <Icon name="fa-link" /> sushianimes.com.br
        </a>
        <a href="https://goyabu.io" target="_blank" rel="noreferrer">
          <Icon name="fa-link" /> goyabu.io
        </a>
        <a href="https://meusanimes.blog" target="_blank" rel="noreferrer">
          <Icon name="fa-link" /> meusanimes.blog
        </a>
        <a href="https://animesonlinecc.to" target="_blank" rel="noreferrer">
          <Icon name="fa-link" /> animesonlinecc.to
        </a>
        <a href="https://animesdigital.org" target="_blank" rel="noreferrer">
          <Icon name="fa-link" /> animesdigital.org
        </a>
      </div>
    </aside>
  );
}
