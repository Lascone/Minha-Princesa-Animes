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
        <a href="https://sushianimes.com.br" target="_blank" rel="noreferrer">
          <Icon name="fa-link" /> sushianimes.com.br
        </a>
      </div>
    </aside>
  );
}
