import { useEffect, useState } from "react";
import { isPermissionGranted, requestPermission } from "@tauri-apps/plugin-notification";
import { Sidebar } from "./components/Sidebar";
import { PasteLinkPanel } from "./components/PasteLinkPanel";
import { CatalogGrid } from "./components/CatalogGrid";
import { DownloadQueue } from "./components/DownloadQueue";
import { DownloadNotifications } from "./components/DownloadNotifications";
import { SettingsPanel } from "./components/SettingsPanel";import { UpdateBanner } from "./components/UpdateBanner";
import { TrayCloseModal } from "./components/TrayCloseModal";
import { useDownloads } from "./hooks/useDownloads";
import type { Page } from "./types";
import "./styles/app.css";
import "./styles/player-theme.css";

function App() {
  const [page, setPage] = useState<Page>("home");
  const [catalogUrl, setCatalogUrl] = useState("");
  const {
    downloads,
    cancel,
    retry,
    pause,
    resume,
    pauseAnime,
    resumeAnime,
    cancelAnime,
    remove,
  } = useDownloads();

  const activeCount = downloads.filter(
    (d) => d.status === "downloading" || d.status === "queued"
  ).length;

  const handleCatalogSelect = (url: string) => {
    setCatalogUrl(url);
    setPage("home");
  };

  useEffect(() => {
    isPermissionGranted().then((granted) => {
      if (!granted) {
        requestPermission();
      }
    });
  }, []);

  return (
    <div className="app-shell">
      <TrayCloseModal />
      <DownloadNotifications />      <Sidebar
        current={page}
        onNavigate={setPage}
        activeDownloads={activeCount}
      />
      <main className="main-content">
        <UpdateBanner onOpenSettings={() => setPage("settings")} />
        <div key={page} className="page-view">
          {page === "home" && (
            <PasteLinkPanel
              key={catalogUrl}
              initialUrl={catalogUrl}
              onDownloadStarted={() => setPage("downloads")}
            />
          )}
          {page === "catalog" && (
            <CatalogGrid
              onSelectAnime={handleCatalogSelect}
              onDownloadStarted={() => setPage("downloads")}
            />
          )}
          {page === "downloads" && (
            <DownloadQueue
              downloads={downloads}
              onCancel={cancel}
              onRetry={retry}
              onPause={pause}
              onResume={resume}
              onPauseAnime={pauseAnime}
              onResumeAnime={resumeAnime}
              onCancelAnime={cancelAnime}
              onRemove={remove}
            />
          )}
          {page === "settings" && <SettingsPanel />}
        </div>
      </main>
    </div>
  );
}

export default App;
