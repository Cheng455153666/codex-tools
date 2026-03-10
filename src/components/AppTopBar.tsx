import type { MouseEvent } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useI18n } from "../i18n/I18nProvider";

type AppTopBarProps = {
  onRefresh: () => void;
  refreshing: boolean;
  onGoHome: () => void;
  showRefresh: boolean;
};

function RefreshIcon({ spinning }: { spinning: boolean }) {
  return (
    <svg
      className={`iconGlyph ${spinning ? "isSpinning" : ""}`}
      viewBox="0 0 24 24"
      aria-hidden="true"
      focusable="false"
    >
      <path d="M21 12a9 9 0 1 1-2.64-6.36" />
      <path d="M21 3v6h-6" />
    </svg>
  );
}

export function AppTopBar({
  onRefresh,
  refreshing,
  onGoHome,
  showRefresh,
}: AppTopBarProps) {
  const { copy } = useI18n();
  const appWindow = getCurrentWindow();

  const handleDragMouseDown = (event: MouseEvent<HTMLElement>) => {
    if (event.button !== 0) {
      return;
    }
    const target = event.target as HTMLElement | null;
    if (
      target?.closest(
        "button, a, input, textarea, select, label, [role='button'], .topActions, .homeLink",
      )
    ) {
      return;
    }
    void appWindow.startDragging().catch(() => {});
  };

  return (
    <header className="topbar" onMouseDown={handleDragMouseDown}>
      <div className="topDragRegion" data-tauri-drag-region>
        <button type="button" className="brandLine homeLink" onClick={onGoHome}>
          <img className="appLogo" src="/codex-tools.png" alt={copy.topBar.logoAlt} />
          <h1>{copy.topBar.appTitle}</h1>
        </button>
      </div>
      <div className="topActions">
        {showRefresh ? (
          <button
            className="iconButton primary"
            onClick={onRefresh}
            disabled={refreshing}
            title={refreshing ? copy.topBar.refreshing : copy.topBar.manualRefresh}
            aria-label={refreshing ? copy.topBar.refreshing : copy.topBar.manualRefresh}
          >
            <RefreshIcon spinning={refreshing} />
          </button>
        ) : null}
      </div>
    </header>
  );
}
